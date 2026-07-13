use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

#[cfg(feature = "test-support")]
use std::{ffi::OsString, path::PathBuf};

use serde_json::Value;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout},
    sync::{broadcast, mpsc, oneshot, watch, Mutex, Notify},
    time::timeout,
};

use crate::{
    application::AppServerSource,
    domain::AuthUiState,
    providers::{
        app_server_protocol::{
            build_initialized_notification, build_method_not_found_response, build_request,
            encode_jsonl, parse_jsonl_line, ClientRequest, InboundMessage, NotificationKind,
            ProtocolError,
        },
        parse_account_read_result, parse_rate_limits_result,
    },
};

use super::app_server_process::{
    locate_candidate, read_limited_line, spawn_child, stop_owned_child, Candidate, ProbeError,
    ProbeOutcome, ProbeTimeouts, STDERR_DRAIN_LIMIT,
};

const WRITER_QUEUE_CAPACITY: usize = 64;
const NOTIFICATION_QUEUE_CAPACITY: usize = 32;
const MAX_PENDING_REQUESTS: usize = 32;

type PendingSender = oneshot::Sender<Result<Value, PendingFailure>>;

#[derive(Clone)]
pub struct AppServerSession {
    inner: Arc<SessionInner>,
    source: AppServerSource,
    request_timeout: Duration,
}

struct SessionInner {
    state: Mutex<SessionState>,
    writer_tx: mpsc::Sender<Value>,
    notification_tx: broadcast::Sender<NotificationKind>,
    shutdown_tx: watch::Sender<bool>,
    next_request_id: AtomicU64,
    closed: AtomicBool,
    process_stopped: AtomicBool,
    process_stopped_notify: Notify,
    shutdown_grace: Duration,
}

struct SessionState {
    pending: HashMap<u64, PendingSender>,
    failure: Option<SessionFailure>,
}

#[derive(Clone)]
enum SessionFailure {
    Exited,
    Protocol(ProtocolError),
}

enum PendingFailure {
    Remote,
    Session(SessionFailure),
}

impl SessionFailure {
    fn to_probe_error(&self) -> ProbeError {
        match self {
            Self::Exited => ProbeError::Exited,
            Self::Protocol(error) => ProbeError::Protocol(error.clone()),
        }
    }
}

impl SessionInner {
    async fn register_pending(&self, id: u64, sender: PendingSender) -> Result<(), ProbeError> {
        let mut state = self.state.lock().await;
        if let Some(failure) = &state.failure {
            return Err(failure.to_probe_error());
        }
        if state.pending.len() >= MAX_PENDING_REQUESTS {
            return Err(ProbeError::SourceBusy);
        }
        state.pending.insert(id, sender);
        Ok(())
    }

    async fn remove_pending(&self, id: u64) {
        self.state.lock().await.pending.remove(&id);
    }

    async fn route_response(&self, id: u64, outcome: Result<Value, ()>) {
        let sender = self.state.lock().await.pending.remove(&id);
        if let Some(sender) = sender {
            let result = outcome.map_err(|()| PendingFailure::Remote);
            let _ = sender.send(result);
        }
    }

    async fn fail(&self, failure: SessionFailure) {
        let pending = {
            let mut state = self.state.lock().await;
            if state.failure.is_some() {
                return;
            }
            state.failure = Some(failure.clone());
            state
                .pending
                .drain()
                .map(|(_, sender)| sender)
                .collect::<Vec<_>>()
        };

        self.closed.store(true, Ordering::Release);
        for sender in pending {
            let _ = sender.send(Err(PendingFailure::Session(failure.clone())));
        }
        let _ = self.shutdown_tx.send(true);
    }

    fn mark_process_stopped(&self) {
        self.process_stopped.store(true, Ordering::Release);
        self.process_stopped_notify.notify_waiters();
    }
}

impl AppServerSession {
    pub async fn connect() -> Result<Self, ProbeError> {
        let candidate = locate_candidate()?;
        Self::connect_candidate(candidate, ProbeTimeouts::default()).await
    }

