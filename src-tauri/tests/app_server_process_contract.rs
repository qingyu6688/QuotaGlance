#![cfg(feature = "test-support")]

use std::{path::PathBuf, time::Duration};

use quota_glance_lib::{
    domain::{AuthUiState, WindowSlot},
    infrastructure::{run_app_server_probe_for_test, ProbeError},
    providers::app_server_protocol::ProtocolErrorKind,
};

fn fake_app_server_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_fake-app-server"))
}

#[tokio::test]
async fn fake_process_validates_complete_read_only_handshake() {
    let outcome =
        run_app_server_probe_for_test(fake_app_server_path(), "success", Duration::from_secs(2))
            .await;
    let quota = outcome.as_ref().ok().and_then(|item| item.quota.as_ref());
    let first_window = quota
        .and_then(|item| item.buckets.first())
        .and_then(|item| item.windows.first());

    assert_eq!(
        outcome.as_ref().ok().map(|item| item.auth.state),
        Some(AuthUiState::Authenticated)
    );
    assert_eq!(quota.map(|item| item.buckets.len()), Some(1));
    assert_eq!(
        first_window.map(|item| item.slot),
        Some(WindowSlot::Primary)
    );
    assert_eq!(first_window.map(|item| item.remaining_percent), Some(74.0));
}

#[tokio::test]
async fn fake_process_timeout_is_mapped_and_child_is_reaped_promptly() {
    let started = std::time::Instant::now();
    let error = run_app_server_probe_for_test(
        fake_app_server_path(),
        "timeout",
        Duration::from_millis(100),
    )
    .await
    .err();

    assert!(matches!(error, Some(ProbeError::RequestTimeout)));
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[tokio::test]
async fn fake_process_early_exit_is_mapped_without_raw_stderr() {
    let error =
        run_app_server_probe_for_test(fake_app_server_path(), "exit", Duration::from_secs(1))
            .await
            .err();

    assert!(matches!(error, Some(ProbeError::Exited)));
    assert_eq!(
        error.as_ref().map(|item| item.to_quota_error().message_key),
        Some("error.appServerExited".to_owned())
    );
}

#[tokio::test]
async fn fake_process_oversized_stdout_is_rejected_before_json_parsing() {
    let error =
        run_app_server_probe_for_test(fake_app_server_path(), "oversized", Duration::from_secs(2))
            .await
            .err();

    assert!(matches!(
        error,
        Some(ProbeError::Protocol(ref protocol_error))
            if protocol_error.kind == ProtocolErrorKind::MessageTooLarge
    ));
}

#[tokio::test]
async fn fake_process_remote_error_is_mapped_without_raw_message() {
    let error = run_app_server_probe_for_test(
        fake_app_server_path(),
        "remote-error",
        Duration::from_secs(1),
    )
    .await
    .err();

    assert!(matches!(error, Some(ProbeError::Remote)));
    assert_eq!(
        error.as_ref().map(|item| item.to_quota_error().message_key),
        Some("error.serviceUnavailable".to_owned())
    );
    assert!(error
        .as_ref()
        .is_some_and(|item| !item.to_string().contains("raw-remote-detail")));
}

#[tokio::test]
async fn fake_process_malformed_stdout_is_rejected_as_invalid_json() {
    let error =
        run_app_server_probe_for_test(fake_app_server_path(), "malformed", Duration::from_secs(1))
            .await
            .err();

    assert!(matches!(
        error,
        Some(ProbeError::Protocol(ref protocol_error))
            if protocol_error.kind == ProtocolErrorKind::InvalidJson
    ));
}
