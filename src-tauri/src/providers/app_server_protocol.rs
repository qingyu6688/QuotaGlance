use std::fmt;

use serde_json::{json, Map, Value};

use crate::domain::{ErrorCode, QuotaError};

/// 单条 JSONL 消息的最大字节数，不包含行尾的 `\r\n` 或 `\n`。
pub const MAX_MESSAGE_BYTES: usize = 1024 * 1024;

const METHOD_NOT_FOUND_CODE: i64 = -32601;
const METHOD_NOT_FOUND_MESSAGE: &str = "Method not found";

/// QuotaGlance 1.0.0 唯一允许出现于协议边界的方法。
///
/// 业务调用方不能传入任意方法字符串，因此不存在调用 consume、logout 或 Shell
/// 方法的类型化路径。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllowedMethod {
    Initialize,
    Initialized,
    AccountRead,
    RateLimitsRead,
    AccountUpdated,
    RateLimitsUpdated,
}

impl AllowedMethod {
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::Initialize => "initialize",
            Self::Initialized => "initialized",
            Self::AccountRead => "account/read",
            Self::RateLimitsRead => "account/rateLimits/read",
            Self::AccountUpdated => "account/updated",
            Self::RateLimitsUpdated => "account/rateLimits/updated",
        }
    }

    pub fn from_wire(value: &str) -> Option<Self> {
        match value {
            "initialize" => Some(Self::Initialize),
            "initialized" => Some(Self::Initialized),
            "account/read" => Some(Self::AccountRead),
            "account/rateLimits/read" => Some(Self::RateLimitsRead),
            "account/updated" => Some(Self::AccountUpdated),
            "account/rateLimits/updated" => Some(Self::RateLimitsUpdated),
            _ => None,
        }
    }
}

