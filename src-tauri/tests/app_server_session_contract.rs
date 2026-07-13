#![cfg(feature = "test-support")]

use std::{path::PathBuf, time::Duration};

use quota_glance_lib::{
    domain::AuthUiState,
    infrastructure::{AppServerSession, ProbeError},
    providers::app_server_protocol::NotificationKind,
};

fn fake_app_server_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_fake-app-server"))
}

#[tokio::test]
async fn pending_map_routes_out_of_order_responses_and_forwards_notifications() {
    let session = AppServerSession::connect_for_test(
        fake_app_server_path(),
        "persistent-concurrent",
        Duration::from_secs(2),
    )
    .await
    .expect("常驻测试会话应完成初始化");
    let mut notifications = session.subscribe_notifications();

    let (first, second) = tokio::join!(
        session.read_account_and_quota(),
        session.read_account_and_quota()
    );

    assert_eq!(
        first.as_ref().ok().map(|outcome| outcome.auth.state),
        Some(AuthUiState::Authenticated)
    );
    assert_eq!(
        second.as_ref().ok().map(|outcome| outcome.auth.state),
        Some(AuthUiState::Authenticated)
    );
    assert_eq!(
        notifications.recv().await.ok(),
        Some(NotificationKind::AccountChanged)
    );
    assert_eq!(
        notifications.recv().await.ok(),
        Some(NotificationKind::Unknown)
    );

    session.shutdown().await;
}

#[tokio::test]
async fn timed_out_pending_entry_is_removed_and_late_response_is_ignored() {
    let session = AppServerSession::connect_for_test(
        fake_app_server_path(),
        "persistent-late-response",
        Duration::from_millis(100),
    )
    .await
    .expect("常驻测试会话应完成初始化");

    let first = session.read_account_and_quota().await;
    assert!(matches!(first, Err(ProbeError::RequestTimeout)));

    tokio::time::sleep(Duration::from_millis(200)).await;
    let second = session.read_account_and_quota().await;
    assert_eq!(
        second.as_ref().ok().map(|outcome| outcome.auth.state),
        Some(AuthUiState::Authenticated)
    );

    session.shutdown().await;
}

#[tokio::test]
async fn process_exit_fails_every_pending_request() {
    let session = AppServerSession::connect_for_test(
        fake_app_server_path(),
        "persistent-exit",
        Duration::from_secs(2),
    )
    .await
    .expect("常驻测试会话应完成初始化");

    let (first, second) = tokio::join!(
        session.read_account_and_quota(),
        session.read_account_and_quota()
    );

    assert!(matches!(first, Err(ProbeError::Exited)));
    assert!(matches!(second, Err(ProbeError::Exited)));
    assert!(session.is_closed());
    session.shutdown().await;
}
