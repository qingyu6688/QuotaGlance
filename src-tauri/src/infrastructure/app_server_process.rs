use std::{
    env,
    ffi::{OsStr, OsString},
    fs,
    io::ErrorKind,
    path::PathBuf,
    process::Stdio,
    time::Duration,
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(target_os = "macos")]
use std::path::Path;
#[cfg(target_os = "windows")]
use std::{path::Path, time::SystemTime};

use thiserror::Error;
use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt},
    process::{Child, Command},
    time::timeout,
};

#[cfg(feature = "test-support")]
use serde_json::Value;
#[cfg(feature = "test-support")]
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    process::{ChildStdin, ChildStdout},
};

use crate::{
    application::AppServerSource,
    domain::{AuthSummary, ErrorCode, QuotaError},
    providers::{
        app_server_protocol::{ProtocolError, MAX_MESSAGE_BYTES},
        ProviderError, ProviderQuotaData,
    },
};

#[cfg(feature = "test-support")]
use crate::{
    domain::AuthUiState,
    providers::{
        app_server_protocol::{
            build_initialized_notification, build_method_not_found_response, build_request,
            encode_jsonl, parse_jsonl_line, ClientRequest, InboundMessage,
        },
        parse_account_read_result, parse_rate_limits_result,
    },
};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const SHUTDOWN_GRACE: Duration = Duration::from_millis(500);
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
pub(super) const STDERR_DRAIN_LIMIT: usize = 64 * 1024;

#[derive(Debug)]
pub struct ProbeOutcome {
    pub source: AppServerSource,
    pub auth: AuthSummary,
    pub quota: Option<ProviderQuotaData>,
}