/// 客户端允许主动发送的请求。通知方法和写方法不在该枚举中。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientRequest<'a> {
    Initialize { application_version: &'a str },
    AccountRead,
    RateLimitsRead,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationKind {
    AccountChanged,
    RateLimitsChanged,
    /// 未知通知不会携带原始方法名进入上层，避免被误用或写入日志。
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemoteError {
    pub code: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InboundMessage {
    Response {
        id: u64,
        outcome: Result<Value, RemoteError>,
    },
    Notification(NotificationKind),
    /// 1.0.0 不处理 App Server 主动发起的请求，只用 ID 构造固定 -32601 响应。
    UnsupportedServerRequest {
        id: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolErrorKind {
    MessageTooLarge,
    InvalidUtf8,
    InvalidJson,
    InvalidStructure,
    SerializationFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolError {
    pub kind: ProtocolErrorKind,
    detail: &'static str,
}

impl ProtocolError {
    fn new(kind: ProtocolErrorKind, detail: &'static str) -> Self {
        Self { kind, detail }
    }

    pub fn to_quota_error(&self) -> QuotaError {
        let (code, message_key, retryable) = match self.kind {
            ProtocolErrorKind::MessageTooLarge => (
                ErrorCode::ProtocolMessageTooLarge,
                "error.protocolMessageTooLarge",
                false,
            ),
            ProtocolErrorKind::InvalidUtf8
            | ProtocolErrorKind::InvalidJson
            | ProtocolErrorKind::InvalidStructure => (
                ErrorCode::ProtocolInvalidMessage,
                "error.protocolInvalidMessage",
                true,
            ),
            ProtocolErrorKind::SerializationFailed => (
                ErrorCode::ProtocolInvalidMessage,
                "error.protocolSerializationFailed",
                false,
            ),
        };

        QuotaError::new(code, message_key, retryable, None)
    }
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.detail)
    }
}

impl std::error::Error for ProtocolError {}

/// 为逐字节读取器构造与协议解析器一致的超长消息错误。
///
/// 进程层可在尚未形成完整行时提前终止读取，避免为了调用解析器而继续扩充缓冲区。
pub fn message_too_large_error() -> ProtocolError {
    ProtocolError::new(
        ProtocolErrorKind::MessageTooLarge,
        "协议消息超过 1 MiB 上限",
    )
}

/// 构造受类型约束的客户端请求。
pub fn build_request(request: ClientRequest<'_>, id: u64) -> Result<Value, ProtocolError> {
    if id == 0 {
        return Err(ProtocolError::new(
            ProtocolErrorKind::InvalidStructure,
            "请求 ID 必须为正整数",
        ));
    }

    match request {
        ClientRequest::Initialize {
            application_version,
        } => {
            validate_application_version(application_version)?;
            Ok(json!({
                "method": AllowedMethod::Initialize.wire_name(),
                "id": id,
                "params": {
                    "clientInfo": {
                        "name": "quota_glance",
                        "title": "QuotaGlance",
                        "version": application_version
                    }
                }
            }))
        }
        ClientRequest::AccountRead => Ok(json!({
            "method": AllowedMethod::AccountRead.wire_name(),
            "id": id,
            "params": { "refreshToken": false }
        })),
        ClientRequest::RateLimitsRead => Ok(json!({
            "method": AllowedMethod::RateLimitsRead.wire_name(),
            "id": id
        })),
    }
}

/// 初始化成功后发送的唯一客户端通知。
pub fn build_initialized_notification() -> Value {
    json!({
        "method": AllowedMethod::Initialized.wire_name(),
        "params": {}
    })
}

/// 为不支持的服务端请求构造固定响应，不回显方法和参数。
pub fn build_method_not_found_response(id: u64) -> Value {
    json!({
        "id": id,
        "error": {
            "code": METHOD_NOT_FOUND_CODE,
            "message": METHOD_NOT_FOUND_MESSAGE
        }
    })
}

/// 使用 JSON 序列化器编码一条带换行的出站消息。
pub fn encode_jsonl(value: &Value) -> Result<Vec<u8>, ProtocolError> {
    let mut bytes = serde_json::to_vec(value).map_err(|_| {
        ProtocolError::new(ProtocolErrorKind::SerializationFailed, "协议消息序列化失败")
    })?;

    if bytes.len() > MAX_MESSAGE_BYTES {
        return Err(message_too_large_error());
    }

    bytes.push(b'\n');
    Ok(bytes)
}

/// 解析并分类一行 App Server 输出。
///
/// 空白行返回 `Ok(None)`；其他错误均应由连接所有者按连接级错误处理。
pub fn parse_jsonl_line(line: &[u8]) -> Result<Option<InboundMessage>, ProtocolError> {
    let payload = strip_line_ending(line);

    if payload.len() > MAX_MESSAGE_BYTES {
        return Err(message_too_large_error());
    }

    if payload.iter().all(u8::is_ascii_whitespace) {
        return Ok(None);
    }

    let text = std::str::from_utf8(payload).map_err(|_| {
        ProtocolError::new(ProtocolErrorKind::InvalidUtf8, "协议消息不是有效 UTF-8")
    })?;
    let value: Value = serde_json::from_str(text)
        .map_err(|_| ProtocolError::new(ProtocolErrorKind::InvalidJson, "协议消息不是有效 JSON"))?;
    let object = value.as_object().ok_or_else(|| {
        ProtocolError::new(
            ProtocolErrorKind::InvalidStructure,
            "协议消息顶层必须是对象",
        )
    })?;

    classify_object(object).map(Some)
}

fn classify_object(object: &Map<String, Value>) -> Result<InboundMessage, ProtocolError> {
    let has_id = object.contains_key("id");
    let has_result = object.contains_key("result");
    let has_error = object.contains_key("error");

    // 必须按文档顺序先判断响应，避免把错误响应误判为服务端请求。
    if has_id && (has_result || has_error) {
        if has_result == has_error {
            return Err(invalid_structure("响应必须且只能包含 result 或 error"));
        }

        let id = parse_id(object.get("id"))?;
        if let Some(result) = object.get("result") {
            return Ok(InboundMessage::Response {
                id,
                outcome: Ok(result.clone()),
            });
        }

        let remote_error = parse_remote_error(object.get("error"))?;
        return Ok(InboundMessage::Response {
            id,
            outcome: Err(remote_error),
        });
    }

    if let Some(method_value) = object.get("method") {
        let method = method_value
            .as_str()
            .ok_or_else(|| invalid_structure("协议 method 必须是字符串"))?;

        if !has_id {
            return Ok(InboundMessage::Notification(classify_notification(method)));
        }

        return Ok(InboundMessage::UnsupportedServerRequest {
            id: parse_id(object.get("id"))?,
        });
    }

    Err(invalid_structure("无法识别协议消息结构"))
}

fn parse_remote_error(value: Option<&Value>) -> Result<RemoteError, ProtocolError> {
    let object = value
        .and_then(Value::as_object)
        .ok_or_else(|| invalid_structure("协议 error 必须是对象"))?;
    let code = object
        .get("code")
        .and_then(Value::as_i64)
        .ok_or_else(|| invalid_structure("协议 error.code 必须是整数"))?;

    Ok(RemoteError { code })
}

fn parse_id(value: Option<&Value>) -> Result<u64, ProtocolError> {
    let id = value
        .and_then(Value::as_u64)
        .ok_or_else(|| invalid_structure("协议 id 必须是无符号整数"))?;

    if id == 0 {
        return Err(invalid_structure("协议 id 必须是正整数"));
    }

    Ok(id)
}

fn classify_notification(method: &str) -> NotificationKind {
    match AllowedMethod::from_wire(method) {
        Some(AllowedMethod::AccountUpdated) => NotificationKind::AccountChanged,
        Some(AllowedMethod::RateLimitsUpdated) => NotificationKind::RateLimitsChanged,
        _ => NotificationKind::Unknown,
    }
}

fn strip_line_ending(line: &[u8]) -> &[u8] {
    let without_lf = line.strip_suffix(b"\n").unwrap_or(line);
    without_lf.strip_suffix(b"\r").unwrap_or(without_lf)
}

fn validate_application_version(version: &str) -> Result<(), ProtocolError> {
    let length = version.chars().count();
    if length == 0 || length > 64 || version.chars().any(char::is_control) {
        return Err(invalid_structure("应用版本号不符合协议约束"));
    }

    Ok(())
}

fn invalid_structure(detail: &'static str) -> ProtocolError {
    ProtocolError::new(ProtocolErrorKind::InvalidStructure, detail)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        build_initialized_notification, build_method_not_found_response, build_request,
        parse_jsonl_line, ClientRequest, InboundMessage, NotificationKind, ProtocolErrorKind,
        MAX_MESSAGE_BYTES,
    };

    #[test]
    fn initialize_request_omits_experimental_capabilities() {
        let request = build_request(
            ClientRequest::Initialize {
                application_version: "1.0.0",
            },
            1,
        );

        assert_eq!(
            request.as_ref().ok().and_then(|item| item["id"].as_u64()),
            Some(1)
        );
        assert_eq!(
            request
                .as_ref()
                .ok()
                .and_then(|item| item["params"]["clientInfo"]["name"].as_str()),
            Some("quota_glance")
        );
        assert!(request
            .as_ref()
            .ok()
            .is_some_and(|item| item.get("capabilities").is_none()));
        assert!(!request
            .as_ref()
            .ok()
            .is_some_and(|item| item.to_string().contains("experimentalApi")));
    }

    #[test]
    fn only_read_only_request_variants_are_constructed() {
        let account = build_request(ClientRequest::AccountRead, 2);
        let quota = build_request(ClientRequest::RateLimitsRead, 3);
        let initialized = build_initialized_notification();

        assert_eq!(
            account
                .as_ref()
                .ok()
                .and_then(|item| item["method"].as_str()),
            Some("account/read")
        );
        assert_eq!(
            account
                .as_ref()
                .ok()
                .and_then(|item| item["params"]["refreshToken"].as_bool()),
            Some(false)
        );
        assert_eq!(
            quota.as_ref().ok().and_then(|item| item["method"].as_str()),
            Some("account/rateLimits/read")
        );
        assert_eq!(initialized["method"], "initialized");
    }

    #[test]
    fn classifies_known_and_unknown_notifications_without_using_payload() {
        let quota = parse_jsonl_line(
            br#"{"method":"account/rateLimits/updated","params":{"rateLimits":{}}}"#,
        );
        let account = parse_jsonl_line(br#"{"method":"account/updated","params":{}}"#);
        let unknown = parse_jsonl_line(br#"{"method":"thread/updated","params":{}}"#);

        assert_eq!(
            quota,
            Ok(Some(InboundMessage::Notification(
                NotificationKind::RateLimitsChanged
            )))
        );
        assert_eq!(
            account,
            Ok(Some(InboundMessage::Notification(
                NotificationKind::AccountChanged
            )))
        );
        assert_eq!(
            unknown,
            Ok(Some(InboundMessage::Notification(
                NotificationKind::Unknown
            )))
        );
    }

    #[test]
    fn classifies_success_and_error_responses() {
        let success = parse_jsonl_line(br#"{"id":3,"result":{"rateLimits":null}}"#);
        let failure = parse_jsonl_line(br#"{"id":4,"error":{"code":-32000,"message":"x"}}"#);

        assert!(matches!(
            success,
            Ok(Some(InboundMessage::Response {
                id: 3,
                outcome: Ok(_)
            }))
        ));
        assert!(matches!(
            failure,
            Ok(Some(InboundMessage::Response {
                id: 4,
                outcome: Err(_)
            }))
        ));
    }

    #[test]
    fn rejects_oversized_message_before_json_parsing() {
        let line = vec![b'a'; MAX_MESSAGE_BYTES + 1];
        let error = parse_jsonl_line(&line).err();

        assert_eq!(
            error.as_ref().map(|item| item.kind),
            Some(ProtocolErrorKind::MessageTooLarge)
        );
    }

    #[test]
    fn empty_lines_are_ignored_and_malformed_json_is_rejected() {
        assert_eq!(parse_jsonl_line(b" \t\r\n"), Ok(None));
        assert_eq!(
            parse_jsonl_line(b"{not-json}\n")
                .err()
                .map(|item| item.kind),
            Some(ProtocolErrorKind::InvalidJson)
        );
    }

    #[test]
    fn server_request_gets_fixed_method_not_found_response() {
        let inbound =
            parse_jsonl_line(br#"{"method":"some/new/request","id":9,"params":{"secret":"x"}}"#);
        let response = build_method_not_found_response(9);

        assert_eq!(
            inbound,
            Ok(Some(InboundMessage::UnsupportedServerRequest { id: 9 }))
        );
        assert_eq!(
            response,
            json!({
                "id": 9,
                "error": {"code": -32601, "message": "Method not found"}
            })
        );
        assert!(!response.to_string().contains("secret"));
    }

    #[test]
    fn response_cannot_contain_result_and_error_together() {
        let error = parse_jsonl_line(br#"{"id":1,"result":{},"error":{"code":-1}}"#).err();

        assert_eq!(
            error.map(|item| item.kind),
            Some(ProtocolErrorKind::InvalidStructure)
        );
    }
}
