use std::{cmp::Ordering, fmt};

use serde_json::{Map, Value};

use crate::domain::{
    AuthSummary, AuthUiState, CreditSummary, ErrorCode, QuotaBucket, QuotaError, QuotaWindow,
    ResetCreditDetail, ResetCreditSummary, WindowKind, WindowSlot,
};

const MAX_LIMIT_ID_CHARS: usize = 128;
const MAX_LIMIT_NAME_CHARS: usize = 256;
const MAX_EXTERNAL_ENUM_CHARS: usize = 128;
const MAX_BALANCE_CHARS: usize = 128;
const MAX_RESET_TEXT_CHARS: usize = 512;
const MAX_WINDOW_DURATION_MINS: u64 = 5_256_000;
const MAX_UNIX_SECONDS: i64 = 253_402_300_799;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidationKind {
    AccountChanged,
    RateLimitsChanged,
}

/// Provider 已校验但尚未进入快照状态机的额度结果。
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderQuotaData {
    pub buckets: Vec<QuotaBucket>,
    pub banked_resets: Option<ResetCreditSummary>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderErrorKind {
    ResponseIncompatible,
    RateLimitsUnavailable,
}

/// Provider 内部错误只保留固定中文说明，不携带服务端值和账号信息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderError {
    pub kind: ProviderErrorKind,
    detail: &'static str,
}

impl ProviderError {
    fn incompatible(detail: &'static str) -> Self {
        Self {
            kind: ProviderErrorKind::ResponseIncompatible,
            detail,
        }
    }

    fn unavailable() -> Self {
        Self {
            kind: ProviderErrorKind::RateLimitsUnavailable,
            detail: "App Server 未返回可用的订阅额度",
        }
    }

    pub fn to_quota_error(&self) -> QuotaError {
        match self.kind {
            ProviderErrorKind::ResponseIncompatible => QuotaError::new(
                ErrorCode::ResponseIncompatible,
                "error.responseIncompatible",
                false,
                None,
            ),
            ProviderErrorKind::RateLimitsUnavailable => QuotaError::new(
                ErrorCode::RateLimitsUnavailable,
                "error.rateLimitsUnavailable",
                true,
                None,
            ),
        }
    }
}

impl fmt::Display for ProviderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.detail)
    }
}

impl std::error::Error for ProviderError {}

/// 归一化 `account.type` 或 `account/updated.authMode`。
///
/// 未知值经安全字符串校验后原样保留，但 UI 状态固定降级为 `unknown`。
pub fn normalize_auth_mode(
    raw_mode: Option<&str>,
    plan_type: Option<&str>,
    requires_openai_auth: Option<bool>,
) -> Result<AuthSummary, ProviderError> {
    let plan_type = validate_optional_external_string(
        plan_type,
        MAX_EXTERNAL_ENUM_CHARS,
        "账户 planType 不符合约束",
    )?;

    let (state, auth_mode) = match raw_mode {
        Some("apiKey" | "apikey") => (AuthUiState::ApiKeyMode, Some("apikey".to_owned())),
        Some("chatgpt") => (AuthUiState::Authenticated, Some("chatgpt".to_owned())),
        Some("chatgptAuthTokens") => (
            AuthUiState::Authenticated,
            Some("chatgptAuthTokens".to_owned()),
        ),
        Some("agentIdentity") => (AuthUiState::Authenticated, Some("agentIdentity".to_owned())),
        Some("personalAccessToken") => (
            AuthUiState::Authenticated,
            Some("personalAccessToken".to_owned()),
        ),
        Some("amazonBedrock" | "bedrockApiKey") => (
            AuthUiState::ExternalProvider,
            Some("bedrockApiKey".to_owned()),
        ),
        Some(other) => {
            let safe_mode = validate_required_external_string(
                other,
                MAX_EXTERNAL_ENUM_CHARS,
                "账户 authMode 不符合约束",
            )?;
            (AuthUiState::Unknown, Some(safe_mode))
        }
        None if requires_openai_auth == Some(true) => (AuthUiState::SignedOut, None),
        None if requires_openai_auth == Some(false) => (AuthUiState::ExternalProvider, None),
        None => (AuthUiState::Unknown, None),
    };

    Ok(AuthSummary {
        state,
        auth_mode,
        plan_type,
        requires_openai_auth,
    })
}