    async fn connect_candidate(
        candidate: Candidate,
        timeouts: ProbeTimeouts,
    ) -> Result<Self, ProbeError> {
        let source = candidate.source;
        let mut child = spawn_child(&candidate)?;
        let stdin = match child.stdin.take() {
            Some(stdin) => stdin,
            None => {
                stop_owned_child(&mut child, timeouts.shutdown_grace).await;
                return Err(ProbeError::SpawnFailed);
            }
        };
        let stdout = match child.stdout.take() {
            Some(stdout) => stdout,
            None => {
                drop(stdin);
                stop_owned_child(&mut child, timeouts.shutdown_grace).await;
                return Err(ProbeError::SpawnFailed);
            }
        };

        if let Some(mut stderr) = child.stderr.take() {
            tokio::spawn(async move {
                let mut buffer = [0_u8; 1024];
                let mut drained = 0_usize;
                while drained < STDERR_DRAIN_LIMIT {
                    let remaining = STDERR_DRAIN_LIMIT - drained;
                    let read_size = remaining.min(buffer.len());
                    match stderr.read(&mut buffer[..read_size]).await {
                        Ok(0) | Err(_) => break,
                        Ok(read) => drained += read,
                    }
                }
            });
        }

        let (writer_tx, writer_rx) = mpsc::channel(WRITER_QUEUE_CAPACITY);
        let (notification_tx, _) = broadcast::channel(NOTIFICATION_QUEUE_CAPACITY);
        let (shutdown_tx, _) = watch::channel(false);
        let inner = Arc::new(SessionInner {
            state: Mutex::new(SessionState {
                pending: HashMap::new(),
                failure: None,
            }),
            writer_tx,
            notification_tx,
            shutdown_tx,
            next_request_id: AtomicU64::new(1),
            closed: AtomicBool::new(false),
            process_stopped: AtomicBool::new(false),
            process_stopped_notify: Notify::new(),
            shutdown_grace: timeouts.shutdown_grace,
        });

        spawn_writer_task(
            stdin,
            writer_rx,
            inner.clone(),
            inner.shutdown_tx.subscribe(),
        );
        spawn_reader_task(stdout, inner.clone(), inner.shutdown_tx.subscribe());
        spawn_supervisor_task(child, inner.clone(), inner.shutdown_tx.subscribe());

        let session = Self {
            inner,
            source,
            request_timeout: timeouts.request,
        };

        let initialized = session
            .request(ClientRequest::Initialize {
                application_version: env!("CARGO_PKG_VERSION"),
            })
            .await;
        if let Err(error) = initialized {
            session.shutdown().await;
            return Err(error);
        }
        if let Err(error) = session.send_value(build_initialized_notification()).await {
            session.shutdown().await;
            return Err(error);
        }

        Ok(session)
    }

    pub async fn read_account_and_quota(&self) -> Result<ProbeOutcome, ProbeError> {
        let account_value = self.request(ClientRequest::AccountRead).await?;
        let auth = parse_account_read_result(&account_value)?;
        let quota = if auth.state == AuthUiState::Authenticated {
            let quota_value = self.request(ClientRequest::RateLimitsRead).await?;
            Some(parse_rate_limits_result(&quota_value)?)
        } else {
            None
        };

        Ok(ProbeOutcome {
            source: self.source,
            auth,
            quota,
        })
    }

    pub fn subscribe_notifications(&self) -> broadcast::Receiver<NotificationKind> {
        self.inner.notification_tx.subscribe()
    }

    pub fn is_closed(&self) -> bool {
        self.inner.closed.load(Ordering::Acquire)
    }

    pub async fn shutdown(&self) {
        let stopped = self.inner.process_stopped_notify.notified();
        self.inner.fail(SessionFailure::Exited).await;
        if !self.inner.process_stopped.load(Ordering::Acquire) {
            let wait = self.inner.shutdown_grace.saturating_mul(2);
            let _ = timeout(wait, stopped).await;
        }
    }

    async fn request(&self, request: ClientRequest<'_>) -> Result<Value, ProbeError> {
        let id = self.next_request_id()?;
        let value = build_request(request, id)?;
        let (sender, receiver) = oneshot::channel();
        self.inner.register_pending(id, sender).await?;

        if self.inner.writer_tx.send(value).await.is_err() {
            self.inner.remove_pending(id).await;
            self.inner.fail(SessionFailure::Exited).await;
            return Err(ProbeError::Exited);
        }

        match timeout(self.request_timeout, receiver).await {
            Ok(Ok(Ok(value))) => Ok(value),
            Ok(Ok(Err(PendingFailure::Remote))) => Err(ProbeError::Remote),
            Ok(Ok(Err(PendingFailure::Session(failure)))) => Err(failure.to_probe_error()),
            Ok(Err(_)) => Err(ProbeError::Exited),
            Err(_) => {
                self.inner.remove_pending(id).await;
                Err(ProbeError::RequestTimeout)
            }
        }
    }

    async fn send_value(&self, value: Value) -> Result<(), ProbeError> {
        if self.inner.writer_tx.send(value).await.is_err() {
            self.inner.fail(SessionFailure::Exited).await;
            return Err(ProbeError::Exited);
        }

        Ok(())
    }

