use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use tokio::sync::broadcast;

use crate::{
    domain::{
        AuthUiState, ErrorCode, ProviderKind, QuotaError, QuotaSnapshot, QuotaSource, QuotaStatus,
    },
    infrastructure::{AppServerSession, ProbeError, ProbeOutcome},
    providers::app_server_protocol::NotificationKind,
};

use super::{
    AppServerPhase, AppServerStatus, AppState, IpcError, RefreshPhase, RefreshReason,
    RefreshReceipt, RefreshState,
};

const MANUAL_REFRESH_COOLDOWN: Duration = Duration::from_secs(30);
const NOTIFICATION_DEBOUNCE: Duration = Duration::from_millis(150);
const JOINED_REFRESH_RETRY: Duration = Duration::from_millis(100);
const SCHEDULER_TICK: Duration = Duration::from_secs(5);

enum RetryStatusUpdate {
    Preserve,
    Reset,
    BackingOff {
        attempt: u32,
        next_retry_at: Option<String>,
    },
}

pub fn spawn_startup_refresh(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let _ = refresh_quota_runtime(&app, RefreshReason::Startup, false).await;
    });
}

pub fn spawn_refresh_scheduler(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(SCHEDULER_TICK);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await;

        loop {
            interval.tick().await;
            let state = app.state::<AppState>();
            let visible = state
                .window_state
                .read()
                .map(|window| window.visible)
                .unwrap_or(false);
            let reason = state
                .refresh_policy
                .lock()
                .ok()
                .and_then(|policy| policy.scheduled_reason(Instant::now(), visible));
            if let Some(reason) = reason {
                let _ = refresh_quota_runtime(&app, reason, false).await;
            }
        }
    });
}

pub async fn shutdown_app_server(app: &AppHandle) {
    let state = app.state::<AppState>();
    let session = state.app_server_session.lock().await.take();
    if let Some(session) = session {
        let _ = update_app_server_phase(
            app,
            &state,
            AppServerPhase::ShuttingDown,
            None,
            None,
            RetryStatusUpdate::Preserve,
        );
        session.shutdown().await;
        let _ = update_app_server_phase(
            app,
            &state,
            AppServerPhase::Stopped,
            None,
            None,
            RetryStatusUpdate::Reset,
        );
    }
}

pub async fn refresh_quota_runtime(
    app: &AppHandle,
    reason: RefreshReason,
    enforce_manual_cooldown: bool,
) -> Result<RefreshReceipt, IpcError> {
    let state = app.state::<AppState>();

    let manual_started = if enforce_manual_cooldown {
        let now = Instant::now();
        let mut last_refresh = state
            .last_manual_refresh
            .lock()
            .map_err(|_| state_unavailable_error())?;
        if let Some(previous) = *last_refresh {
            let elapsed = previous.elapsed();
            if elapsed < MANUAL_REFRESH_COOLDOWN {
                let remaining = MANUAL_REFRESH_COOLDOWN.saturating_sub(elapsed);
                return Err(IpcError::new(
                    ErrorCode::RefreshCooldown,
                    "error.refreshCooldown",
                    true,
                    Some(remaining.as_millis().try_into().unwrap_or(u64::MAX)),
                ));
            }
        }
        *last_refresh = Some(now);
        Some(now)
    } else {
        None
    };
    let initial_manual_deadline =
        manual_started.and_then(|_| now_rfc3339_after(MANUAL_REFRESH_COOLDOWN));

    let Ok(_guard) = state.refresh_guard.try_lock() else {
        let current = read_refresh_state(&state)?;
        return Ok(RefreshReceipt {
            accepted: true,
            joined_existing_request: true,
            request_revision: current.revision,
            state: current,
        });
    };

    let attempt_started = Instant::now();
    let should_skip = state
        .refresh_policy
        .lock()
        .map_err(|_| state_unavailable_error())?
        .should_skip(reason, attempt_started);
    if should_skip {
        let current = read_refresh_state(&state)?;
        return Ok(RefreshReceipt {
            accepted: false,
            joined_existing_request: false,
            request_revision: current.revision,
            state: current,
        });
    }
    state
        .refresh_policy
        .lock()
        .map_err(|_| state_unavailable_error())?
        .begin_attempt(attempt_started);

    let request_revision = state.next_revision();
    let refreshing = RefreshState {
        revision: request_revision,
        phase: RefreshPhase::Refreshing,
        reason: Some(reason),
        started_at: now_rfc3339(),
        next_allowed_manual_refresh_at: initial_manual_deadline,
        next_retry_at: None,
    };
    write_refresh_state(&state, refreshing.clone())?;
    emit_to_widget(app, "quota://refresh-state-changed", &refreshing);

    let next_retry_at = match run_session_probe(app, &state).await {
        Ok(outcome) => {
            state
                .refresh_policy
                .lock()
                .map_err(|_| state_unavailable_error())?
                .record_success(Instant::now());
            apply_probe_success(app, &state, outcome)?;
            None
        }
        Err(error) => {
            let retryable = error.to_quota_error().retryable;
            let (retry_delay, retry_attempt) = {
                let mut policy = state
                    .refresh_policy
                    .lock()
                    .map_err(|_| state_unavailable_error())?;
                let delay = policy.record_failure(Instant::now(), retryable);
                (delay, policy.retry_attempt())
            };
            let next_retry_at = retry_delay.and_then(now_rfc3339_after);
            apply_probe_failure(app, &state, error, next_retry_at.clone(), retry_attempt)?;
            next_retry_at
        }
    };

    let manual_remaining = manual_started
        .map(|started| MANUAL_REFRESH_COOLDOWN.saturating_sub(started.elapsed()))
        .filter(|remaining| !remaining.is_zero());
    let next_allowed_manual_refresh_at = manual_remaining.and_then(now_rfc3339_after);

    let completed_revision = state.next_revision();
    let completed = RefreshState {
        revision: completed_revision,
        phase: if next_retry_at.is_some() {
            RefreshPhase::BackingOff
        } else if next_allowed_manual_refresh_at.is_some() {
            RefreshPhase::Cooldown
        } else {
            RefreshPhase::Idle
        },
        reason: None,
        started_at: None,
        next_allowed_manual_refresh_at,
        next_retry_at,
    };
    write_refresh_state(&state, completed.clone())?;
    emit_to_widget(app, "quota://refresh-state-changed", &completed);
    if completed.phase == RefreshPhase::Cooldown {
        if let Some(remaining) = manual_remaining {
            spawn_cooldown_completion(app.clone(), completed_revision, remaining);
        }
    }

    Ok(RefreshReceipt {
        accepted: true,
        joined_existing_request: false,
        request_revision,
        state: completed,
    })
}

