# QuotaGlance 接口设计说明书

> 文档状态：目标接口基线；`0.1.0` 已实现首批 IPC 与事件  
> 目标版本：`1.0.0`  
> IPC Schema：`1`  
> 最后更新：2026-07-13  
> 维护联系：`maorongkang@gmail.com`

## 1. 文档范围

本文定义 QuotaGlance 1.0.0 的两层接口：

1. React WebView 与 Rust Core 之间的 Tauri Commands 和 Events。
2. Rust Core 与 Codex App Server 之间的只读 JSONL 协议子集及字段映射。

本文同时记录 1.0.0 目标契约与 0.1.0 已实现子集。只有明确标记为“已实现”的命令和事件可以按当前代码调用；其余内容仍是后续实现目标。Rust 与 TypeScript 类型仍需在 CI 中持续校验，避免模型漂移。

### 1.1 0.1.0 当前实现边界

- 已实现动态 Rust 额度领域模型、严格 JSONL 校验、只读常驻 Codex App Server 会话与 pending request map，并具备基础刷新状态和 React UI。
- 已通过仅测试特性构建的假 App Server 执行跨进程契约，验证单次握手、乱序响应匹配、通知转发、迟到响应忽略、未知请求拒绝、错误脱敏、畸形输出、超时回收、异常退出和消息上限。
- 当前已实现 9 个 IPC：`get_quota_snapshot`、`refresh_quota`、`get_app_server_status`、`get_preferences`、`set_theme`、`set_widget_mode`、`set_always_on_top`、`set_click_through`、`quit_app`。
- 当前已发出 6 类状态事件：`quota://snapshot-updated`、`quota://refresh-state-changed`、`quota://auth-state-changed`、`quota://app-server-state-changed`、`preferences://changed`、`window://state-changed`。React UI 当前消费其中的快照、刷新、偏好和窗口事件。
- 窗口与托盘功能已接入；窗口模式、置顶和穿透偏好已写入应用配置目录并支持备份恢复。
- 常驻 App Server、通知驱动完整重读、30 秒自动刷新缓存、SingleFlight、最后成功快照、可见/隐藏重同步、退避和主题偏好保存已经实现；语言、开机启动、更新器以及 sidecar 生产分发尚未完成。

## 2. 接口原则

- 前端只调用 `src/api/` 中的封装，不在组件内直接调用 `invoke` 或 `listen`。
- IPC 使用 camelCase JSON；Rust 字段通过 Serde 显式配置，不依赖默认命名。
- Commands 细粒度授权，不提供任意文件、Shell、URL、App Server 方法或命令执行接口。
- Events 用于状态同步，不保证重放；页面重载后必须通过 `get_*` 命令取得当前状态。
- 所有时间对前端使用 UTC RFC 3339 字符串，所有持续时间使用毫秒并在名称中带 `Ms`。
- 可选值统一使用 `null`，不依赖 `undefined` 穿越 IPC。
- 未知数据保持未知；错误或缺失额度不得转换为 `0%`。
- 错误返回稳定 `code` 和 `messageKey`，不返回原始 App Server 消息、路径或账号信息。
- 1.0.0 的 App Server 集成严格只读，并且不启用 `experimentalApi`。

## 3. 公共 IPC 类型

以下 TypeScript 是前端消费契约。实现时建议由 Rust Schema 生成或在 CI 中双向校验。

```ts
export type IsoTimestamp = string;

export type QuotaStatus =
  | "loading"
  | "ok"
  | "stale"
  | "signedOut"
  | "apiKeyMode"
  | "quotaReached"
  | "sourceBusy"
  | "offline"
  | "serviceUnavailable"
  | "appServerUnavailable"
  | "incompatible";

export type AuthUiState =
  | "signedOut"
  | "authenticated"
  | "apiKeyMode"
  | "externalProvider"
  | "unknown";

export interface AuthSummary {
  state: AuthUiState;
  // 已知值使用官方 authMode；未来值经校验后原样保留。
  authMode: string | null;
  planType: string | null;
  requiresOpenaiAuth: boolean | null;
}

export type QuotaSource = "appServer" | "legacyCompat";
export type WindowSlot = "primary" | "secondary" | "other";
export type WindowKind = "shortTerm" | "weekly" | "monthly" | "unknown";

export interface QuotaWindow {
  slot: WindowSlot;
  kind: WindowKind;
  label: string;
  usedPercent: number;
  remainingPercent: number;
  windowDurationMins: number;
  resetsAt: IsoTimestamp;
}

export interface CreditSummary {
  hasCredits: boolean | null;
  unlimited: boolean | null;
  // 保留服务端字符串，不自行补货币符号或进行浮点计算。
  balance: string | null;
}

export interface QuotaBucket {
  limitId: string;
  limitName: string | null;
  planType: string | null;
  windows: QuotaWindow[];
  credits: CreditSummary | null;
  rateLimitReachedType: string | null;
}

export interface ResetCreditDetail {
  resetType: string;
  status: string;
  grantedAt: IsoTimestamp;
  expiresAt: IsoTimestamp | null;
  title: string | null;
  description: string | null;
}

export interface ResetCreditSummary {
  // 官方说明该数量为权威值，明细可能被截断。
  availableCount: number;
  // null 表示只取得数量；[] 表示已取明细但没有可用项。
  details: ResetCreditDetail[] | null;
}

export interface QuotaError {
  code: ErrorCode;
  messageKey: string;
  retryable: boolean;
  retryAfterMs: number | null;
}

export interface QuotaSnapshot {
  schemaVersion: 1;
  revision: number;
  source: QuotaSource | null;
  provider: "codexAppServer" | "legacyWham" | null;
  auth: AuthSummary;
  buckets: QuotaBucket[];
  bankedResets: ResetCreditSummary | null;
  status: QuotaStatus;
  fetchedAt: IsoTimestamp | null;
  lastGoodAt: IsoTimestamp | null;
  nextRetryAt: IsoTimestamp | null;
  error: QuotaError | null;
}
```