    fn next_request_id(&self) -> Result<u64, ProbeError> {
        self.inner
            .next_request_id
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                current.checked_add(1)
            })
            .map_err(|_| ProbeError::RequestIdExhausted)
    }

    #[cfg(feature = "test-support")]
    #[doc(hidden)]
    pub async fn connect_for_test(
        program: PathBuf,
        scenario: &str,
        request_timeout: Duration,
    ) -> Result<Self, ProbeError> {
        let candidate = Candidate {
            program,
            source: AppServerSource::External,
            arguments: vec![OsString::from("--scenario"), OsString::from(scenario)],
        };
        Self::connect_candidate(
            candidate,
            ProbeTimeouts {
                request: request_timeout,
                shutdown_grace: Duration::from_millis(100),
            },
        )
        .await
    }
}

fn spawn_writer_task(
    mut stdin: ChildStdin,
    mut receiver: mpsc::Receiver<Value>,
    inner: Arc<SessionInner>,
    mut shutdown: watch::Receiver<bool>,
) {
    tokio::spawn(async move {
        loop {
            let value = tokio::select! {
                changed = shutdown.changed() => {
                    let _ = changed;
                    break;
                }
                value = receiver.recv() => match value {
                    Some(value) => value,
                    None => break,
                }
            };

            let bytes = match encode_jsonl(&value) {
                Ok(bytes) => bytes,
                Err(error) => {
                    inner.fail(SessionFailure::Protocol(error)).await;
                    break;
                }
            };
            if stdin.write_all(&bytes).await.is_err() || stdin.flush().await.is_err() {
                inner.fail(SessionFailure::Exited).await;
                break;
            }
        }
    });
}

fn spawn_reader_task(
    stdout: ChildStdout,
    inner: Arc<SessionInner>,
    mut shutdown: watch::Receiver<bool>,
) {
    tokio::spawn(async move {
        let mut reader = BufReader::new(stdout);
        loop {
            let line = tokio::select! {
                changed = shutdown.changed() => {
                    let _ = changed;
                    break;
                }
                line = read_limited_line(&mut reader) => line,
            };
            let line = match line {
                Ok(line) => line,
                Err(ProbeError::Protocol(error)) => {
                    inner.fail(SessionFailure::Protocol(error)).await;
                    break;
                }
                Err(_) => {
                    inner.fail(SessionFailure::Exited).await;
                    break;
                }
            };
            let message = match parse_jsonl_line(&line) {
                Ok(Some(message)) => message,
                Ok(None) => continue,
                Err(error) => {
                    inner.fail(SessionFailure::Protocol(error)).await;
                    break;
                }
            };

            match message {
                InboundMessage::Response { id, outcome } => {
                    inner.route_response(id, outcome.map_err(|_| ())).await;
                }
                InboundMessage::Notification(kind) => {
                    let _ = inner.notification_tx.send(kind);
                }
                InboundMessage::UnsupportedServerRequest { id } => {
                    if inner
                        .writer_tx
                        .send(build_method_not_found_response(id))
                        .await
                        .is_err()
                    {
                        inner.fail(SessionFailure::Exited).await;
                        break;
                    }
                }
            }
        }
    });
}

fn spawn_supervisor_task(
    mut child: Child,
    inner: Arc<SessionInner>,
    mut shutdown: watch::Receiver<bool>,
) {
    tokio::spawn(async move {
        tokio::select! {
            _ = child.wait() => {
                inner.fail(SessionFailure::Exited).await;
            }
            _ = shutdown.changed() => {
                stop_owned_child(&mut child, inner.shutdown_grace).await;
            }
        }
        inner.mark_process_stopped();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn pending_map_rejects_requests_after_reaching_its_bound() {
        let (writer_tx, _writer_rx) = mpsc::channel(WRITER_QUEUE_CAPACITY);
        let (notification_tx, _) = broadcast::channel(NOTIFICATION_QUEUE_CAPACITY);
        let (shutdown_tx, _) = watch::channel(false);
        let inner = SessionInner {
            state: Mutex::new(SessionState {
                pending: HashMap::new(),
                failure: None,
            }),
            writer_tx,
            notification_tx,
            shutdown_tx,
            next_request_id: AtomicU64::new(1),
            closed: AtomicBool::new(false),
            process_stopped: AtomicBool::new(false),
            process_stopped_notify: Notify::new(),
            shutdown_grace: Duration::from_millis(100),
        };

        for id in 1..=MAX_PENDING_REQUESTS as u64 {
            let (sender, _receiver) = oneshot::channel();
            assert!(inner.register_pending(id, sender).await.is_ok());
        }

        let (sender, _receiver) = oneshot::channel();
        let error = inner
            .register_pending(MAX_PENDING_REQUESTS as u64 + 1, sender)
            .await;
        assert!(matches!(error, Err(ProbeError::SourceBusy)));
    }
}
