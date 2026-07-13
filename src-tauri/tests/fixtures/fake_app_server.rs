use std::{
    env,
    io::{self, BufRead, Write},
    thread,
    time::Duration,
};

use serde_json::{json, Value};

const MAX_MESSAGE_BYTES: usize = 1024 * 1024;

fn main() {
    if let Err(error) = run() {
        eprintln!("假 App Server 执行失败：{error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut arguments = env::args().skip(1);
    require_argument(arguments.next().as_deref(), "app-server", "启动子命令")?;
    require_argument(arguments.next().as_deref(), "--scenario", "场景参数")?;
    let scenario = arguments
        .next()
        .ok_or_else(|| "缺少假服务场景名称".to_owned())?;

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();

    match scenario.as_str() {
        "success" => run_success(&mut reader, &mut writer),
        "timeout" => run_timeout(&mut reader),
        "exit" => run_early_exit(&mut reader),
        "oversized" => run_oversized(&mut reader, &mut writer),
        "remote-error" => run_remote_error(&mut reader, &mut writer),
        "malformed" => run_malformed(&mut reader, &mut writer),
        "persistent-concurrent" => run_persistent_concurrent(&mut reader, &mut writer),
        "persistent-late-response" => run_persistent_late_response(&mut reader, &mut writer),
        "persistent-exit" => run_persistent_exit(&mut reader, &mut writer),
        _ => Err("收到未知假服务场景".to_owned()),
    }
}

fn run_success<R, W>(reader: &mut R, writer: &mut W) -> Result<(), String>
where
    R: BufRead,
    W: Write,
{
    let initialize = read_json_line(reader)?;
    require_message(&initialize, "initialize", Some(1))?;
    if initialize
        .pointer("/params/clientInfo/name")
        .and_then(Value::as_str)
        != Some("quota_glance")
        || initialize.to_string().contains("experimentalApi")
    {
        return Err("初始化参数不符合只读客户端约束".to_owned());
    }
    write_json_line(writer, &json!({"id": 1, "result": {}}))?;

    let initialized = read_json_line(reader)?;
    require_message(&initialized, "initialized", None)?;

    let account_read = read_json_line(reader)?;
    require_message(&account_read, "account/read", Some(2))?;
    if account_read.pointer("/params/refreshToken") != Some(&Value::Bool(false)) {
        return Err("账户读取不得刷新凭据".to_owned());
    }
    write_json_line(writer, &json!({"id": 700, "result": {"ignored": true}}))?;
    write_json_line(
        writer,
        &json!({
            "id": 2,
            "result": {
                "account": {
                    "type": "chatgpt",
                    "planType": "pro",
                    "email": "must-not-enter-domain@example.invalid"
                },
                "requiresOpenaiAuth": true
            }
        }),
    )?;

    let rate_limits_read = read_json_line(reader)?;
    require_message(&rate_limits_read, "account/rateLimits/read", Some(3))?;
    write_json_line(
        writer,
        &json!({
            "method": "account/rateLimits/updated",
            "params": {"partialPayloadMustNotBeMerged": true}
        }),
    )?;
    write_json_line(
        writer,
        &json!({
            "method": "future/unknown/notification",
            "params": {"mustBeIgnored": true}
        }),
    )?;
    write_json_line(
        writer,
        &json!({
            "id": 91,
            "method": "future/write/request",
            "params": {"secret": "must-not-be-echoed"}
        }),
    )?;

    let unsupported_response = read_json_line(reader)?;
    if unsupported_response.get("id").and_then(Value::as_u64) != Some(91)
        || unsupported_response
            .pointer("/error/code")
            .and_then(Value::as_i64)
            != Some(-32601)
        || unsupported_response
            .to_string()
            .contains("must-not-be-echoed")
    {
        return Err("客户端没有安全拒绝未知服务端请求".to_owned());
    }

    write_json_line(
        writer,
        &json!({
            "id": 3,
            "result": {
                "rateLimitsByLimitId": {
                    "codex": {
                        "limitId": "codex",
                        "limitName": "Codex",
                        "planType": "pro",
                        "primary": {
                            "usedPercent": 26,
                            "windowDurationMins": 300,
                            "resetsAt": 1_893_456_000_i64
                        },
                        "secondary": {
                            "usedPercent": 58,
                            "windowDurationMins": 10_080,
                            "resetsAt": 1_893_888_000_i64
                        },
                        "rateLimitReachedType": null
                    }
                }
            }
        }),
    )
}

fn run_timeout<R>(reader: &mut R) -> Result<(), String>
where
    R: BufRead,
{
    let initialize = read_json_line(reader)?;
    require_message(&initialize, "initialize", Some(1))?;
    thread::sleep(Duration::from_secs(5));
    Ok(())
}

fn run_early_exit<R>(reader: &mut R) -> Result<(), String>
where
    R: BufRead,
{
    let initialize = read_json_line(reader)?;
    require_message(&initialize, "initialize", Some(1))?;
    Err("按测试场景在握手响应前退出".to_owned())
}

fn run_oversized<R, W>(reader: &mut R, writer: &mut W) -> Result<(), String>
where
    R: BufRead,
    W: Write,
{
    let initialize = read_json_line(reader)?;
    require_message(&initialize, "initialize", Some(1))?;

    let payload = vec![b'a'; MAX_MESSAGE_BYTES + 1];
    writer
        .write_all(&payload)
        .and_then(|_| writer.write_all(b"\n"))
        .and_then(|_| writer.flush())
        .map_err(|error| format!("写入超长消息失败：{error}"))
}

fn run_remote_error<R, W>(reader: &mut R, writer: &mut W) -> Result<(), String>
where
    R: BufRead,
    W: Write,
{
    let initialize = read_json_line(reader)?;
    require_message(&initialize, "initialize", Some(1))?;
    write_json_line(
        writer,
        &json!({
            "id": 1,
            "error": {
                "code": -32_000,
                "message": "raw-remote-detail-must-not-escape"
            }
        }),
    )
}

fn run_malformed<R, W>(reader: &mut R, writer: &mut W) -> Result<(), String>
where
    R: BufRead,
    W: Write,
{
    let initialize = read_json_line(reader)?;
    require_message(&initialize, "initialize", Some(1))?;
    writer
        .write_all(b"{not-json}\n")
        .and_then(|_| writer.flush())
        .map_err(|error| format!("写入畸形消息失败：{error}"))
}

fn run_persistent_concurrent<R, W>(reader: &mut R, writer: &mut W) -> Result<(), String>
where
    R: BufRead,
    W: Write,
{
    perform_initialize(reader, writer)?;

    let first_account = read_json_line(reader)?;
    let second_account = read_json_line(reader)?;
    require_method(&first_account, "account/read")?;
    require_method(&second_account, "account/read")?;
    let first_account_id = message_id(&first_account)?;
    let second_account_id = message_id(&second_account)?;
    write_json_line(writer, &account_result(second_account_id))?;
    write_json_line(writer, &account_result(first_account_id))?;

    let first_rate_limits = read_json_line(reader)?;
    let second_rate_limits = read_json_line(reader)?;
    require_method(&first_rate_limits, "account/rateLimits/read")?;
    require_method(&second_rate_limits, "account/rateLimits/read")?;
    let first_rate_limits_id = message_id(&first_rate_limits)?;
    let second_rate_limits_id = message_id(&second_rate_limits)?;

    write_json_line(writer, &json!({"method": "account/updated", "params": {}}))?;
    write_json_line(
        writer,
        &json!({"method": "future/unknown/notification", "params": {}}),
    )?;
    write_json_line(writer, &rate_limits_result(second_rate_limits_id))?;
    write_json_line(writer, &rate_limits_result(first_rate_limits_id))?;

    wait_for_client_close(reader)
}

fn run_persistent_late_response<R, W>(reader: &mut R, writer: &mut W) -> Result<(), String>
where
    R: BufRead,
    W: Write,
{
    perform_initialize(reader, writer)?;

    let timed_out = read_json_line(reader)?;
    require_method(&timed_out, "account/read")?;
    let timed_out_id = message_id(&timed_out)?;
    thread::sleep(Duration::from_millis(250));
    write_json_line(writer, &account_result(timed_out_id))?;

    let account = read_json_line(reader)?;
    require_method(&account, "account/read")?;
    write_json_line(writer, &account_result(message_id(&account)?))?;

    let rate_limits = read_json_line(reader)?;
    require_method(&rate_limits, "account/rateLimits/read")?;
    write_json_line(writer, &rate_limits_result(message_id(&rate_limits)?))?;

    wait_for_client_close(reader)
}

fn run_persistent_exit<R, W>(reader: &mut R, writer: &mut W) -> Result<(), String>
where
    R: BufRead,
    W: Write,
{
    perform_initialize(reader, writer)?;
    let first = read_json_line(reader)?;
    let second = read_json_line(reader)?;
    require_method(&first, "account/read")?;
    require_method(&second, "account/read")?;
    Err("按测试场景在存在待处理请求时退出".to_owned())
}

fn perform_initialize<R, W>(reader: &mut R, writer: &mut W) -> Result<(), String>
where
    R: BufRead,
    W: Write,
{
    let initialize = read_json_line(reader)?;
    require_message(&initialize, "initialize", Some(1))?;
    write_json_line(writer, &json!({"id": 1, "result": {}}))?;
    let initialized = read_json_line(reader)?;
    require_message(&initialized, "initialized", None)
}

fn require_method(value: &Value, method: &str) -> Result<(), String> {
    if value.get("method").and_then(Value::as_str) == Some(method) {
        Ok(())
    } else {
        Err(format!("期望收到 {method} 请求"))
    }
}

fn message_id(value: &Value) -> Result<u64, String> {
    value
        .get("id")
        .and_then(Value::as_u64)
        .ok_or_else(|| "请求缺少有效 ID".to_owned())
}

fn account_result(id: u64) -> Value {
    json!({
        "id": id,
        "result": {
            "account": {"type": "chatgpt", "planType": "pro"},
            "requiresOpenaiAuth": true
        }
    })
}

fn rate_limits_result(id: u64) -> Value {
    json!({
        "id": id,
        "result": {
            "rateLimitsByLimitId": {
                "codex": {
                    "limitId": "codex",
                    "limitName": "Codex",
                    "planType": "pro",
                    "primary": {
                        "usedPercent": 26,
                        "windowDurationMins": 300,
                        "resetsAt": 1_893_456_000_i64
                    },
                    "rateLimitReachedType": null
                }
            }
        }
    })
}

fn wait_for_client_close<R>(reader: &mut R) -> Result<(), String>
where
    R: BufRead,
{
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map(|_| ())
        .map_err(|error| format!("等待客户端关闭失败：{error}"))
}

fn require_argument(actual: Option<&str>, expected: &str, name: &str) -> Result<(), String> {
    if actual == Some(expected) {
        Ok(())
    } else {
        Err(format!("{name}不正确"))
    }
}

fn require_message(value: &Value, method: &str, id: Option<u64>) -> Result<(), String> {
    if value.get("method").and_then(Value::as_str) != Some(method) {
        return Err(format!("期望收到 {method} 消息"));
    }

    match id {
        Some(expected) if value.get("id").and_then(Value::as_u64) != Some(expected) => {
            Err(format!("{method} 的请求 ID 不正确"))
        }
        None if value.get("id").is_some() => Err(format!("{method} 通知不应携带请求 ID")),
        _ => Ok(()),
    }
}

fn read_json_line<R>(reader: &mut R) -> Result<Value, String>
where
    R: BufRead,
{
    let mut line = String::new();
    let read = reader
        .read_line(&mut line)
        .map_err(|error| format!("读取客户端消息失败：{error}"))?;
    if read == 0 {
        return Err("客户端提前关闭标准输入".to_owned());
    }

    serde_json::from_str(&line).map_err(|error| format!("客户端消息不是有效 JSON：{error}"))
}

fn write_json_line<W>(writer: &mut W, value: &Value) -> Result<(), String>
where
    W: Write,
{
    serde_json::to_writer(&mut *writer, value)
        .map_err(|error| format!("序列化假服务响应失败：{error}"))?;
    writer
        .write_all(b"\n")
        .and_then(|_| writer.flush())
        .map_err(|error| format!("写入假服务响应失败：{error}"))
}