/// 解析 `account/read` 的 `result`。邮箱和其他未使用字段在此处直接丢弃。
pub fn parse_account_read_result(result: &Value) -> Result<AuthSummary, ProviderError> {
    let object = result
        .as_object()
        .ok_or_else(|| ProviderError::incompatible("账户响应顶层必须是对象"))?;
    let requires_openai_auth = optional_bool(object, "requiresOpenaiAuth")?;

    match object.get("account") {
        None | Some(Value::Null) => normalize_auth_mode(None, None, requires_openai_auth),
        Some(Value::Object(account)) => {
            let account_type = account
                .get("type")
                .and_then(Value::as_str)
                .ok_or_else(|| ProviderError::incompatible("账户 type 字段缺失或无效"))?;
            let plan_type = optional_string_value(account, "planType")?;

            normalize_auth_mode(
                Some(account_type),
                plan_type.as_deref(),
                requires_openai_auth,
            )
        }
        Some(_) => Err(ProviderError::incompatible(
            "账户 account 字段必须是对象或 null",
        )),
    }
}

/// 解析 `account/rateLimits/read` 的 `result`。
///
/// `rateLimitsByLimitId` 只要出现就不会回退单桶；对象为空也是权威结果。
pub fn parse_rate_limits_result(result: &Value) -> Result<ProviderQuotaData, ProviderError> {
    let object = result
        .as_object()
        .ok_or_else(|| ProviderError::incompatible("额度响应顶层必须是对象"))?;

    let mut buckets = match object.get("rateLimitsByLimitId") {
        Some(Value::Object(items)) => parse_multi_bucket_view(items)?,
        Some(Value::Null) => return Err(ProviderError::unavailable()),
        Some(_) => {
            return Err(ProviderError::incompatible(
                "rateLimitsByLimitId 必须是对象或 null",
            ))
        }
        None => match object.get("rateLimits") {
            Some(Value::Object(bucket)) => vec![parse_bucket(bucket, None)?],
            None | Some(Value::Null) => return Err(ProviderError::unavailable()),
            Some(_) => return Err(ProviderError::incompatible("rateLimits 必须是对象或 null")),
        },
    };

    sort_buckets(&mut buckets);

    // Reset credits 是可选扩展；单独不兼容时局部降级，不影响合法额度桶。
    let banked_resets = object
        .get("rateLimitResetCredits")
        .and_then(|value| parse_reset_credit_summary(value).ok().flatten());

    Ok(ProviderQuotaData {
        buckets,
        banked_resets,
    })
}

fn parse_multi_bucket_view(items: &Map<String, Value>) -> Result<Vec<QuotaBucket>, ProviderError> {
    let mut buckets = Vec::with_capacity(items.len());

    for (map_key, value) in items {
        let Some(bucket_object) = value.as_object() else {
            continue;
        };

        if let Ok(bucket) = parse_bucket(bucket_object, Some(map_key)) {
            buckets.push(bucket);
        }
    }

    if !items.is_empty() && buckets.is_empty() {
        return Err(ProviderError::incompatible(
            "多桶响应中没有可安全解析的额度桶",
        ));
    }

    Ok(buckets)
}