#[derive(Debug, Error)]
pub enum ProbeError {
    #[error("未找到可执行的 Codex App Server")]
    NotFound,
    #[error("系统拒绝执行 Codex App Server")]
    ExecutionDenied,
    #[error("Codex App Server 启动失败")]
    SpawnFailed,
    #[error("Codex App Server 握手或请求超时")]
    RequestTimeout,
    #[error("Codex App Server 在请求完成前退出")]
    Exited,
    #[error("Codex App Server 返回了受控远端错误")]
    Remote,
    #[error("Codex App Server 会话中的待处理请求已达到上限")]
    SourceBusy,
    #[error("Codex App Server 会话请求 ID 已耗尽")]
    RequestIdExhausted,
    #[error(transparent)]
    Protocol(#[from] ProtocolError),
    #[error(transparent)]
    Provider(#[from] ProviderError),
}

impl ProbeError {
    pub fn to_quota_error(&self) -> QuotaError {
        match self {
            Self::NotFound => QuotaError::new(
                ErrorCode::AppServerNotFound,
                "error.appServerNotFound",
                true,
                None,
            ),
            Self::ExecutionDenied => QuotaError::new(
                ErrorCode::AppServerExecutionDenied,
                "error.appServerExecutionDenied",
                true,
                None,
            ),
            Self::SpawnFailed | Self::Exited => QuotaError::new(
                ErrorCode::AppServerExited,
                "error.appServerExited",
                true,
                None,
            ),
            Self::RequestTimeout => QuotaError::new(
                ErrorCode::ProtocolRequestTimeout,
                "error.protocolRequestTimeout",
                true,
                None,
            ),
            Self::Remote => QuotaError::new(
                ErrorCode::ServiceUnavailable,
                "error.serviceUnavailable",
                true,
                None,
            ),
            Self::SourceBusy => {
                QuotaError::new(ErrorCode::SourceBusy, "error.sourceBusy", true, None)
            }
            Self::RequestIdExhausted => QuotaError::new(
                ErrorCode::ServiceUnavailable,
                "error.requestIdExhausted",
                false,
                None,
            ),
            Self::Protocol(error) => error.to_quota_error(),
            Self::Provider(error) => error.to_quota_error(),
        }
    }
}

pub(super) struct Candidate {
    pub program: PathBuf,
    pub source: AppServerSource,
    pub arguments: Vec<OsString>,
}

#[derive(Clone, Copy)]
pub(super) struct ProbeTimeouts {
    pub request: Duration,
    pub shutdown_grace: Duration,
}

impl Default for ProbeTimeouts {
    fn default() -> Self {
        Self {
            request: REQUEST_TIMEOUT,
            shutdown_grace: SHUTDOWN_GRACE,
        }
    }
}

#[cfg(feature = "test-support")]
async fn run_app_server_probe_with_candidate(
    candidate: Candidate,
    timeouts: ProbeTimeouts,
) -> Result<ProbeOutcome, ProbeError> {
    let mut child = spawn_child(&candidate)?;

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

    let mut stdin = child.stdin.take().ok_or(ProbeError::SpawnFailed)?;
    let stdout = child.stdout.take().ok_or(ProbeError::SpawnFailed)?;
    let mut reader = BufReader::new(stdout);

    let result = match timeout(
        timeouts.request.saturating_mul(4),
        run_protocol_sequence(&mut stdin, &mut reader, timeouts.request),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => Err(ProbeError::RequestTimeout),
    };

    drop(stdin);
    stop_owned_child(&mut child, timeouts.shutdown_grace).await;

    result.map(|(auth, quota)| ProbeOutcome {
        source: candidate.source,
        auth,
        quota,
    })
}

pub(super) fn locate_candidate() -> Result<Candidate, ProbeError> {
    #[cfg(debug_assertions)]
    if let Some(configured) = env::var_os("QUOTAGLANCE_CODEX_PATH") {
        let configured = PathBuf::from(configured);
        if !configured.is_absolute() {
            return Err(ProbeError::NotFound);
        }
        let program = canonical_executable(configured).ok_or(ProbeError::NotFound)?;
        return Ok(Candidate {
            program,
            source: AppServerSource::External,
            arguments: Vec::new(),
        });
    }

    #[cfg(target_os = "windows")]
    if let Some(program) = locate_managed_codex_desktop_runtime() {
        return Ok(Candidate {
            program,
            source: AppServerSource::External,
            arguments: Vec::new(),
        });
    }

    #[cfg(target_os = "macos")]
    if let Some(program) = locate_macos_desktop_runtime() {
        return Ok(Candidate {
            program,
            source: AppServerSource::External,
            arguments: Vec::new(),
        });
    }

    if let Some(program) = locate_codex_from_path() {
        return Ok(Candidate {
            program,
            source: if cfg!(debug_assertions) {
                AppServerSource::DevelopmentPath
            } else {
                AppServerSource::External
            },
            arguments: Vec::new(),
        });
    }

    #[cfg(unix)]
    if let Some(program) = locate_common_unix_codex_runtime() {
        return Ok(Candidate {
            program,
            source: AppServerSource::External,
            arguments: Vec::new(),
        });
    }

    #[cfg(debug_assertions)]
    {
        Ok(Candidate {
            program: PathBuf::from("codex"),
            source: AppServerSource::DevelopmentPath,
            arguments: Vec::new(),
        })
    }

    #[cfg(not(debug_assertions))]
    {
        Err(ProbeError::NotFound)
    }
}

fn locate_codex_from_path() -> Option<PathBuf> {
    let search_path = env::var_os("PATH")?;
    find_codex_in_search_path(&search_path)
}

fn find_codex_in_search_path(search_path: &OsStr) -> Option<PathBuf> {
    env::split_paths(search_path)
        .find_map(|directory| canonical_executable(directory.join(codex_executable_name())))
}

#[cfg(target_os = "windows")]
const fn codex_executable_name() -> &'static str {
    "codex.exe"
}

#[cfg(not(target_os = "windows"))]
const fn codex_executable_name() -> &'static str {
    "codex"
}

fn canonical_executable(candidate: PathBuf) -> Option<PathBuf> {
    let canonical = fs::canonicalize(candidate).ok()?;
    if !canonical.is_file() {
        return None;
    }

    #[cfg(unix)]
    if canonical.metadata().ok()?.permissions().mode() & 0o111 == 0 {
        return None;
    }

    Some(canonical)
}

#[cfg(target_os = "macos")]
const MACOS_CODEX_BUNDLES: [&str; 2] = ["ChatGPT.app", "Codex.app"];

#[cfg(target_os = "macos")]
const MACOS_CODEX_RELATIVE_PATH: &str = "Contents/Resources/codex";