### 3.1 Credits 兼容说明

官方 App Server 文档确认额度桶可带 `credits`，但具体字段仍须以固定 sidecar 生成的 JSON Schema 为准。1.0.0 内部契约只接受并暴露已验证的 `hasCredits`、`unlimited` 和字符串 `balance`；字段缺失时返回 `null`，新增外部字段默认忽略。若固定版本 Schema 与该结构不同，必须先更新本文、类型和契约测试，不能用无约束 `any` 透传。

### 3.2 App Server 状态

```ts
export type AppServerPhase =
  | "stopped"
  | "locating"
  | "starting"
  | "initializing"
  | "ready"
  | "backingOff"
  | "failed"
  | "incompatible"
  | "shuttingDown";

export interface AppServerStatus {
  revision: number;
  phase: AppServerPhase;
  source: "bundled" | "external" | "developmentPath" | null;
  version: string | null;
  targetTriple: string | null;
  restartAttempt: number;
  nextRetryAt: IsoTimestamp | null;
  error: QuotaError | null;
}
```

`version` 来自受控 sidecar 清单或已校验的 CLI 版本，不从原始 stderr 猜测。接口不返回可执行文件绝对路径。

### 3.3 刷新状态

```ts
export type RefreshReason =
  | "startup"
  | "manual"
  | "accountNotification"
  | "quotaNotification"
  | "visibleResync"
  | "hiddenResync"
  | "resume";

export type RefreshPhase =
  | "idle"
  | "refreshing"
  | "cooldown"
  | "backingOff";

export interface RefreshState {
  revision: number;
  phase: RefreshPhase;
  reason: RefreshReason | null;
  startedAt: IsoTimestamp | null;
  nextAllowedManualRefreshAt: IsoTimestamp | null;
  nextRetryAt: IsoTimestamp | null;
}

export interface RefreshReceipt {
  accepted: boolean;
  joinedExistingRequest: boolean;
  requestRevision: number;
  state: RefreshState;
}
```

### 3.4 偏好类型

```ts
export type Locale = "system" | "zh-CN" | "en";
export type Theme =
  | "system"
  | "aurora"
  | "graphite"
  | "paper"
  | "sunset"
  | "honey"
  | "rose";
export type WidgetMode = "orb" | "card" | "hidden";

export interface WindowBounds {
  x: number;
  y: number;
  width: number;
  height: number;
  monitorId: string | null;
  scaleFactorAtSave: number;
}

export interface Preferences {
  schemaVersion: 1;
  revision: number;
  locale: Locale;
  theme: Theme;
  widget: {
    mode: WidgetMode;
    alwaysOnTop: boolean;
    clickThrough: boolean;
    selectedQuota: {
      limitId: string | null;
      slot: "primary" | "secondary" | null;
    };
    boundsByMode: {
      orb: WindowBounds | null;
      card: WindowBounds | null;
    };
  };
  notifications: {
    enabled: boolean;
    warningRemainingPercent: number;
    criticalRemainingPercent: number;
    notifyWhenRecovered: boolean;
  };
  startup: {
    launchAtLogin: boolean;
  };
  updates: {
    autoCheck: boolean;
    channel: "stable";
    lastCheckedAt: IsoTimestamp | null;
  };
}

export interface PreferencesPatch {
  locale?: Locale;
  theme?: Theme;
  widget?: {
    mode?: WidgetMode;
    alwaysOnTop?: boolean;
    clickThrough?: boolean;
    selectedQuota?: {
      limitId: string | null;
      slot: "primary" | "secondary" | null;
    };
  };
  notifications?: Partial<Preferences["notifications"]>;
  startup?: Partial<Preferences["startup"]>;
  updates?: Pick<Partial<Preferences["updates"]>, "autoCheck" | "channel">;
}

export interface PreferencesEnvelope {
  preferences: Preferences;
  recovery: null | {
    source: "backup" | "defaults";
    reasonCode: "PREFERENCES_CORRUPTED" | "PREFERENCES_VERSION_UNSUPPORTED";
  };
}
```

前端不能通过 `save_preferences` 修改 `schemaVersion`、`revision`、窗口边界或 `lastCheckedAt`。窗口边界由后端平台事件受控保存，最后检查时间由更新服务保存。