fn parse_bucket(
    object: &Map<String, Value>,
    expected_limit_id: Option<&str>,
) -> Result<QuotaBucket, ProviderError> {
    let raw_limit_id = object
        .get("limitId")
        .and_then(Value::as_str)
        .ok_or_else(|| ProviderError::incompatible("额度桶 limitId 字段缺失或无效"))?;
    let limit_id = validate_required_external_string(
        raw_limit_id,
        MAX_LIMIT_ID_CHARS,
        "额度桶 limitId 不符合约束",
    )?;

    if let Some(map_key) = expected_limit_id {
        let safe_map_key = validate_required_external_string(
            map_key,
            MAX_LIMIT_ID_CHARS,
            "额度桶 Map key 不符合约束",
        )?;
        if safe_map_key != limit_id {
            return Err(ProviderError::incompatible(
                "额度桶 Map key 与 limitId 不一致",
            ));
        }
    }

    let limit_name = parse_optional_safe_string(
        object,
        "limitName",
        MAX_LIMIT_NAME_CHARS,
        "额度桶 limitName 不符合约束",
    )?;
    let plan_type = parse_optional_safe_string(
        object,
        "planType",
        MAX_EXTERNAL_ENUM_CHARS,
        "额度桶 planType 不符合约束",
    )?;
    let rate_limit_reached_type = parse_optional_safe_string(
        object,
        "rateLimitReachedType",
        MAX_EXTERNAL_ENUM_CHARS,
        "额度桶 rateLimitReachedType 不符合约束",
    )?;

    let mut windows = Vec::with_capacity(2);
    parse_window_if_valid(object.get("primary"), WindowSlot::Primary, &mut windows);
    parse_window_if_valid(object.get("secondary"), WindowSlot::Secondary, &mut windows);

    if windows.is_empty() {
        return Err(ProviderError::incompatible("额度桶中没有可安全解析的窗口"));
    }

    let credits = match object.get("credits") {
        None | Some(Value::Null) => None,
        Some(Value::Object(value)) => Some(parse_credit_summary(value)?),
        Some(_) => {
            return Err(ProviderError::incompatible(
                "额度桶 credits 必须是对象或 null",
            ))
        }
    };

    Ok(QuotaBucket {
        limit_id,
        limit_name,
        plan_type,
        windows,
        credits,
        rate_limit_reached_type,
    })
}

fn parse_window_if_valid(value: Option<&Value>, slot: WindowSlot, windows: &mut Vec<QuotaWindow>) {
    let Some(Value::Object(object)) = value else {
        return;
    };

    if let Ok(window) = parse_window(object, slot) {
        windows.push(window);
    }
}

fn parse_window(
    object: &Map<String, Value>,
    slot: WindowSlot,
) -> Result<QuotaWindow, ProviderError> {
    let used_percent = object
        .get("usedPercent")
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite() && (0.0..=100.0).contains(value))
        .ok_or_else(|| ProviderError::incompatible("窗口 usedPercent 缺失或越界"))?;

    let duration = object
        .get("windowDurationMins")
        .and_then(Value::as_u64)
        .filter(|value| *value > 0 && *value <= MAX_WINDOW_DURATION_MINS)
        .ok_or_else(|| ProviderError::incompatible("窗口 windowDurationMins 缺失或越界"))?;
    let window_duration_mins =
        u32::try_from(duration).map_err(|_| ProviderError::incompatible("窗口时长超出支持范围"))?;

    let reset_seconds = object
        .get("resetsAt")
        .and_then(Value::as_i64)
        .ok_or_else(|| ProviderError::incompatible("窗口 resetsAt 缺失或无效"))?;
    let resets_at = unix_seconds_to_rfc3339(reset_seconds)
        .ok_or_else(|| ProviderError::incompatible("窗口 resetsAt 超出支持范围"))?;
    let (kind, label) = classify_window_duration(window_duration_mins);

    Ok(QuotaWindow {
        slot,
        kind,
        label,
        used_percent,
        remaining_percent: 100.0 - used_percent,
        window_duration_mins,
        resets_at,
    })
}

