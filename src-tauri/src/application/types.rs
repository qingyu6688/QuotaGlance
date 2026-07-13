use serde::{Deserialize, Serialize};

use crate::domain::{ErrorCode, QuotaError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AppServerPhase {
    Stopped,
    Locating,
    Starting,
    Initializing,
    Ready,
    BackingOff,
    Failed,
    Incompatible,
    ShuttingDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AppServerSource {
    Bundled,
    External,
    DevelopmentPath,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppServerStatus {
    pub revision: u64,
    pub phase: AppServerPhase,
    pub source: Option<AppServerSource>,
    pub version: Option<String>,
    pub target_triple: Option<String>,
    pub restart_attempt: u32,
    pub next_retry_at: Option<String>,
    pub error: Option<QuotaError>,
}

impl Default for AppServerStatus {
    fn default() -> Self {
        Self {
            revision: 0,
            phase: AppServerPhase::Stopped,
            source: None,
            version: None,
            target_triple: None,
            restart_attempt: 0,
            next_retry_at: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RefreshReason {
    Startup,
    Manual,
    AccountNotification,
    QuotaNotification,
    VisibleResync,
    HiddenResync,
    Resume,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RefreshPhase {
    Idle,
    Refreshing,
    Cooldown,
    BackingOff,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshState {
    pub revision: u64,
    pub phase: RefreshPhase,
    pub reason: Option<RefreshReason>,
    pub started_at: Option<String>,
    pub next_allowed_manual_refresh_at: Option<String>,
    pub next_retry_at: Option<String>,
}

impl Default for RefreshState {
    fn default() -> Self {
        Self {
            revision: 0,
            phase: RefreshPhase::Idle,
            reason: None,
            started_at: None,
            next_allowed_manual_refresh_at: None,
            next_retry_at: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshReceipt {
    pub accepted: bool,
    pub joined_existing_request: bool,
    pub request_revision: u64,
    pub state: RefreshState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Locale {
    #[serde(rename = "system")]
    System,
    #[serde(rename = "zh-CN")]
    ZhCn,
    #[serde(rename = "en")]
    En,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    System,
    #[serde(alias = "light")]
    Aurora,
    #[serde(alias = "dark")]
    Graphite,
    Paper,
    Sunset,
    Honey,
    Rose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WidgetMode {
    Orb,
    Card,
    Hidden,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub monitor_id: Option<String>,
    pub scale_factor_at_save: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectedQuota {
    pub limit_id: Option<String>,
    pub slot: Option<crate::domain::WindowSlot>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoundsByMode {
    pub orb: Option<WindowBounds>,
    pub card: Option<WindowBounds>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetPreferences {
    pub mode: WidgetMode,
    pub always_on_top: bool,
    pub click_through: bool,
    pub selected_quota: SelectedQuota,
    pub bounds_by_mode: BoundsByMode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationPreferences {
    pub enabled: bool,
    pub warning_remaining_percent: f64,
    pub critical_remaining_percent: f64,
    pub notify_when_recovered: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupPreferences {
    pub launch_at_login: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePreferences {
    pub auto_check: bool,
    pub channel: String,
    pub last_checked_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    pub schema_version: u32,
    pub revision: u64,
    pub locale: Locale,
    pub theme: Theme,
    pub widget: WidgetPreferences,
    pub notifications: NotificationPreferences,
    pub startup: StartupPreferences,
    pub updates: UpdatePreferences,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            schema_version: 1,
            revision: 0,
            locale: Locale::ZhCn,
            theme: Theme::System,
            widget: WidgetPreferences {
                mode: WidgetMode::Card,
                always_on_top: true,
                click_through: false,
                selected_quota: SelectedQuota {
                    limit_id: None,
                    slot: None,
                },
                bounds_by_mode: BoundsByMode {
                    orb: None,
                    card: None,
                },
            },
            notifications: NotificationPreferences {
                enabled: false,
                warning_remaining_percent: 50.0,
                critical_remaining_percent: 10.0,
                notify_when_recovered: false,
            },
            startup: StartupPreferences {
                launch_at_login: false,
            },
            updates: UpdatePreferences {
                auto_check: true,
                channel: "stable".to_owned(),
                last_checked_at: None,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreferencesRecovery {
    pub source: String,
    pub reason_code: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreferencesEnvelope {
    pub preferences: Preferences,
    pub recovery: Option<PreferencesRecovery>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowState {
    pub revision: u64,
    pub mode: WidgetMode,
    pub visible: bool,
    pub always_on_top: bool,
    pub click_through: bool,
    pub bounds: Option<WindowBounds>,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            revision: 0,
            mode: WidgetMode::Card,
            visible: true,
            always_on_top: true,
            click_through: false,
            bounds: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpcContext {
    pub field: Option<String>,
    pub current_revision: Option<u64>,
    pub app_server_phase: Option<AppServerPhase>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpcError {
    pub code: ErrorCode,
    pub message_key: String,
    pub retryable: bool,
    pub retry_after_ms: Option<u64>,
    pub context: IpcContext,
}

impl IpcError {
    pub fn new(
        code: ErrorCode,
        message_key: impl Into<String>,
        retryable: bool,
        retry_after_ms: Option<u64>,
    ) -> Self {
        Self {
            code,
            message_key: message_key.into(),
            retryable,
            retry_after_ms,
            context: IpcContext {
                field: None,
                current_revision: None,
                app_server_phase: None,
            },
        }
    }
}

impl From<QuotaError> for IpcError {
    fn from(value: QuotaError) -> Self {
        Self::new(
            value.code,
            value.message_key,
            value.retryable,
            value.retry_after_ms,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UpdatePhase {
    Idle,
    Checking,
    UpToDate,
    Available,
    Downloading,
    ReadyToInstall,
    Installing,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatus {
    pub revision: u64,
    pub phase: UpdatePhase,
    pub current_version: String,
    pub available_version: Option<String>,
    pub checked_at: Option<String>,
    pub error: Option<QuotaError>,
}

impl UpdateStatus {
    pub fn idle(current_version: impl Into<String>) -> Self {
        Self {
            revision: 0,
            phase: UpdatePhase::Idle,
            current_version: current_version.into(),
            available_version: None,
            checked_at: None,
            error: None,
        }
    }
}