### 3.5 窗口与更新类型

```ts
export interface WindowState {
  revision: number;
  mode: WidgetMode;
  visible: boolean;
  alwaysOnTop: boolean;
  clickThrough: boolean;
  bounds: WindowBounds | null;
}

export type UpdatePhase =
  | "idle"
  | "checking"
  | "upToDate"
  | "available"
  | "downloading"
  | "readyToInstall"
  | "installing"
  | "failed";

export interface UpdateStatus {
  revision: number;
  phase: UpdatePhase;
  currentVersion: string;
  availableVersion: string | null;
  checkedAt: IsoTimestamp | null;
  error: QuotaError | null;
}
```

## 4. 统一错误格式

Tauri Command 失败时返回以下可序列化对象：

```ts
export interface IpcError {
  code: ErrorCode;
  messageKey: string;
  retryable: boolean;
  retryAfterMs: number | null;
  context: {
    field: string | null;
    currentRevision: number | null;
    appServerPhase: AppServerPhase | null;
  };
}
```

`messageKey` 由前端中英文资源转换为用户文案。后端不得把原始外部错误拼进 `messageKey` 或 `context`。

### 4.1 错误码

```ts
export type ErrorCode =
  | "INVALID_ARGUMENT"
  | "FORBIDDEN"
  | "NOT_READY"
  | "SHUTTING_DOWN"
  | "APP_SERVER_NOT_FOUND"
  | "APP_SERVER_EXECUTION_DENIED"
  | "APP_SERVER_VERSION_INCOMPATIBLE"
  | "APP_SERVER_HANDSHAKE_TIMEOUT"
  | "APP_SERVER_EXITED"
  | "PROTOCOL_INVALID_MESSAGE"
  | "PROTOCOL_MESSAGE_TOO_LARGE"
  | "PROTOCOL_REQUEST_TIMEOUT"
  | "AUTH_REQUIRED"
  | "API_KEY_MODE"
  | "RATE_LIMITS_UNAVAILABLE"
  | "RESPONSE_INCOMPATIBLE"
  | "SOURCE_BUSY"
  | "OFFLINE"
  | "SERVICE_UNAVAILABLE"
  | "REFRESH_COOLDOWN"
  | "PREFERENCES_CONFLICT"
  | "PREFERENCES_CORRUPTED"
  | "PREFERENCES_VERSION_UNSUPPORTED"
  | "PREFERENCES_WRITE_FAILED"
  | "WINDOW_OPERATION_FAILED"
  | "UPDATE_CHECK_FAILED"
  | "UPDATE_SIGNATURE_INVALID"
  | "UPDATE_INSTALL_FAILED";
```

| 错误码 | 可重试 | 使用场景 |
|---|---:|---|
| `INVALID_ARGUMENT` | 否 | 输入缺失、枚举错误、数值越界、未知 Patch 字段 |
| `FORBIDDEN` | 否 | 当前窗口没有命令权限 |
| `NOT_READY` | 是 | 服务尚在启动且该命令不能立即完成 |
| `SHUTTING_DOWN` | 否 | 应用正在退出，不再受理写入或刷新 |
| `APP_SERVER_NOT_FOUND` | 是 | 无随包 sidecar，且没有有效外部 CLI |
| `APP_SERVER_EXECUTION_DENIED` | 是 | 文件不可执行、被系统策略拒绝或架构不匹配 |
| `APP_SERVER_VERSION_INCOMPATIBLE` | 否 | 版本不在应用兼容范围 |
| `APP_SERVER_HANDSHAKE_TIMEOUT` | 是 | 初始化握手超时 |
| `APP_SERVER_EXITED` | 是 | 在途请求期间子进程退出 |
| `PROTOCOL_INVALID_MESSAGE` | 是 | 畸形 JSON 或不符合消息结构 |
| `PROTOCOL_MESSAGE_TOO_LARGE` | 否 | 单条消息超过安全上限 |
| `PROTOCOL_REQUEST_TIMEOUT` | 是 | App Server 请求超过本地截止时间 |
| `AUTH_REQUIRED` | 否 | 确认未登录或登录失效 |
| `API_KEY_MODE` | 否 | 当前为 API 按量模式，不适用订阅额度 |
| `RATE_LIMITS_UNAVAILABLE` | 是 | 认证有效但没有可用额度结果 |
| `RESPONSE_INCOMPATIBLE` | 否 | 当前应用无法安全解析固定响应 |
| `SOURCE_BUSY` | 是 | 本地队列或 App Server 暂时繁忙 |
| `OFFLINE` | 是 | 已能可靠判断本机离线或代理不可达 |
| `SERVICE_UNAVAILABLE` | 是 | 无法细分的上游临时故障 |
| `REFRESH_COOLDOWN` | 是 | 30 秒手动刷新冷却，必须提供 `retryAfterMs` |
| `PREFERENCES_CONFLICT` | 是 | `expectedRevision` 落后，返回当前 revision |
| `PREFERENCES_CORRUPTED` | 否 | 当前偏好不可读，已使用备份或默认值 |
| `PREFERENCES_VERSION_UNSUPPORTED` | 否 | 文件来自未来 Schema，旧应用不覆盖 |
| `PREFERENCES_WRITE_FAILED` | 是 | 临时写、同步、备份或原子替换失败 |
| `WINDOW_OPERATION_FAILED` | 是 | 操作系统窗口 API 调用失败 |
| `UPDATE_*` | 视情况 | 更新检查、签名或安装失败 |