fn spawn_cooldown_completion(app: AppHandle, expected_revision: u64, remaining: Duration) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(remaining).await;
        let state = app.state::<AppState>();
        let payload = {
            let Ok(mut refresh) = state.refresh_state.write() else {
                return;
            };
            if refresh.revision != expected_revision || refresh.phase != RefreshPhase::Cooldown {
                return;
            }
            refresh.revision = state.next_revision();
            refresh.phase = RefreshPhase::Idle;
            refresh.next_allowed_manual_refresh_at = None;
            refresh.clone()
        };
        emit_to_widget(&app, "quota://refresh-state-changed", &payload);
    });
}

async fn run_session_probe(app: &AppHandle, state: &AppState) -> Result<ProbeOutcome, ProbeError> {
    let session = acquire_session(app, state).await?;
    let result = session.read_account_and_quota().await;

    if result.is_err() && should_discard_session(result.as_ref().err()) {
        let stale = state.app_server_session.lock().await.take();
        if let Some(stale) = stale {
            stale.shutdown().await;
        }
    }

    result
}

async fn acquire_session(
    app: &AppHandle,
    state: &AppState,
) -> Result<AppServerSession, ProbeError> {
    let (existing, stale) = {
        let mut slot = state.app_server_session.lock().await;
        match slot.as_ref() {
            Some(session) if !session.is_closed() => (Some(session.clone()), None),
            Some(_) => (None, slot.take()),
            None => (None, None),
        }
    };

    if let Some(existing) = existing {
        return Ok(existing);
    }
    if let Some(stale) = stale {
        stale.shutdown().await;
    }

    let _ = update_app_server_phase(
        app,
        state,
        AppServerPhase::Locating,
        None,
        None,
        RetryStatusUpdate::Preserve,
    );
    let _ = update_app_server_phase(
        app,
        state,
        AppServerPhase::Starting,
        None,
        None,
        RetryStatusUpdate::Preserve,
    );
    let _ = update_app_server_phase(
        app,
        state,
        AppServerPhase::Initializing,
        None,
        None,
        RetryStatusUpdate::Preserve,
    );
    let session = AppServerSession::connect().await?;
    let notifications = session.subscribe_notifications();
    state
        .app_server_session
        .lock()
        .await
        .replace(session.clone());
    spawn_notification_watcher(app.clone(), notifications);

    Ok(session)
}

fn should_discard_session(error: Option<&ProbeError>) -> bool {
    matches!(
        error,
        Some(ProbeError::Exited | ProbeError::Protocol(_) | ProbeError::RequestIdExhausted)
    )
}