#[cfg(target_os = "macos")]
fn locate_macos_desktop_runtime() -> Option<PathBuf> {
    let mut application_roots = vec![PathBuf::from("/Applications")];
    if let Some(home) = env::var_os("HOME") {
        application_roots.push(PathBuf::from(home).join("Applications"));
    }
    find_macos_desktop_runtime_in_roots(&application_roots)
}

#[cfg(target_os = "macos")]
fn find_macos_desktop_runtime(applications_root: &Path) -> Option<PathBuf> {
    find_macos_desktop_runtime_in_roots(&[applications_root.to_path_buf()])
}

#[cfg(target_os = "macos")]
fn find_macos_desktop_runtime_in_roots(applications_roots: &[PathBuf]) -> Option<PathBuf> {
    let roots = applications_roots
        .iter()
        .filter_map(|root| {
            fs::canonicalize(root)
                .ok()
                .map(|canonical| (root, canonical))
        })
        .collect::<Vec<_>>();

    MACOS_CODEX_BUNDLES.iter().find_map(|bundle_name| {
        roots.iter().find_map(|(root, canonical_root)| {
            validate_macos_bundle_runtime(canonical_root, &root.join(bundle_name))
        })
    })
}

#[cfg(target_os = "macos")]
fn validate_macos_bundle_runtime(
    canonical_applications_root: &Path,
    bundle: &Path,
) -> Option<PathBuf> {
    let bundle_metadata = fs::symlink_metadata(bundle).ok()?;
    if bundle_metadata.file_type().is_symlink() || !bundle_metadata.is_dir() {
        return None;
    }

    let canonical_bundle = fs::canonicalize(bundle).ok()?;
    if !canonical_bundle.starts_with(canonical_applications_root) {
        return None;
    }

    let candidate = bundle.join(MACOS_CODEX_RELATIVE_PATH);
    let candidate_metadata = fs::symlink_metadata(&candidate).ok()?;
    if candidate_metadata.file_type().is_symlink() || !candidate_metadata.is_file() {
        return None;
    }

    let canonical_candidate = canonical_executable(candidate)?;
    if !canonical_candidate.starts_with(&canonical_bundle)
        || !canonical_candidate.starts_with(canonical_applications_root)
    {
        return None;
    }

    Some(canonical_candidate)
}

#[cfg(unix)]
fn locate_common_unix_codex_runtime() -> Option<PathBuf> {
    let mut candidates = vec![
        PathBuf::from("/usr/local/bin/codex"),
        PathBuf::from("/opt/homebrew/bin/codex"),
    ];

    if let Some(home) = env::var_os("HOME") {
        let home = PathBuf::from(home);
        candidates.push(home.join(".local").join("bin").join("codex"));
        candidates.push(home.join(".npm-global").join("bin").join("codex"));
    }

    candidates.into_iter().find_map(canonical_executable)
}

#[cfg(target_os = "windows")]
fn locate_managed_codex_desktop_runtime() -> Option<PathBuf> {
    let local_app_data = env::var_os("LOCALAPPDATA")?;
    let managed_bin = PathBuf::from(local_app_data)
        .join("OpenAI")
        .join("Codex")
        .join("bin");
    find_latest_managed_codex(&managed_bin)
}

#[cfg(target_os = "windows")]
fn find_latest_managed_codex(managed_bin: &Path) -> Option<PathBuf> {
    let canonical_root = fs::canonicalize(managed_bin).ok()?;
    fs::read_dir(&canonical_root)
        .ok()?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|file_type| file_type.is_dir()))
        .filter_map(|entry| {
            let candidate = entry.path().join("codex.exe");
            let canonical_candidate = fs::canonicalize(candidate).ok()?;
            if !canonical_candidate.starts_with(&canonical_root) || !canonical_candidate.is_file() {
                return None;
            }
            let modified = canonical_candidate
                .metadata()
                .and_then(|metadata| metadata.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            Some((modified, canonical_candidate))
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, candidate)| candidate)
}