刷新失败但有最后成功快照时，命令可以成功返回收据，业务错误放入最新 `QuotaSnapshot.error`，快照状态为 `stale`。只有命令本身未被受理时才返回 `IpcError`。

## 5. Tauri Commands

### 5.1 命令总览

状态列以当前 `0.1.0` 代码为准；“目标契约”表示保留在 1.0.0 设计中、当前尚不可调用或尚未具备所述完整语义。

| Command | 输入 | 输出 | 副作用 | 0.1.0 状态 |
|---|---|---|---|---|
| `get_quota_snapshot` | 无 | `QuotaSnapshot` | 无；首次调用可确保后台启动流程已安排 | 已实现 |
| `refresh_quota` | 无 | `RefreshReceipt` | 受理一次手动刷新；完整 SingleFlight/退避仍待实现 | 已实现（基础语义） |
| `get_app_server_status` | 无 | `AppServerStatus` | 无 | 已实现 |
| `get_preferences` | 无 | `PreferencesEnvelope` | 读取已加载并经过损坏恢复的本地偏好 | 已实现 |
| `set_theme` | theme | `PreferencesEnvelope` | 原子保存主题并广播 `preferences://changed` | 已实现 |
| `save_preferences` | revision + Patch | `PreferencesEnvelope` | 原子保存偏好，必要时同步平台设置 | 目标契约 |
| `set_widget_mode` | mode | `WindowState` | 显示、展开或隐藏窗口，并原子保存偏好 | 已实现 |
| `set_always_on_top` | enabled | `WindowState` | 修改平台窗口，并原子保存偏好 | 已实现 |
| `set_click_through` | enabled | `WindowState` | 修改鼠标穿透，并原子保存偏好 | 已实现 |
| `quit_app` | 无 | `void` | 结束 QuotaGlance，应用生命周期钩子负责回收 App Server 子进程 | 已实现 |
| `check_for_updates` | 无 | `UpdateStatus` | 执行一次签名更新检查 | 目标契约 |
| `install_update` | expectedVersion | `UpdateStatus` | 下载/安装已验证更新，可能重启应用 | 目标契约 |

### 5.2 `get_quota_snapshot`

调用：

```ts
invoke<QuotaSnapshot>("get_quota_snapshot");
```

输入：无。不要传空对象以外的业务参数。

输出：当前内存快照。命令不会等待网络；首次启动可以返回：

```json
{
  "schemaVersion": 1,
  "revision": 0,
  "source": null,
  "provider": null,
  "auth": {
    "state": "unknown",
    "authMode": null,
    "planType": null,
    "requiresOpenaiAuth": null
  },
  "buckets": [],
  "bankedResets": null,
  "status": "loading",
  "fetchedAt": null,
  "lastGoodAt": null,
  "nextRetryAt": null,
  "error": null
}
```

### 5.3 `refresh_quota`

调用：

```ts
invoke<RefreshReceipt>("refresh_quota");
```

输入：无。刷新原因固定为 `manual`，前端不能伪造系统触发原因。

行为：

- 绕过 30 秒缓存 TTL。
- 受 30 秒手动刷新冷却限制。
- 已有完整刷新在途时加入 SingleFlight，并设置 `joinedExistingRequest=true`。
- 只表示刷新被受理，不保证 Provider 已成功；最终结果通过快照事件或重新读取获得。

冷却中返回 `REFRESH_COOLDOWN`，并在 `retryAfterMs` 给出剩余时间。

### 5.4 `get_app_server_status`

调用：

```ts
invoke<AppServerStatus>("get_app_server_status");
```

输入：无。输出不包含 sidecar 绝对路径、stderr、环境变量或原始协议错误。

### 5.5 `get_preferences`

调用：

```ts
invoke<PreferencesEnvelope>("get_preferences");
```

输入：无。偏好损坏但已安全恢复时命令仍返回可用偏好，并通过 `recovery` 告知来源。

### 5.6 `set_theme`

调用：

```ts
invoke<PreferencesEnvelope>("set_theme", {
  theme: "system" | "aurora" | "graphite" | "paper" | "sunset" | "honey" | "rose",
});
```

输入只能是 `system`、`aurora`、`graphite` 或 `paper`。读取历史偏好时，旧值 `light`、`dark` 分别按 `aurora`、`graphite` 兼容。命令在偏好文件原子写入成功后更新内存状态、递增 revision，并向 `widget` 窗口广播 `preferences://changed`；写入失败时保留旧偏好并返回受控错误。

### 5.7 `save_preferences`

输入：

```ts
interface SavePreferencesRequest {
  expectedRevision: number;
  patch: PreferencesPatch;
}
```

示例：