fn spawn_notification_watcher(
    app: AppHandle,
    mut notifications: broadcast::Receiver<NotificationKind>,
) {
    tauri::async_runtime::spawn(async move {
        loop {
            let mut reason = match notifications.recv().await {
                Ok(NotificationKind::AccountChanged) => RefreshReason::AccountNotification,
                Ok(NotificationKind::RateLimitsChanged) => RefreshReason::QuotaNotification,
                Ok(NotificationKind::Unknown) => continue,
                Err(broadcast::error::RecvError::Lagged(_)) => RefreshReason::QuotaNotification,
                Err(broadcast::error::RecvError::Closed) => break,
            };

            tokio::time::sleep(NOTIFICATION_DEBOUNCE).await;
            while let Ok(notification) = notifications.try_recv() {
                if notification == NotificationKind::AccountChanged {
                    reason = RefreshReason::AccountNotification;
                }
            }

            loop {
                match refresh_quota_runtime(&app, reason, false).await {
                    Ok(receipt) if receipt.joined_existing_request => {
                        tokio::time::sleep(JOINED_REFRESH_RETRY).await;
                    }
                    _ => break,
                }
            }

            while notifications.try_recv().is_ok() {}
        }
    });
}

fn apply_probe_success(
    app: &AppHandle,
    state: &AppState,
    outcome: ProbeOutcome,
) -> Result<(), IpcError> {
    let revision = state.next_revision();
    let now = now_rfc3339();
    let mut snapshot = QuotaSnapshot::loading(revision);
    snapshot.source = Some(QuotaSource::AppServer);
    snapshot.provider = Some(ProviderKind::CodexAppServer);
    snapshot.auth = outcome.auth.clone();
    snapshot.fetched_at = now.clone();

    match outcome.auth.state {
        AuthUiState::SignedOut => {
            snapshot.status = QuotaStatus::SignedOut;
            snapshot.error = Some(QuotaError::new(
                ErrorCode::AuthRequired,
                "error.authRequired",
                false,
                None,
            ));
        }
        AuthUiState::ApiKeyMode => {
            snapshot.status = QuotaStatus::ApiKeyMode;
            snapshot.error = Some(QuotaError::new(
                ErrorCode::ApiKeyMode,
                "error.apiKeyMode",
                false,
                None,
            ));
        }
        AuthUiState::Authenticated => {
            if let Some(quota) = outcome.quota {
                snapshot.buckets = quota.buckets;
                snapshot.banked_resets = quota.banked_resets;
                if snapshot.buckets.is_empty() {
                    snapshot.status = QuotaStatus::ServiceUnavailable;
                    snapshot.error = Some(QuotaError::new(
                        ErrorCode::RateLimitsUnavailable,
                        "error.rateLimitsUnavailable",
                        true,
                        None,
                    ));
                } else {
                    let reached = snapshot
                        .buckets
                        .iter()
                        .any(|bucket| bucket.rate_limit_reached_type.is_some());
                    snapshot.status = if reached {
                        QuotaStatus::QuotaReached
                    } else {
                        QuotaStatus::Ok
                    };
                    snapshot.last_good_at = now;
                }
            } else {
                snapshot.status = QuotaStatus::ServiceUnavailable;
                snapshot.error = Some(QuotaError::new(
                    ErrorCode::RateLimitsUnavailable,
                    "error.rateLimitsUnavailable",
                    true,
                    None,
                ));
            }
        }
        AuthUiState::ExternalProvider | AuthUiState::Unknown => {
            snapshot.status = QuotaStatus::ServiceUnavailable;
            snapshot.error = Some(QuotaError::new(
                ErrorCode::RateLimitsUnavailable,
                "error.unsupportedAuth",
                false,
                None,
            ));
        }
    }

    let successful_quota = matches!(snapshot.status, QuotaStatus::Ok | QuotaStatus::QuotaReached);
    if successful_quota {
        write_last_good_snapshot(state, Some(snapshot.clone()))?;
    } else if outcome.auth.state != AuthUiState::Authenticated {
        write_last_good_snapshot(state, None)?;
    } else if let Some(last_good) = read_last_good_snapshot(state)? {
        let error = snapshot.error.clone();
        snapshot = last_good;
        snapshot.revision = revision;
        snapshot.status = QuotaStatus::Stale;
        snapshot.error = error;
    }

    write_snapshot(state, snapshot.clone())?;
    emit_to_widget(app, "quota://auth-state-changed", &snapshot.auth);
    emit_to_widget(app, "quota://snapshot-updated", &snapshot);
    update_app_server_phase(
        app,
        state,
        AppServerPhase::Ready,
        Some(outcome.source),
        None,
        RetryStatusUpdate::Reset,
    )
}