fn parse_credit_summary(object: &Map<String, Value>) -> Result<CreditSummary, ProviderError> {
    let has_credits = optional_bool(object, "hasCredits")?;
    let unlimited = optional_bool(object, "unlimited")?;
    let balance = parse_optional_safe_string(
        object,
        "balance",
        MAX_BALANCE_CHARS,
        "额度 credits.balance 不符合约束",
    )?;

    Ok(CreditSummary {
        has_credits,
        unlimited,
        balance,
    })
}

fn parse_reset_credit_summary(value: &Value) -> Result<Option<ResetCreditSummary>, ProviderError> {
    let Some(object) = value.as_object() else {
        return if value.is_null() {
            Ok(None)
        } else {
            Err(ProviderError::incompatible(
                "rateLimitResetCredits 必须是对象或 null",
            ))
        };
    };

    let available_count = object
        .get("availableCount")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| ProviderError::incompatible("重置额度 availableCount 缺失或越界"))?;

    let details = match object.get("credits") {
        None | Some(Value::Null) => None,
        Some(Value::Array(items)) => {
            let mut details = Vec::with_capacity(items.len());
            for item in items {
                let detail_object = item
                    .as_object()
                    .ok_or_else(|| ProviderError::incompatible("重置额度明细必须是对象"))?;
                details.push(parse_reset_credit_detail(detail_object)?);
            }
            Some(details)
        }
        Some(_) => {
            return Err(ProviderError::incompatible(
                "重置额度 credits 必须是数组或 null",
            ))
        }
    };

    Ok(Some(ResetCreditSummary {
        available_count,
        details,
    }))
}

fn parse_reset_credit_detail(
    object: &Map<String, Value>,
) -> Result<ResetCreditDetail, ProviderError> {
    let reset_type = parse_required_safe_string(
        object,
        "resetType",
        MAX_EXTERNAL_ENUM_CHARS,
        "重置额度 resetType 缺失或无效",
    )?;
    let status = parse_required_safe_string(
        object,
        "status",
        MAX_EXTERNAL_ENUM_CHARS,
        "重置额度 status 缺失或无效",
    )?;
    let granted_at = parse_unix_timestamp(object, "grantedAt", "重置额度 grantedAt 无效")?;
    let expires_at = match object.get("expiresAt") {
        None | Some(Value::Null) => None,
        Some(value) => {
            let seconds = value
                .as_i64()
                .ok_or_else(|| ProviderError::incompatible("重置额度 expiresAt 无效"))?;
            Some(
                unix_seconds_to_rfc3339(seconds)
                    .ok_or_else(|| ProviderError::incompatible("重置额度 expiresAt 越界"))?,
            )
        }
    };
    let title = parse_optional_safe_string(
        object,
        "title",
        MAX_RESET_TEXT_CHARS,
        "重置额度 title 不符合约束",
    )?;
    let description = parse_optional_safe_string(
        object,
        "description",
        MAX_RESET_TEXT_CHARS,
        "重置额度 description 不符合约束",
    )?;

    // 服务端明细中的不透明 id 故意不读取，也不会进入领域模型。
    Ok(ResetCreditDetail {
        reset_type,
        status,
        granted_at,
        expires_at,
        title,
        description,
    })
}

fn parse_unix_timestamp(
    object: &Map<String, Value>,
    field: &str,
    detail: &'static str,
) -> Result<String, ProviderError> {
    let seconds = object
        .get(field)
        .and_then(Value::as_i64)
        .ok_or_else(|| ProviderError::incompatible(detail))?;

    unix_seconds_to_rfc3339(seconds).ok_or_else(|| ProviderError::incompatible(detail))
}

fn optional_bool(object: &Map<String, Value>, field: &str) -> Result<Option<bool>, ProviderError> {
    match object.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(value)) => Ok(Some(*value)),
        Some(_) => Err(ProviderError::incompatible("布尔字段类型与协议不一致")),
    }
}

fn optional_string_value(
    object: &Map<String, Value>,
    field: &str,
) -> Result<Option<String>, ProviderError> {
    match object.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(ProviderError::incompatible("字符串字段类型与协议不一致")),
    }
}

