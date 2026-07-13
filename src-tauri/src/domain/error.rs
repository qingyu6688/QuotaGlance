use serde::{Deserialize, Serialize};

/// 可穿越 IPC 的稳定错误码。
///
/// 错误码只描述可安全公开的分类，不携带 App Server 原始消息、路径或账号信息。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    InvalidArgument,
    Forbidden,
    NotReady,
    ShuttingDown,
    AppServerNotFound,
    AppServerExecutionDenied,
    AppServerVersionIncompatible,
    AppServerHandshakeTimeout,
    AppServerExited,
    ProtocolInvalidMessage,
    ProtocolMessageTooLarge,
    ProtocolRequestTimeout,
    AuthRequired,
    ApiKeyMode,
    RateLimitsUnavailable,
    ResponseIncompatible,
    SourceBusy,
    Offline,
    ServiceUnavailable,
    RefreshCooldown,
    PreferencesConflict,
    PreferencesCorrupted,
    PreferencesVersionUnsupported,
    PreferencesWriteFailed,
    WindowOperationFailed,
    UpdateCheckFailed,
    UpdateSignatureInvalid,
    UpdateInstallFailed,
}

/// 面向前端的脱敏错误。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaError {
    pub code: ErrorCode,
    pub message_key: String,
    pub retryable: bool,
    pub retry_after_ms: Option<u64>,
}

impl QuotaError {
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ErrorCode;

    #[test]
    fn error_code_uses_documented_wire_format() {
        let value = serde_json::to_string(&ErrorCode::ProtocolMessageTooLarge);

        assert_eq!(
            value.ok().as_deref(),
            Some("\"PROTOCOL_MESSAGE_TOO_LARGE\"")
        );
    }
}