fn apply_probe_failure(
    app: &AppHandle,
    state: &AppState,
    error: ProbeError,
    next_retry_at: Option<String>,
    retry_attempt: u32,
) -> Result<(), IpcError> {
    let quota_error = error.to_quota_error();
    let revision = state.next_revision();
    let last_good = read_last_good_snapshot(state)?;
    let mut snapshot = last_good.unwrap_or_else(|| QuotaSnapshot::loading(revision));
    snapshot.revision = revision;
    snapshot.status = if snapshot.buckets.is_empty() {
        match quota_error.code {
            ErrorCode::ResponseIncompatible | ErrorCode::ProtocolMessageTooLarge => {
                QuotaStatus::Incompatible
            }
            ErrorCode::AppServerNotFound
            | ErrorCode::AppServerExecutionDenied
            | ErrorCode::AppServerExited => QuotaStatus::AppServerUnavailable,
            _ => QuotaStatus::ServiceUnavailable,
        }
    } else {
        QuotaStatus::Stale
    };
    snapshot.error = Some(quota_error.clone());
    snapshot.next_retry_at = next_retry_at.clone();

    write_snapshot(state, snapshot.clone())?;
    emit_to_widget(app, "quota://snapshot-updated", &snapshot);

    let phase = if quota_error.code == ErrorCode::ResponseIncompatible {
        AppServerPhase::Incompatible
    } else if next_retry_at.is_some() {
        AppServerPhase::BackingOff
    } else {
        AppServerPhase::Failed
    };
    let retry_status = if phase == AppServerPhase::BackingOff {
        RetryStatusUpdate::BackingOff {
            attempt: retry_attempt,
            next_retry_at,
        }
    } else {
        RetryStatusUpdate::Reset
    };
    update_app_server_phase(app, state, phase, None, Some(quota_error), retry_status)
}

fn update_app_server_phase(
    app: &AppHandle,
    state: &AppState,
    phase: AppServerPhase,
    source: Option<super::AppServerSource>,
    error: Option<QuotaError>,
    retry_status: RetryStatusUpdate,
) -> Result<(), IpcError> {
    let mut status = state
        .app_server_status
        .write()
        .map_err(|_| state_unavailable_error())?;
    status.revision = state.next_revision();
    status.phase = phase;
    if source.is_some() {
        status.source = source;
    }
    match retry_status {
        RetryStatusUpdate::Preserve => {}
        RetryStatusUpdate::Reset => {
            status.restart_attempt = 0;
            status.next_retry_at = None;
        }
        RetryStatusUpdate::BackingOff {
            attempt,
            next_retry_at,
        } => {
            status.restart_attempt = attempt;
            status.next_retry_at = next_retry_at;
        }
    }
    status.error = error;
    let payload: AppServerStatus = status.clone();
    drop(status);
    emit_to_widget(app, "quota://app-server-state-changed", &payload);
    Ok(())
}

fn now_rfc3339() -> Option<String> {
    OffsetDateTime::now_utc().format(&Rfc3339).ok()
}

fn now_rfc3339_after(delay: Duration) -> Option<String> {
    let seconds = i64::try_from(delay.as_secs()).ok()?;
    (OffsetDateTime::now_utc() + time::Duration::seconds(seconds))
        .format(&Rfc3339)
        .ok()
}

fn write_snapshot(state: &AppState, snapshot: QuotaSnapshot) -> Result<(), IpcError> {
    let mut current = state
        .snapshot
        .write()
        .map_err(|_| state_unavailable_error())?;
    *current = snapshot;
    Ok(())
}

fn read_last_good_snapshot(state: &AppState) -> Result<Option<QuotaSnapshot>, IpcError> {
    state
        .last_good_snapshot
        .read()
        .map(|snapshot| snapshot.clone())
        .map_err(|_| state_unavailable_error())
}

fn write_last_good_snapshot(
    state: &AppState,
    snapshot: Option<QuotaSnapshot>,
) -> Result<(), IpcError> {
    let mut current = state
        .last_good_snapshot
        .write()
        .map_err(|_| state_unavailable_error())?;
    *current = snapshot;
    Ok(())
}

fn read_refresh_state(state: &AppState) -> Result<RefreshState, IpcError> {
    state
        .refresh_state
        .read()
        .map(|refresh| refresh.clone())
        .map_err(|_| state_unavailable_error())
}

fn write_refresh_state(state: &AppState, refresh: RefreshState) -> Result<(), IpcError> {
    let mut current = state
        .refresh_state
        .write()
        .map_err(|_| state_unavailable_error())?;
    *current = refresh;
    Ok(())
}

fn emit_to_widget<T>(app: &AppHandle, event: &str, payload: &T)
where
    T: serde::Serialize + Clone,
{
    let _ = app.emit_to("widget", event, payload.clone());
}

fn state_unavailable_error() -> IpcError {
    IpcError::new(
        ErrorCode::ServiceUnavailable,
        "error.internalStateUnavailable",
        true,
        None,
    )
}