```json
{
  "expectedRevision": 4,
  "patch": {
    "theme": "rose",
    "notifications": {
      "warningRemainingPercent": 25,
      "criticalRemainingPercent": 10
    }
  }
}
```

Patch 是受控深层合并对象，不是 RFC 6902 JSON Patch。未知字段一律返回 `INVALID_ARGUMENT`。保存成功后返回新的完整偏好，revision 增加 1。

修改 `launchAtLogin` 时先执行平台设置，成功后再落盘；平台失败则偏好不变。与窗口直接相关的字段建议使用对应 `set_*` 命令，避免平台状态与文件状态分离。

### 5.8 `set_widget_mode`

输入：

```ts
interface SetWidgetModeRequest {
  mode: WidgetMode;
}
```

行为：切换窗口模式，执行可见区域校验，平台操作成功后保存偏好。`hidden` 只隐藏窗口，不退出应用，也不覆盖上次 `orb`/`card` 边界。

### 5.9 `set_always_on_top`

输入：

```ts
interface SetAlwaysOnTopRequest {
  enabled: boolean;
}
```

平台调用成功后才写入偏好。该设置与鼠标穿透互不隐含。

### 5.10 `set_click_through`

输入：

```ts
interface SetClickThroughRequest {
  enabled: boolean;
}
```

启用后，托盘必须保留后端直接调用的“解除穿透并显示”恢复入口。设置失败时返回旧 `WindowState` 对应的错误，不写入偏好。

### 5.11 `quit_app`

调用：

```ts
invoke<void>("quit_app");
```

输入：无。该命令只用于浮球右键菜单中的明确“退出”动作，不处理账号登出，也不修改 Codex 登录态。应用退出时由既有生命周期清理逻辑关闭常驻 App Server 会话并回收子进程。

### 5.12 `check_for_updates`

调用：

```ts
invoke<UpdateStatus>("check_for_updates");
```

只使用固定 HTTPS 更新源和 Tauri updater 签名校验。前端不能传更新 URL、通道或公钥。自动检查由后端调度，不通过伪造 IPC 参数触发。

### 5.13 `install_update`

输入：

```ts
interface InstallUpdateRequest {
  expectedVersion: string;
}
```

仅安装当前后端已检查并缓存元数据的同版本更新，防止页面使用过期状态。签名不合法返回 `UPDATE_SIGNATURE_INVALID`，不提供跳过验证选项。

## 6. Tauri Events

### 6.1 事件总览

| Event | Payload | 触发时机 | 0.1.0 状态 |
|---|---|---|---|
| `quota://snapshot-updated` | `QuotaSnapshot` | 内存快照替换后 | 已实现 |
| `quota://refresh-state-changed` | `RefreshState` | 基础刷新开始或结束 | 已实现（基础语义） |
| `quota://auth-state-changed` | `AuthSummary` | 完整账户重读并归一化后 | 已实现 |
| `quota://app-server-state-changed` | `AppServerStatus` | App Server 状态 revision 变化后 | 已实现 |
| `preferences://changed` | `PreferencesEnvelope` | 窗口偏好内存状态变化后 | 已实现（内存态） |
| `window://state-changed` | `WindowState` | 平台窗口操作成功后 | 已实现 |
| `app://update-available` | `UpdateStatus` | 发现已通过元数据校验的新版本 | 目标契约 |
| `app://update-state-changed` | `UpdateStatus` | 更新阶段变化 | 目标契约 |

### 6.2 顺序与幂等

- 每类状态都有单调递增的 `revision`；前端忽略比当前 revision 更旧的事件。
- 后端先提交状态，再发送事件。事件监听者立即调用 `get_*` 时不会读到更旧状态。
- 事件可能在页面尚未监听时发生，前端初始化顺序应为“注册监听 → 读取当前状态 → 按 revision 合并”。
- 同 revision 重复事件按幂等处理。
- 多类事件之间不承诺全局顺序；页面以各自 revision 和完整快照为准。
- 事件发送失败不回滚已完成的缓存或文件写入。

## 7. Capability 与权限

窗口标签固定为 `widget` 和 `settings`。Tauri Capability 只授权必要命令：

| Command | `widget` | `settings` | 后端内部/托盘 |
|---|:---:|:---:|:---:|
| `get_quota_snapshot` | 允许 | 允许 | 允许 |
| `refresh_quota` | 允许 | 允许 | 允许 |
| `get_app_server_status` | 允许 | 允许 | 允许 |
| `get_preferences` | 允许 | 允许 | 允许 |
| `set_theme` | 允许 | 允许 | 允许 |
| `save_preferences` | 禁止 | 允许 | 允许 |
| `set_widget_mode` | 允许 | 允许 | 允许 |
| `set_always_on_top` | 允许 | 允许 | 允许 |
| `set_click_through` | 允许 | 允许 | 允许 |
| `quit_app` | 允许 | 禁止 | 允许 |
| `check_for_updates` | 禁止 | 允许 | 允许 |
| `install_update` | 禁止 | 允许 | 允许 |

除 Capability 外，Rust Command 还要核对调用窗口标签，不能把前端传入的字符串当作调用者身份。

