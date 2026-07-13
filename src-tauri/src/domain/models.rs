use serde::{Deserialize, Serialize};

use super::QuotaError;

pub const QUOTA_SNAPSHOT_SCHEMA_VERSION: u32 = 1;

/// 快照主状态。状态不可被缺失或非法额度伪装成正常的零值。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QuotaStatus {
    Loading,
    Ok,
    Stale,
    SignedOut,
    ApiKeyMode,
    QuotaReached,
    SourceBusy,
    Offline,
    ServiceUnavailable,
    AppServerUnavailable,
    Incompatible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AuthUiState {
    SignedOut,
    Authenticated,
    ApiKeyMode,
    ExternalProvider,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthSummary {
    pub state: AuthUiState,
    pub auth_mode: Option<String>,
    pub plan_type: Option<String>,
    pub requires_openai_auth: Option<bool>,
}

impl AuthSummary {
    pub fn unknown() -> Self {
        Self {
            state: AuthUiState::Unknown,
            auth_mode: None,
            plan_type: None,
            requires_openai_auth: None,
        }
    }
}

impl Default for AuthSummary {
    fn default() -> Self {
        Self::unknown()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QuotaSource {
    AppServer,
    LegacyCompat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProviderKind {
    CodexAppServer,
    LegacyWham,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WindowSlot {
    Primary,
    Secondary,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WindowKind {
    ShortTerm,
    Weekly,
    Monthly,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaWindow {
    pub slot: WindowSlot,
    pub kind: WindowKind,
    pub label: String,
    pub used_percent: f64,
    pub remaining_percent: f64,
    pub window_duration_mins: u32,
    /// UTC RFC 3339 时间。
    pub resets_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreditSummary {
    pub has_credits: Option<bool>,
    pub unlimited: Option<bool>,
    /// 保留经过校验的服务端字符串，不把余额转换为浮点数或猜测货币。
    pub balance: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaBucket {
    pub limit_id: String,
    pub limit_name: Option<String>,
    pub plan_type: Option<String>,
    pub windows: Vec<QuotaWindow>,
    pub credits: Option<CreditSummary>,
    pub rate_limit_reached_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetCreditDetail {
    pub reset_type: String,
    pub status: String,
    /// UTC RFC 3339 时间。
    pub granted_at: String,
    /// UTC RFC 3339 时间。
    pub expires_at: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetCreditSummary {
    pub available_count: u32,
    /// `None` 表示服务端只给出数量；空数组表示已返回明细但没有项目。
    pub details: Option<Vec<ResetCreditDetail>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaSnapshot {
    pub schema_version: u32,
    pub revision: u64,
    pub source: Option<QuotaSource>,
    pub provider: Option<ProviderKind>,
    pub auth: AuthSummary,
    pub buckets: Vec<QuotaBucket>,
    pub banked_resets: Option<ResetCreditSummary>,
    pub status: QuotaStatus,
    /// UTC RFC 3339 时间。
    pub fetched_at: Option<String>,
    /// UTC RFC 3339 时间。
    pub last_good_at: Option<String>,
    /// UTC RFC 3339 时间。
    pub next_retry_at: Option<String>,
    pub error: Option<QuotaError>,
}

impl QuotaSnapshot {
    /// 构造首次启动快照，不生成任何虚假额度窗口。
    pub fn loading(revision: u64) -> Self {
        Self {
            schema_version: QUOTA_SNAPSHOT_SCHEMA_VERSION,
            revision,
            source: None,
            provider: None,
            auth: AuthSummary::unknown(),
            buckets: Vec::new(),
            banked_resets: None,
            status: QuotaStatus::Loading,
            fetched_at: None,
            last_good_at: None,
            next_retry_at: None,
            error: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{QuotaSnapshot, QuotaStatus};

    #[test]
    fn loading_snapshot_does_not_contain_fake_quota() {
        let snapshot = QuotaSnapshot::loading(0);

        assert_eq!(snapshot.status, QuotaStatus::Loading);
        assert!(snapshot.buckets.is_empty());
        assert!(snapshot.fetched_at.is_none());
    }

    #[test]
    fn ipc_fields_are_serialized_as_camel_case() {
        let value = serde_json::to_value(QuotaSnapshot::loading(3));
        let object = value.ok().and_then(|item| item.as_object().cloned());

        assert!(object
            .as_ref()
            .is_some_and(|item| item.contains_key("schemaVersion")));
        assert!(object
            .as_ref()
            .is_some_and(|item| item.contains_key("lastGoodAt")));
    }
}