fn parse_required_safe_string(
    object: &Map<String, Value>,
    field: &str,
    max_chars: usize,
    detail: &'static str,
) -> Result<String, ProviderError> {
    let value = object
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| ProviderError::incompatible(detail))?;
    validate_required_external_string(value, max_chars, detail)
}

fn parse_optional_safe_string(
    object: &Map<String, Value>,
    field: &str,
    max_chars: usize,
    detail: &'static str,
) -> Result<Option<String>, ProviderError> {
    let value = optional_string_value(object, field)?;
    validate_optional_external_string(value.as_deref(), max_chars, detail)
}

fn validate_required_external_string(
    value: &str,
    max_chars: usize,
    detail: &'static str,
) -> Result<String, ProviderError> {
    let trimmed = value.trim();
    let length = trimmed.chars().count();
    if length == 0 || length > max_chars || trimmed.chars().any(char::is_control) {
        return Err(ProviderError::incompatible(detail));
    }

    Ok(trimmed.to_owned())
}

fn validate_optional_external_string(
    value: Option<&str>,
    max_chars: usize,
    detail: &'static str,
) -> Result<Option<String>, ProviderError> {
    value
        .map(|item| validate_required_external_string(item, max_chars, detail))
        .transpose()
}

fn classify_window_duration(duration_mins: u32) -> (WindowKind, String) {
    match duration_mins {
        300 => (WindowKind::ShortTerm, "五小时".to_owned()),
        10_080 => (WindowKind::Weekly, "一周".to_owned()),
        40_320 => (WindowKind::Monthly, "28 天".to_owned()),
        43_200 => (WindowKind::Monthly, "30 天".to_owned()),
        44_640 => (WindowKind::Monthly, "31 天".to_owned()),
        value if value % 1_440 == 0 => (WindowKind::Unknown, format!("{} 天", value / 1_440)),
        value if value % 60 == 0 => (WindowKind::Unknown, format!("{} 小时", value / 60)),
        value => (WindowKind::Unknown, format!("{value} 分钟")),
    }
}

fn sort_buckets(buckets: &mut [QuotaBucket]) {
    buckets.sort_by(|left, right| {
        match (
            left.limit_id.as_str() == "codex",
            right.limit_id.as_str() == "codex",
        ) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => left.limit_id.cmp(&right.limit_id),
        }
    });
}

/// 将合法 Unix 秒转换为 UTC RFC 3339，不引入本地时区语义。
fn unix_seconds_to_rfc3339(seconds: i64) -> Option<String> {
    if !(0..=MAX_UNIX_SECONDS).contains(&seconds) {
        return None;
    }

    let days = seconds.div_euclid(86_400);
    let seconds_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_date_from_unix_days(days);
    if !(1970..=9999).contains(&year) {
        return None;
    }

    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;

    Some(format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z"
    ))
}