pub(super) fn spawn_child(candidate: &Candidate) -> Result<Child, ProbeError> {
    let mut command = Command::new(&candidate.program);
    command
        .arg("app-server")
        .args(&candidate.arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    // Codex CLI 是控制台程序；GUI 应用启动它时禁止 Windows 创建额外的 CMD 窗口。
    #[cfg(target_os = "windows")]
    command.creation_flags(CREATE_NO_WINDOW);

    command.spawn().map_err(|error| match error.kind() {
        ErrorKind::NotFound => ProbeError::NotFound,
        ErrorKind::PermissionDenied => ProbeError::ExecutionDenied,
        _ => ProbeError::SpawnFailed,
    })
}

#[cfg(feature = "test-support")]
async fn run_protocol_sequence(
    stdin: &mut ChildStdin,
    reader: &mut BufReader<ChildStdout>,
    request_timeout: Duration,
) -> Result<(AuthSummary, Option<ProviderQuotaData>), ProbeError> {
    request(
        stdin,
        reader,
        ClientRequest::Initialize {
            application_version: env!("CARGO_PKG_VERSION"),
        },
        1,
        request_timeout,
    )
    .await?;

    write_value(stdin, &build_initialized_notification()).await?;

    let account_value = request(
        stdin,
        reader,
        ClientRequest::AccountRead,
        2,
        request_timeout,
    )
    .await?;
    let auth = parse_account_read_result(&account_value)?;

    let quota = if auth.state == AuthUiState::Authenticated {
        let quota_value = request(
            stdin,
            reader,
            ClientRequest::RateLimitsRead,
            3,
            request_timeout,
        )
        .await?;
        Some(parse_rate_limits_result(&quota_value)?)
    } else {
        None
    };

    Ok((auth, quota))
}

#[cfg(feature = "test-support")]
async fn request(
    stdin: &mut ChildStdin,
    reader: &mut BufReader<ChildStdout>,
    request: ClientRequest<'_>,
    id: u64,
    request_timeout: Duration,
) -> Result<Value, ProbeError> {
    let value = build_request(request, id)?;
    write_value(stdin, &value).await?;

    timeout(request_timeout, read_response(stdin, reader, id))
        .await
        .map_err(|_| ProbeError::RequestTimeout)?
}

#[cfg(feature = "test-support")]
async fn write_value(stdin: &mut ChildStdin, value: &Value) -> Result<(), ProbeError> {
    let bytes = encode_jsonl(value)?;
    stdin
        .write_all(&bytes)
        .await
        .map_err(|_| ProbeError::Exited)?;
    stdin.flush().await.map_err(|_| ProbeError::Exited)
}

#[cfg(feature = "test-support")]
async fn read_response(
    stdin: &mut ChildStdin,
    reader: &mut BufReader<ChildStdout>,
    expected_id: u64,
) -> Result<Value, ProbeError> {
    loop {
        let line = read_limited_line(reader).await?;
        let Some(message) = parse_jsonl_line(&line)? else {
            continue;
        };

        match message {
            InboundMessage::Response { id, outcome } if id == expected_id => {
                return outcome.map_err(|_| ProbeError::Remote);
            }
            InboundMessage::UnsupportedServerRequest { id } => {
                write_value(stdin, &build_method_not_found_response(id)).await?;
            }
            InboundMessage::Notification(_) | InboundMessage::Response { .. } => {}
        }
    }
}

pub(super) async fn read_limited_line<R>(reader: &mut R) -> Result<Vec<u8>, ProbeError>
where
    R: AsyncBufRead + Unpin,
{
    let mut line = Vec::new();

    loop {
        let available = reader.fill_buf().await.map_err(|_| ProbeError::Exited)?;
        if available.is_empty() {
            return Err(ProbeError::Exited);
        }

        let newline = available.iter().position(|byte| *byte == b'\n');
        let take = newline.map_or(available.len(), |index| index + 1);
        let maximum_buffered = if newline.is_some() {
            MAX_MESSAGE_BYTES + 2
        } else {
            MAX_MESSAGE_BYTES
        };
        if line.len().saturating_add(take) > maximum_buffered {
            return Err(ProbeError::Protocol(
                crate::providers::app_server_protocol::message_too_large_error(),
            ));
        }

        line.extend_from_slice(&available[..take]);
        reader.consume(take);

        if newline.is_some() {
            return Ok(line);
        }
    }
}

pub(super) async fn stop_owned_child(child: &mut Child, shutdown_grace: Duration) {
    match timeout(shutdown_grace, child.wait()).await {
        Ok(_) => {}
        Err(_) => {
            let _ = child.start_kill();
            let _ = timeout(shutdown_grace, child.wait()).await;
        }
    }
}

/// 使用受控假进程执行跨进程契约测试，不会进入默认或发布构建。
#[cfg(feature = "test-support")]
#[doc(hidden)]
pub async fn run_app_server_probe_for_test(
    program: PathBuf,
    scenario: &str,
    request_timeout: Duration,
) -> Result<ProbeOutcome, ProbeError> {
    let candidate = Candidate {
        program,
        source: AppServerSource::External,
        arguments: vec![OsString::from("--scenario"), OsString::from(scenario)],
    };
    let timeouts = ProbeTimeouts {
        request: request_timeout,
        shutdown_grace: Duration::from_millis(100),
    };

    run_app_server_probe_with_candidate(candidate, timeouts).await
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    #[cfg(target_os = "macos")]
    use std::os::unix::fs::symlink;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    #[cfg(target_os = "macos")]
    use std::path::PathBuf;

    use tokio::io::{duplex, AsyncWriteExt, BufReader};

    #[cfg(target_os = "windows")]
    use super::find_latest_managed_codex;
    use super::{codex_executable_name, find_codex_in_search_path, read_limited_line, ProbeError};
    #[cfg(target_os = "macos")]
    use super::{find_macos_desktop_runtime, find_macos_desktop_runtime_in_roots};
    use crate::providers::app_server_protocol::MAX_MESSAGE_BYTES;

    #[tokio::test]
    async fn limited_reader_accepts_one_complete_json_line() {
        let (mut writer, reader) = duplex(128);
        let task = tokio::spawn(async move {
            let _ = writer.write_all(b"{\"id\":1,\"result\":{}}\n").await;
        });
        let mut reader = BufReader::new(reader);

        let line = read_limited_line(&mut reader).await;
        let _ = task.await;

        assert!(matches!(
            line.as_deref(),
            Ok(bytes) if bytes == b"{\"id\":1,\"result\":{}}\n"
        ));
    }

    #[tokio::test]
    async fn limited_reader_rejects_oversized_message_before_newline() {
        let capacity = MAX_MESSAGE_BYTES + 16;
        let (mut writer, reader) = duplex(capacity);
        let task = tokio::spawn(async move {
            let payload = vec![b'a'; MAX_MESSAGE_BYTES + 1];
            let _ = writer.write_all(&payload).await;
        });
        let mut reader = BufReader::new(reader);

        let error = read_limited_line(&mut reader).await.err();
        let _ = task.await;

        assert!(matches!(error, Some(ProbeError::Protocol(_))));
    }

    #[test]
    fn path_locator_prefers_first_executable_candidate() {
        static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let root = std::env::temp_dir().join(format!(
            "quota-glance-codex-path-{nonce}-{}",
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        let first_directory = root.join("first");
        let second_directory = root.join("second");
        assert!(fs::create_dir_all(&first_directory).is_ok());
        assert!(fs::create_dir_all(&second_directory).is_ok());

        let expected = first_directory.join(codex_executable_name());
        let fallback = second_directory.join(codex_executable_name());
        assert!(fs::write(&expected, b"first").is_ok());
        assert!(fs::write(&fallback, b"second").is_ok());

        #[cfg(unix)]
        for candidate in [&expected, &fallback] {
            if let Ok(metadata) = fs::metadata(candidate) {
                let mut permissions = metadata.permissions();
                permissions.set_mode(0o755);
                assert!(fs::set_permissions(candidate, permissions).is_ok());
            }
        }

        let search_path = std::env::join_paths([&first_directory, &second_directory]);
        assert!(search_path.is_ok());
        let located = search_path
            .ok()
            .and_then(|path| find_codex_in_search_path(&path));

        assert_eq!(located, fs::canonicalize(&expected).ok());
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(target_os = "macos")]
    fn create_macos_runtime(applications: &std::path::Path, bundle_name: &str) -> PathBuf {
        let candidate = applications
            .join(bundle_name)
            .join("Contents")
            .join("Resources")
            .join("codex");
        assert!(candidate
            .parent()
            .is_some_and(|parent| fs::create_dir_all(parent).is_ok()));
        assert!(fs::write(&candidate, b"managed codex").is_ok());
        if let Ok(metadata) = fs::metadata(&candidate) {
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o755);
            assert!(fs::set_permissions(&candidate, permissions).is_ok());
        }
        candidate
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_locator_prefers_unified_chatgpt_and_keeps_legacy_codex_fallback() {
        static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let root = std::env::temp_dir().join(format!(
            "quota-glance-macos-apps-{nonce}-{}",
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        let applications = root.join("Applications");
        assert!(fs::create_dir_all(&applications).is_ok());
        let legacy = create_macos_runtime(&applications, "Codex.app");
        let unified = create_macos_runtime(&applications, "ChatGPT.app");

        assert_eq!(
            find_macos_desktop_runtime(&applications),
            fs::canonicalize(&unified).ok()
        );
        assert!(fs::remove_dir_all(applications.join("ChatGPT.app")).is_ok());
        assert_eq!(
            find_macos_desktop_runtime(&applications),
            fs::canonicalize(&legacy).ok()
        );
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_locator_prefers_unified_chatgpt_across_installation_roots() {
        static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let root = std::env::temp_dir().join(format!(
            "quota-glance-macos-root-priority-{nonce}-{}",
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        let system_applications = root.join("system").join("Applications");
        let user_applications = root.join("user").join("Applications");
        assert!(fs::create_dir_all(&system_applications).is_ok());
        assert!(fs::create_dir_all(&user_applications).is_ok());
        let _ = create_macos_runtime(&system_applications, "Codex.app");
        let unified = create_macos_runtime(&user_applications, "ChatGPT.app");

        assert_eq!(
            find_macos_desktop_runtime_in_roots(&[system_applications, user_applications]),
            fs::canonicalize(unified).ok()
        );
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_locator_ignores_classic_unknown_and_non_executable_bundles() {
        static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let root = std::env::temp_dir().join(format!(
            "quota-glance-macos-untrusted-{nonce}-{}",
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        let applications = root.join("Applications");
        assert!(fs::create_dir_all(&applications).is_ok());
        let _ = create_macos_runtime(&applications, "ChatGPT Classic.app");
        let _ = create_macos_runtime(&applications, "Fake.app");
        let candidate = create_macos_runtime(&applications, "ChatGPT.app");
        if let Ok(metadata) = fs::metadata(&candidate) {
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o644);
            assert!(fs::set_permissions(candidate, permissions).is_ok());
        }

        assert_eq!(find_macos_desktop_runtime(&applications), None);
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_locator_rejects_bundle_and_binary_symlink_escape() {
        static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let root = std::env::temp_dir().join(format!(
            "quota-glance-macos-symlink-{nonce}-{}",
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        let applications = root.join("Applications");
        let outside_bundle = root.join("outside.app");
        assert!(fs::create_dir_all(&applications).is_ok());
        let _ = create_macos_runtime(&root, "outside.app");
        assert!(symlink(&outside_bundle, applications.join("ChatGPT.app")).is_ok());
        assert_eq!(find_macos_desktop_runtime(&applications), None);

        assert!(fs::remove_file(applications.join("ChatGPT.app")).is_ok());
        let resources = applications
            .join("ChatGPT.app")
            .join("Contents")
            .join("Resources");
        assert!(fs::create_dir_all(&resources).is_ok());
        let outside_binary = root.join("outside-codex");
        assert!(fs::write(&outside_binary, b"outside").is_ok());
        if let Ok(metadata) = fs::metadata(&outside_binary) {
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o755);
            assert!(fs::set_permissions(&outside_binary, permissions).is_ok());
        }
        assert!(symlink(&outside_binary, resources.join("codex")).is_ok());
        assert_eq!(find_macos_desktop_runtime(&applications), None);
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn managed_codex_locator_only_accepts_one_level_runtime_binary() {
        static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let root = std::env::temp_dir().join(format!(
            "quota-glance-codex-bin-{nonce}-{}",
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        let managed_directory = root.join("managed-id");
        assert!(fs::create_dir_all(&managed_directory).is_ok());
        assert!(fs::write(root.join("codex.exe"), b"ignored").is_ok());
        let expected = managed_directory.join("codex.exe");
        assert!(fs::write(&expected, b"managed").is_ok());

        let located = find_latest_managed_codex(&root);

        assert_eq!(
            located,
            fs::canonicalize(&expected).ok(),
            "只接受受控 bin 目录下一层的 codex.exe"
        );
        let _ = fs::remove_dir_all(root);
    }
}