浮球右键菜单使用 Tauri 原生菜单模块，Capability 仅开放创建菜单与在当前鼠标位置弹出菜单所需的 `core:menu:allow-new`、`core:menu:allow-popup`，不开放完整菜单默认权限。菜单内容固定在前端 API 封装中，只包含“设置”和“退出”。

两个窗口均禁止获得：

- 通用文件系统读写。
- Shell、进程启动或 sidecar 原始调用。
- 任意 HTTP、WebSocket 或本地网络访问。
- App Server 原始方法调用。
- updater 自定义 URL、公钥或跳过签名验证。

详细能力配置以 [Tauri Capabilities 官方文档](https://v2.tauri.app/security/capabilities/) 为准。

## 8. Codex App Server 协议

### 8.1 官方基线

本节依据 [Codex App Server 官方文档](https://developers.openai.com/codex/app-server/) 于 2026-07-12 核对。实现时必须固定 sidecar 版本，并用该二进制生成 TypeScript 或 JSON Schema；官方 `main` 分支或网页后续变化不能直接改变已发布客户端契约。

### 8.2 传输

- 启动：`codex app-server`。
- 默认传输：`stdio`。
- 帧格式：UTF-8、逐行 JSON（JSONL），一行一条消息。
- 消息采用 JSON-RPC 2.0 形态，但线路上省略 `"jsonrpc":"2.0"`。
- 1.0.0 不启用 WebSocket、Unix socket 或远程连接模式，不监听端口。

### 8.3 握手

每个连接先发送一次：

```json
{
  "method": "initialize",
  "id": 1,
  "params": {
    "clientInfo": {
      "name": "quota_glance",
      "title": "QuotaGlance",
      "version": "1.0.0"
    }
  }
}
```

收到成功响应后发送通知：

```json
{
  "method": "initialized",
  "params": {}
}
```

握手完成前不得调用账户或额度方法。1.0.0 省略 `capabilities`，等价于不启用 `experimentalApi`；不得为了绕过方法限制设置该能力。

### 8.4 允许的账户读取

请求：

```json
{
  "method": "account/read",
  "id": 2,
  "params": {
    "refreshToken": false
  }
}
```

可能的结果示例：

```json
{
  "id": 2,
  "result": {
    "account": {
      "type": "chatgpt",
      "email": "user@example.com",
      "planType": "pro"
    },
    "requiresOpenaiAuth": true
  }
}
```

邮箱只用于说明官方返回结构，QuotaGlance Parser 必须立即丢弃，不能进入 IPC、日志、偏好或错误。

账户变化通知：

```json
{
  "method": "account/updated",
  "params": {
    "authMode": "chatgpt",
    "planType": "pro"
  }
}
```

当前官方列出的 `authMode` 包括：

```text
apikey
chatgpt
chatgptAuthTokens
agentIdentity
personalAccessToken
bedrockApiKey
null
```

未来字符串必须保留为受长度限制的 `authMode`，同时把 UI 状态归一化为 `unknown`，不能因未知枚举反序列化失败。

### 8.5 允许的额度读取

请求：

```json
{"method":"account/rateLimits/read","id":3}
```

结构示例：

```json
{
  "id": 3,
  "result": {
    "rateLimits": {
      "limitId": "codex",
      "limitName": null,
      "primary": {
        "usedPercent": 25,
        "windowDurationMins": 300,
        "resetsAt": 1783900800
      },
      "secondary": {
        "usedPercent": 18,
        "windowDurationMins": 10080,
        "resetsAt": 1784332800
      },
      "credits": {
        "hasCredits": true,
        "unlimited": false,
        "balance": "12.50"
      },
      "planType": "pro",
      "rateLimitReachedType": null
    },
    "rateLimitsByLimitId": {
      "codex": {
        "limitId": "codex",
        "limitName": null,
        "primary": {
          "usedPercent": 25,
          "windowDurationMins": 300,
          "resetsAt": 1783900800
        },
        "secondary": {
          "usedPercent": 18,
          "windowDurationMins": 10080,
          "resetsAt": 1784332800
        },
        "credits": {
          "hasCredits": true,
          "unlimited": false,
          "balance": "12.50"
        },
        "planType": "pro",
        "rateLimitReachedType": null
      }
    },
    "rateLimitResetCredits": {
      "availableCount": 2,
      "credits": [
        {
          "id": "RateLimitResetCredit_opaque",
          "resetType": "codexRateLimits",
          "status": "available",
          "grantedAt": 1781654400,
          "expiresAt": 1784246400,
          "title": "Rate-limit reset",
          "description": "Reset an eligible Codex rate-limit window."
        }
      ]
    }
  }
}
```

示例中的 credits 子字段必须由固定 sidecar Schema 再确认，不能仅凭示例实现宽松透传。

### 8.6 额度字段映射

| App Server 字段 | QuotaGlance 字段 | 映射规则 |
|---|---|---|
| `rateLimitsByLimitId` | `buckets[]` | 存在时作为权威多桶视图；Map key 是 metered `limit_id` |
| `rateLimits` | `buckets[0]` | 仅在多桶字段缺失时作为向后兼容视图 |
| Map key / `limitId` | `QuotaBucket.limitId` | 必须非空且一致；不一致桶隔离为不兼容 |
| `limitName` | `QuotaBucket.limitName` | 可选；缺失不影响其他字段 |
| `primary` | `windows[].slot=primary` | 对象存在且字段合法时加入 |
| `secondary` | `windows[].slot=secondary` | 可选，不能假定一定是周窗口 |
| `usedPercent` | `usedPercent` | 必须为 `0..=100` 有限数值 |
| `usedPercent` | `remainingPercent` | Rust 计算 `100-usedPercent`，前端不重复计算 |
| `windowDurationMins` | 同名字段、`kind`、`label` | 保留原始分钟数；常见时长做友好分类，其他为 `unknown` |
| `resetsAt` | `resetsAt` | Unix 秒转换为 UTC RFC 3339；不是毫秒 |
| `planType` | `QuotaBucket.planType` | 可选安全字符串；未知值保留 |
| `credits` | `QuotaBucket.credits` | 可选；仅映射固定 Schema 已确认字段，不猜货币或用途 |
| `rateLimitReachedType` | 同名字段和 `quotaReached` | 可选服务端分类；未知值保留 |
| `rateLimitResetCredits.availableCount` | `bankedResets.availableCount` | 权威数量，非负整数 |
| `rateLimitResetCredits.credits` | `bankedResets.details` | `null` 与空数组语义不同；明细可能被截断 |
| reset credit `id` | 不映射 | UI 无消费权限，按最小数据原则丢弃 |
| reset credit 时间 | `grantedAt` / `expiresAt` | Unix 秒转换为 UTC RFC 3339 |

`rateLimitsByLimitId` 存在但为空时仍是权威视图，不能悄悄回退到可能陈旧的单桶字段。响应存在多个合法桶时按稳定规则排序：用户选中的 `limitId` 优先，其次 `limitId=codex`，其余按 `limitId` 字典序；排序只是 UI 规则，不改变语义。

### 8.7 额度通知

官方通知示例可能只包含部分桶或部分字段：

```json
{
  "method": "account/rateLimits/updated",
  "params": {
    "rateLimits": {
      "limitId": "codex",
      "primary": {
        "usedPercent": 31,
        "windowDurationMins": 300,
        "resetsAt": 1783901700
      }
    }
  }
}
```

QuotaGlance 只把该通知视为缓存失效信号：

```text
通知到达
→ 150 ms 尾沿去抖并合并当前通知批次
→ SingleFlight 完整调用 account/rateLimits/read
→ 校验并原子替换快照
→ 发送 quota://snapshot-updated
```

禁止直接把通知载荷覆盖或合并到当前快照，避免缺失 `secondary`、其他额度桶、credits 或 reset credits 时误删数据。

### 8.8 账户通知归一化

| 官方值 | `AuthSummary.state` | 后续动作 |
|---|---|---|
| `apikey` / `account.type=apiKey` | `apiKeyMode` | 不展示 ChatGPT 订阅额度 |
| `chatgpt` | `authenticated` | 重读账户和额度 |
| `chatgptAuthTokens` | `authenticated` | 重读账户和额度 |
| `agentIdentity` | `authenticated` | 重读账户和额度，按服务端结果展示 |
| `personalAccessToken` | `authenticated` | 重读账户和额度，按服务端结果展示 |
| `bedrockApiKey` / `amazonBedrock` | `externalProvider` | 不混入 ChatGPT 订阅额度 |
| `null` 且需要 OpenAI 认证 | `signedOut` | 停止高频额度重试 |
| 未知字符串 | `unknown` | 保留安全字符串，尝试一次只读读取后按错误分类 |

`account/updated` 同样是失效信号。收到后完整调用 `account/read`，不能仅依赖通知中的 `planType` 构造账户快照。

## 9. App Server 禁止操作

### 9.1 明确禁止的方法

| 方法 | 禁止原因 |
|---|---|
| `account/login/start` | 会启动认证写流程，不属于额度查看工具 |
| `account/login/cancel` | 改变正在进行的认证流程 |
| `account/logout` | 改变账户状态 |
| `account/rateLimitResetCredit/consume` | 消耗一次已获得的重置额度，属于不可逆业务写操作 |
| `account/sendAddCreditsNudgeEmail` | 会向 workspace owner 发送邮件 |
| 任意账户 Token 注入/刷新请求 | QuotaGlance 不持有或管理 Token；`account/read.refreshToken` 固定为 false |
| 任意 thread/turn/item 写操作 | 与额度展示无关，扩大数据和权限边界 |
| `thread/shellCommand` 等执行方法 | 可运行命令，严重超出产品范围 |

### 9.2 未纳入 MVP 的只读方法

以下方法即使本身只读，也不在 1.0.0 允许列表中：

- `account/usage/read`
- `account/workspaceMessages/read`
- 线程、会话、配置、MCP、模型和插件等读取方法

如果以后加入活动摘要，必须单独更新需求、隐私、接口、权限和测试文档；不能复用“通用 App Server 调用”绕过评审。

### 9.3 代码级强制

- Provider 使用 Rust 枚举表示允许方法，不接受 `String method` 公共参数。
- Tauri 不暴露 `call_app_server`、`send_json` 或类似通用命令。
- 测试扫描代码和 fixture，确保禁止方法没有出现在调用分支；文档说明和测试字符串可列入豁免。
- App Server 发送未知服务端请求时返回固定 `-32601`，不执行回调。
- `experimentalApi` 始终为 false/省略，禁止通过偏好或环境变量开启。

## 10. 协议错误映射

| 外部情况 | 内部错误 | 快照行为 |
|---|---|---|
| App Server 返回未初始化 | `PROTOCOL_INVALID_MESSAGE` | 视为握手缺陷，重连；有旧值则 `stale` |
| 请求超时 | `PROTOCOL_REQUEST_TIMEOUT` | 可重试，有旧值则 `stale` |
| 进程 EOF/退出 | `APP_SERVER_EXITED` | 进程退避并重连，有旧值则 `stale` |
| 畸形 JSON | `PROTOCOL_INVALID_MESSAGE` | 关闭不可信连接并重启 |
| 消息超过上限 | `PROTOCOL_MESSAGE_TOO_LARGE` | 关闭连接；无旧值时 `incompatible` |
| App Server 返回认证错误 | `AUTH_REQUIRED` | 重读账户，确认后进入 `signedOut` |
| 可识别的繁忙/过载 | `SOURCE_BUSY` | 读取退避，不假定有 HTTP Header |
| 无法细分的上游故障 | `SERVICE_UNAVAILABLE` | 读取退避 |
| 所有额度桶均无法解析 | `RESPONSE_INCOMPATIBLE` | 不显示 0；保留旧值或进入 `incompatible` |
| 单个可选桶/窗口错误 | 局部解析警告 | 隔离该项，其余合法数据继续显示 |

原始 App Server error code/message 只用于后端分类，不发送到前端或日志。无法安全分类时使用 `SERVICE_UNAVAILABLE` 或 `RESPONSE_INCOMPATIBLE`，不猜测 HTTP 状态。

## 11. 接口版本管理

- `QuotaSnapshot.schemaVersion` 和偏好 `schemaVersion` 是不同版本域，当前都为 1。
- 新增可选字段可以保持 IPC Schema 1；删除字段、改变语义或收紧已有枚举需提升 IPC Schema。
- 前端和 Rust 同包发布，不支持不同应用版本的远程独立部署，但仍需版本字段防止缓存和事件混淆。
- App Server Schema 与 sidecar 版本绑定，升级 sidecar 时必须更新生成 Schema 和契约 fixture。
- 未知外部枚举通常映射为安全字符串或 `unknown`；未知外部结构不能用 `any` 穿透到 UI。

## 12. 接口测试清单

### 12.1 IPC 契约

- [ ] 所有命令输入拒绝未知字段、错误枚举、非有限数值和超长字符串。
- [ ] Rust 序列化结果与 TypeScript 契约一致，所有可选值稳定为 `null`。
- [ ] widget/settings 权限矩阵逐项验证，伪造窗口标签不能越权。
- [ ] 错误对象不含路径、邮箱、账号 ID、Token、原始消息或 reset credit ID。
- [ ] 事件丢失、重复和乱序时，前端按 revision 收敛。
- [ ] `save_preferences` 的旧 revision 不覆盖新偏好。

### 12.2 App Server 契约

- [ ] 每个连接只执行一次 `initialize → initialized`，握手前无业务请求。
- [ ] 初始化中没有 `experimentalApi=true`。
- [ ] `account/read` 覆盖空账户、API Key、ChatGPT、Bedrock、官方全部 authMode 和未知值。
- [ ] 邮箱在 Parser 后不可见。
- [ ] 多桶存在时优先 `rateLimitsByLimitId`，字段缺失时兼容单桶 `rateLimits`。
- [ ] `usedPercent`、`windowDurationMins` 和 Unix 秒 `resetsAt` 的边界转换正确。
- [ ] credits 可选字段、未知字段和数值字符串不会造成精度误判。
- [ ] reset credits 的 `null` 明细、空明细、截断明细和权威数量均正确处理，`id` 不进入 IPC。
- [ ] 部分、重复、乱序额度通知只触发合并后的完整重读。
- [ ] 代码无法调用 consume、logout、nudge email、Shell 或通用 App Server 方法。

### 12.3 状态与恢复

- [ ] 首次读取返回 `loading`，不出现假 0%。
- [ ] 成功后刷新失败进入 `stale`，业务值保持最后成功版本。
- [ ] App Server 退出后在途请求失败，重连握手并完整重读。
- [ ] 30 秒手动冷却和 SingleFlight 均不产生重复请求。
- [ ] 未知响应结构进入 `incompatible`，升级后可以恢复。

以上清单是 1.0.0 完整验收条件。0.1.0 已实现子集以第 1.1 节及命令、事件总览的状态列为准；未通过 Windows/macOS 实机验证的条目仍保持“目标契约”状态。