/// Howard Hinnant 的公历换算公式；输入为自 Unix Epoch 起的完整天数。
fn civil_date_from_unix_days(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    if month <= 2 {
        year += 1;
    }

    (year, month, day)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::domain::{AuthUiState, WindowKind, WindowSlot};

    use super::{
        normalize_auth_mode, parse_account_read_result, parse_rate_limits_result,
        unix_seconds_to_rfc3339, ProviderErrorKind,
    };

    fn window(used_percent: i64, duration: u64, resets_at: i64) -> serde_json::Value {
        json!({
            "usedPercent": used_percent,
            "windowDurationMins": duration,
            "resetsAt": resets_at
        })
    }

    #[test]
    fn parses_backward_compatible_single_bucket_and_remaining_percent() {
        let result = json!({
            "rateLimits": {
                "limitId": "codex",
                "limitName": null,
                "primary": window(25, 300, 0),
                "secondary": window(18, 10_080, 86_400),
                "credits": {
                    "hasCredits": true,
                    "unlimited": false,
                    "balance": "12.50"
                },
                "planType": "pro",
                "rateLimitReachedType": null
            }
        });

        let parsed = parse_rate_limits_result(&result);
        let bucket = parsed.as_ref().ok().and_then(|data| data.buckets.first());

        assert_eq!(parsed.as_ref().ok().map(|data| data.buckets.len()), Some(1));
        assert_eq!(bucket.map(|item| item.limit_id.as_str()), Some("codex"));
        assert_eq!(
            bucket
                .and_then(|item| item.windows.first())
                .map(|item| item.slot),
            Some(WindowSlot::Primary)
        );
        assert_eq!(
            bucket
                .and_then(|item| item.windows.first())
                .map(|item| item.remaining_percent),
            Some(75.0)
        );
        assert_eq!(
            bucket
                .and_then(|item| item.windows.first())
                .map(|item| item.resets_at.as_str()),
            Some("1970-01-01T00:00:00Z")
        );
    }

    #[test]
    fn authoritative_multi_bucket_view_wins_over_single_bucket() {
        let result = json!({
            "rateLimits": {
                "limitId": "stale-single",
                "primary": window(101, 300, 0)
            },
            "rateLimitsByLimitId": {
                "zeta": {
                    "limitId": "zeta",
                    "primary": window(40, 60, 0)
                },
                "codex": {
                    "limitId": "codex",
                    "primary": window(20, 300, 0),
                    "rateLimitReachedType": "futureReachedKind"
                }
            }
        });

        let parsed = parse_rate_limits_result(&result);
        let ids = parsed.as_ref().ok().map(|data| {
            data.buckets
                .iter()
                .map(|bucket| bucket.limit_id.as_str())
                .collect::<Vec<_>>()
        });

        assert_eq!(ids, Some(vec!["codex", "zeta"]));
        assert_eq!(
            parsed
                .as_ref()
                .ok()
                .and_then(|data| data.buckets.first())
                .and_then(|bucket| bucket.rate_limit_reached_type.as_deref()),
            Some("futureReachedKind")
        );
    }

    #[test]
    fn empty_multi_bucket_view_does_not_fall_back_to_single_bucket() {
        let result = json!({
            "rateLimitsByLimitId": {},
            "rateLimits": {
                "limitId": "stale-single",
                "primary": window(20, 300, 0)
            }
        });

        let parsed = parse_rate_limits_result(&result);

        assert_eq!(parsed.ok().map(|data| data.buckets.len()), Some(0));
    }

    #[test]
    fn unknown_auth_mode_is_preserved_and_safely_downgraded() {
        let auth = normalize_auth_mode(Some("futureAuthMode"), Some("futurePlan"), Some(true));

        assert_eq!(
            auth.as_ref().ok().map(|item| item.state),
            Some(AuthUiState::Unknown)
        );
        assert_eq!(
            auth.as_ref()
                .ok()
                .and_then(|item| item.auth_mode.as_deref()),
            Some("futureAuthMode")
        );
        assert_eq!(
            auth.as_ref()
                .ok()
                .and_then(|item| item.plan_type.as_deref()),
            Some("futurePlan")
        );
    }

    #[test]
    fn known_account_aliases_are_normalized_and_email_is_discarded() {
        let api_key = parse_account_read_result(&json!({
            "account": {
                "type": "apiKey",
                "email": "must-not-survive@example.com",
                "planType": null
            },
            "requiresOpenaiAuth": true
        }));
        let bedrock = normalize_auth_mode(Some("amazonBedrock"), None, Some(false));

        assert_eq!(
            api_key.as_ref().ok().map(|item| item.state),
            Some(AuthUiState::ApiKeyMode)
        );
        assert!(api_key
            .as_ref()
            .ok()
            .and_then(|item| serde_json::to_string(item).ok())
            .is_some_and(|item| !item.contains("must-not-survive")));
        assert_eq!(
            bedrock.as_ref().ok().map(|item| item.state),
            Some(AuthUiState::ExternalProvider)
        );
        assert_eq!(
            bedrock
                .as_ref()
                .ok()
                .and_then(|item| item.auth_mode.as_deref()),
            Some("bedrockApiKey")
        );
    }

    #[test]
    fn missing_required_window_field_is_incompatible() {
        let result = json!({
            "rateLimits": {
                "limitId": "codex",
                "primary": {
                    "windowDurationMins": 300,
                    "resetsAt": 0
                }
            }
        });

        let error = parse_rate_limits_result(&result).err();

        assert_eq!(
            error.map(|item| item.kind),
            Some(ProviderErrorKind::ResponseIncompatible)
        );
    }

    #[test]
    fn invalid_percentages_never_become_zero_percent_windows() {
        for invalid in [-1, 101] {
            let result = json!({
                "rateLimits": {
                    "limitId": "codex",
                    "primary": window(invalid, 300, 0)
                }
            });

            assert_eq!(
                parse_rate_limits_result(&result)
                    .err()
                    .map(|item| item.kind),
                Some(ProviderErrorKind::ResponseIncompatible)
            );
        }
    }

    #[test]
    fn invalid_optional_window_is_isolated_when_another_window_is_valid() {
        let result = json!({
            "rateLimits": {
                "limitId": "codex",
                "primary": window(101, 300, 0),
                "secondary": window(10, 10_080, 0)
            }
        });

        let parsed = parse_rate_limits_result(&result);
        let windows = parsed
            .as_ref()
            .ok()
            .and_then(|data| data.buckets.first())
            .map(|bucket| bucket.windows.as_slice());

        assert_eq!(windows.map(<[_]>::len), Some(1));
        assert_eq!(
            windows
                .and_then(|items| items.first())
                .map(|item| item.kind),
            Some(WindowKind::Weekly)
        );
    }

    #[test]
    fn mismatched_multi_bucket_key_isolated_from_valid_buckets() {
        let result = json!({
            "rateLimitsByLimitId": {
                "wrong-key": {
                    "limitId": "other",
                    "primary": window(10, 300, 0)
                },
                "codex": {
                    "limitId": "codex",
                    "primary": window(20, 300, 0)
                }
            }
        });

        let parsed = parse_rate_limits_result(&result);

        assert_eq!(parsed.as_ref().ok().map(|data| data.buckets.len()), Some(1));
        assert_eq!(
            parsed
                .as_ref()
                .ok()
                .and_then(|data| data.buckets.first())
                .map(|bucket| bucket.limit_id.as_str()),
            Some("codex")
        );
    }

    #[test]
    fn reset_credit_id_does_not_enter_domain_model() {
        let result = json!({
            "rateLimits": {
                "limitId": "codex",
                "primary": window(20, 300, 0)
            },
            "rateLimitResetCredits": {
                "availableCount": 1,
                "credits": [{
                    "id": "opaque-secret-id",
                    "resetType": "codexRateLimits",
                    "status": "available",
                    "grantedAt": 0,
                    "expiresAt": null,
                    "title": null,
                    "description": null
                }]
            }
        });

        let serialized = parse_rate_limits_result(&result)
            .ok()
            .and_then(|data| serde_json::to_string(&data.banked_resets).ok());

        assert!(serialized.is_some_and(|value| !value.contains("opaque-secret-id")));
    }

    #[test]
    fn unix_seconds_are_converted_without_local_timezone() {
        assert_eq!(
            unix_seconds_to_rfc3339(86_400).as_deref(),
            Some("1970-01-02T00:00:00Z")
        );
        assert_eq!(
            unix_seconds_to_rfc3339(1_783_900_800).as_deref(),
            Some("2026-07-13T00:00:00Z")
        );
        assert_eq!(unix_seconds_to_rfc3339(-1), None);
    }
}
