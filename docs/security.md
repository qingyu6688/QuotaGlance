# QuotaGlance 安全设计与威胁模型

> 文档状态：安全设计初稿  
> 核对日期：2026-07-12  
> 当前阶段：0.1.0 开发基线已实现部分控制并通过单元测试，尚未完成完整安全审计与实机验证  
> 文档维护与临时安全联系：maorongkang@gmail.com

## 1. 安全目标

QuotaGlance 是本地只读额度查看工具。1.0 的安全目标是：

1. 正常路径不读取、保存或向前端传递 Codex Token、API Key、Cookie 或系统凭据库内容；
2. 只读取账号模式和额度状态，不执行登录、登出、购买、发信、兑换重置次数等写操作；
3. WebView、Rust Core、App Server、更新系统之间保持最小权限和明确边界；
4. sidecar、应用包和更新产物具有可验证的来源与完整性；
5. 错误、日志、崩溃和诊断流程不会成为敏感数据泄漏通道；
6. 协议变化、畸形输入、进程退出和网络故障不会导致任意代码执行或显示伪造额度。

“只读”是产品能力限制，不代表 App Server 与官方服务之间没有认证流量。认证由 Codex App Server 负责，QuotaGlance 不应复制或重新实现认证。

当前代码已实现只读方法白名单、类型化协议模型、固定参数且不经过 Shell 的进程启动、JSONL 大小限制与请求超时，并将 WebView Capability 收敛到最小范围。9 项假 App Server 跨进程契约覆盖未知请求拒绝、错误脱敏、畸形输出、超时和异常退出；Windows 真实 POC 通过 Codex 自身登录态读取额度，未读取认证文件或凭据内容。以上仍不能替代 production sidecar 供应链、双平台运行时、签名更新和安装器的完整安全审计。

## 2. 范围与非目标

### 2.1 本文覆盖

- React WebView 与 Tauri IPC；
- Rust 领域层、缓存、调度和本地偏好；
- Codex App Server sidecar 的启动、协议与生命周期；
- 默认关闭的 `LegacyWhamProvider`；
- Windows/macOS 安装、签名与自动更新；
- 构建依赖、sidecar 和 CI/CD 供应链；
- 日志、诊断、漏洞报告和事件响应。

### 2.2 不在应用自身可保证的范围

- 已被管理员/root 权限完全控制的操作系统；
- 用户主动关闭系统安全机制并替换应用或 sidecar；
- Codex App Server 或 OpenAI 官方服务自身的安全与可用性；
- 企业网络代理、终端安全软件和系统凭据库的内部实现；
- 用户在 QuotaGlance 之外泄露的 Codex 凭据。

即使属于非目标，应用也不得通过不安全默认值扩大损害，例如不得因 sidecar 启动失败而自动读取 Token。

## 3. 资产与敏感度

| 资产 | 敏感度 | 位置 | 安全要求 |
|---|---|---|---|
| Codex access token、API Key、刷新令牌 | 极高 | Codex 管理的文件、系统凭据库或 App Server | 正常路径不得读取、复制、记录或发送给前端 |
| updater 私钥、平台签名私钥 | 极高 | CI Secret、硬件或可信签名服务、离线备份 | 不进入客户端、仓库、日志和普通 artifact |
| sidecar 与应用二进制 | 高 | 安装目录和发布存储 | 固定版本、哈希、架构、签名和来源 |
| updater 公钥与更新元数据 | 高 | 应用配置和发布端 | 公钥防篡改；元数据经 HTTPS 获取且产物验签 |
| 账号认证模式与套餐 | 中 | Rust 内存中的最小字段 | 不保留邮箱、账号 ID 等非必要字段 |
| 额度快照与重置时间 | 中 | 默认只在内存 | 不形成默认历史，不进入遥测 |
| 用户偏好 | 低至中 | 本地应用数据目录 | 最小字段、原子写入、当前用户权限 |
| 日志和诊断信息 | 取决于内容 | 本地日志或用户主动导出 | 默认脱敏，不含原始协议和个人路径 |

## 4. 信任边界与数据流

```text
用户
  ↓ 本地交互
React WebView
  ↓ 细粒度 Tauri Commands / Events
Rust Core
  ↓ 子进程 stdin/stdout：JSONL
固定 Codex App Server sidecar
  ↓ HTTPS，由 App Server 管理认证
Codex 官方服务

更新发布端
  ↓ HTTPS 元数据与更新包
Tauri Updater
  ↓ 内置公钥验签 + 操作系统代码签名验证
已安装应用
```

### 4.1 WebView ↔ Rust

WebView 属于低信任展示层。即使发生 XSS，也不应获得通用 Shell、文件系统、网络或子进程能力。Rust 必须重新校验来自前端的枚举、数值范围、路径和状态转换，不能把 TypeScript 类型当作安全边界。

### 4.2 Rust ↔ App Server

App Server 是受控子进程，但其输出仍按不可信输入处理。Rust 只解析允许的方法和字段，设置消息大小、请求超时和进程资源边界，不把原始 JSON 直接转发给前端。

### 4.3 App Server ↔ 官方服务

App Server 负责读取 `file | keyring | auto` 凭据和访问官方服务。Codex 官方说明：

- `file` 把含 access token 的 `auth.json` 保存到 `CODEX_HOME`；
- `keyring` 使用操作系统凭据库；
- `auto` 优先使用凭据库，不可用时回退到文件。

QuotaGlance 正常路径不读取 `auth.json`，不访问 keyring，不构造 `Authorization` Header。Rust 在协议边界解析 App Server 响应，只把白名单转换后的额度字段交给领域层和前端；非必要账号字段立即丢弃。

### 4.4 更新发布端 ↔ 客户端

HTTPS 保护传输，Tauri updater 签名保护更新产物完整性，Windows/macOS 代码签名保护平台信任链。这三者用途不同，不能相互替代。

## 5. 威胁主体

- 利用 WebView 内容或前端依赖漏洞的远程攻击者；
- 能以当前普通用户权限运行其他进程的本地攻击者；
- 向仓库提交恶意依赖、脚本或构建变更的供应链攻击者；
- 控制下载站、更新元数据或网络中间节点的攻击者；
- 被替换、损坏或版本不兼容的 sidecar；
- 误把真实响应、Token 或个人路径提交到 Git/CI 的开发者；
- 诱导用户开启 Legacy 兼容路径的社会工程攻击者。

## 6. 主要威胁与控制

| 威胁 | 影响 | 主要控制 | 验证方式 |
|---|---|---|---|
| XSS 后调用高权限 Tauri API | 文件泄露、任意命令执行 | 严格 CSP、无远程脚本、按窗口 capability、细粒度命令白名单 | 权限负向测试、CSP 检查 |
| IPC 参数注入 | 路径逃逸、状态破坏 | Rust 端类型与范围校验，不执行前端传入命令 | 模糊测试、非法参数测试 |
| sidecar 路径或参数注入 | 任意程序执行 | 固定路径、参数固定为 `app-server`、不经过 Shell、外部 CLI 需显式选择与校验 | 特殊字符路径、替换文件测试 |
| sidecar 被替换 | 凭据或数据泄露 | 构建期哈希、系统签名、受保护安装目录、启动前完整性与版本检查 | 篡改二进制测试 |
| 恶意或畸形 JSONL | 崩溃、内存耗尽、UI 欺骗 | 消息大小上限、超时、schema 校验、未知字段容忍、原始值范围检查 | fuzz、超长和乱序 fixture |
| 通知风暴或反复退出 | CPU/网络耗尽 | 防抖、SingleFlight、指数退避、熔断和可控重启次数 | 假进程压力测试 |
| Token 进入前端或日志 | 账号接管 | 正常路径不读凭据、字段白名单、结构化脱敏、禁止原始协议日志 | secret scan、运行时哨兵值测试 |
| 更新站被控制 | 恶意更新 | HTTPS、Tauri 强制签名、系统代码签名、默认防降级 | 篡改更新包和元数据测试 |
| updater 私钥泄露 | 可伪造更新 | CI 最小权限、隔离签名、离线备份、Secret 不进入 PR 工作流 | CI 权限审查、密钥演练 |
| 依赖或构建脚本投毒 | 构建产物失陷 | 锁文件、固定 action、依赖审计、SBOM、受保护发布工作流 | 可复现检查、审计 |
| 偏好文件被构造 | 崩溃、窗口不可找回 | 大小和 schema 校验、默认值恢复、原子写入 | 损坏和超大配置测试 |
| Legacy 私有接口变化 | 凭据暴露、误报额度 | 默认关闭、显式警告、固定主机、禁重定向、响应上限、独立可移除 | 专项安全测试 |

## 7. 最小权限设计

### 7.1 应用进程

- 普通运行不申请管理员/root 权限；
- 不修改 Codex 安装、认证文件或系统凭据库；
- 不开放本地 TCP、WebSocket 或 HTTP 端口；
- 不扫描用户项目、主目录或浏览器数据；
- 本地偏好只写应用自己的数据目录；
- 开机启动只在用户明确开启时创建最小自启动项；
- 通知权限只在需要本地额度提醒时申请；
- 不申请摄像头、麦克风、位置、通讯录或屏幕录制权限。

### 7.2 Tauri 权限

`widget`、`settings` 等窗口分别声明能力。只授予业务所需的事件、窗口、托盘、通知、开机启动和 updater 权限。

前端不得获得：

- 通用 `shell` 执行；
- 任意文件读写；
- 任意 URL 网络访问；
- 任意子进程启动；
- 无边界的剪贴板或系统信息读取。

Tauri 默认注册的自定义 command 可能对全部窗口可用，因此工程建立时必须同时使用 capability 和自定义命令清单限制调用面，并加入自动化检查。

### 7.3 App Server 方法白名单

1.0 仅允许实现业务所需的稳定读取面，例如：

- `account/read`；
- `account/rateLimits/read`；
- `account/updated` 通知；
- `account/rateLimits/updated` 通知。

明确禁止从 QuotaGlance 调用：

- 登录、登出或凭据刷新入口；
- `account/rateLimitResetCredit/consume`；
- `account/sendAddCreditsNudgeEmail`；
- 购买、修改账号或工作区的操作；
- 1.0 范围外的 `account/usage/read`；
- 任意 thread、turn 或代码执行相关方法。

`initialize` 必须使用 `clientInfo.name = "quota_glance"` 如实标识客户端。1.0 省略 `experimentalApi` 或显式设为 `false`。

## 8. Sidecar 安全

`0.1.x` 公开预览版不捆绑或重新分发 Codex App Server。它会复用用户已经安装的 Codex：Windows 优先使用 Codex 桌面应用管理的运行副本，随后检查 `PATH`；macOS/Linux 检查 `PATH` 与常见用户安装目录。候选路径必须规范化为普通文件，Unix 还必须具有可执行位；启动参数固定为 `app-server`，不经过 Shell，也不会读取 `auth.json` 或 Token。该模式的供应链信任依赖用户本机已有安装，因此 Release 必须明确标记为预览版。

以下要求面向正式 `1.0.0` 的 bundled sidecar 分发：

- 版本、来源、目标架构、SHA-256、许可和签名结果进入发布清单；
- Windows/macOS 的 sidecar 与外层应用一同签名和验证；
- 不依赖 PATH 搜索，不自动选择 WindowsApps 中的内部文件；
- 外部 CLI 模式由用户明确选择，只接受常规文件，规范化后校验版本；
- 禁止符号链接或重解析点把固定资源路径导向应用目录之外；
- 子进程工作目录使用受控目录，不使用用户项目目录；
- stdin/stdout 只传输 JSONL；stderr 作为诊断流，仍需逐行脱敏和限长；
- 每个请求设置超时和待处理请求上限；
- 单条消息、缓冲区和累计未解析数据均设置上限；
- App Server 退出后清理句柄和待处理请求，按退避策略重启；
- 不向子进程传入与运行无关的命令行参数；
- 不枚举、回显或记录父进程环境；按官方运行要求最小化传递环境，并覆盖 file/keyring/auto 实机测试。

完整数值上限在实现时提取为具名常量，并由测试固定。修改上限视为安全相关变更。

## 9. WebView 与内容安全

- 所有脚本、样式、字体和图片随应用打包；
- 生产 CSP 禁止 `unsafe-eval`，避免 `unsafe-inline`；
- 不加载第三方分析脚本、远程字体、远程图片和广告；
- 外部链接通过受控命令交给系统浏览器，不在高权限 WebView 中导航；
- 禁止把 App Server 原始 HTML、Markdown 或错误字符串作为不受控 HTML 渲染；
- 用户可见文本使用框架默认转义；
- 依赖升级后重新检查 CSP 和 Tauri capability；
- 开发服务器只绑定回环地址，生产包不包含开发工具入口。

## 10. 本地存储

默认只持久化窗口位置、窗口模式、语言、主题、提醒、更新通道和最后检查时间等偏好。

要求：

- 不保存 Token、账号 ID、邮箱、认证文件路径和原始额度响应；
- 不默认保存额度历史；
- 快照只驻留内存，进程退出后自然清除；
- 配置写入采用临时文件加原子替换，避免断电损坏；
- 配置文件使用当前用户可读写权限，不放入公共目录；
- 读取时校验文件类型、大小、schema 和字段范围；
- 损坏配置回退安全默认值，并给出不含原始内容的提示；
- 卸载与手动删除路径在用户手册中明确记录。

## 11. 日志与脱敏

### 11.1 允许记录

- 应用、平台和 App Server 的非敏感版本；
- 内部事件名称，例如“刷新开始”“sidecar 退出”；
- 脱敏错误码和错误类别；
- 耗时、重试次数、缓存命中和状态转换；
- 不含个人信息的目标架构和签名验证结果。

### 11.2 禁止记录

- access token、API Key、Cookie、Authorization Header 和 `auth.json` 内容；
- 邮箱、账号 ID、workspace ID、用户名和 reset credit opaque ID；
- 原始请求、响应、JSONL 行和 HTTP 正文；
- 提示词、聊天历史、项目名、文件内容；
- 完整主目录、`CODEX_HOME`、外部 CLI 路径和环境变量；
- updater 私钥、证书、密码、PIN 和 CI Secret。

### 11.3 实现要求

- 采用结构化事件和固定字段，不通过 `Debug` 打印整个对象；
- 在数据进入日志框架前脱敏，而不是导出诊断包时补救；
- 错误链逐层检查，第三方库错误可能携带 URL、路径或正文；
- 生产默认使用 `info` 或更低详细度，不开放包含协议原文的 trace；
- 崩溃报告和遥测默认关闭；
- 如果未来增加诊断导出，先生成预览，由用户主动确认，仍不得包含凭据。

## 12. LegacyWhamProvider 专项风险

`LegacyWhamProvider` 使用非公开兼容接口，是唯一允许直接读取文件型凭据的模块，风险显著高于 App Server 主路径。

必须满足：

- 默认关闭，启动失败、离线或 App Server 缺失时不得自动启用；
- 用户在设置中主动开启，并看到其会读取文件型凭据、接口可能失效的明确警告；
- 只读取 `CODEX_HOME/auth.json` 或官方默认文件路径，不访问系统 keyring；
- 认证文件有文件类型、所有者、大小和 JSON schema 限制；
- Token 只在 Rust 内存中短暂使用，不进入前端、设置、日志和错误；
- 只允许 HTTPS 的固定 `chatgpt.com` 主机；
- 禁止重定向，防止敏感 Header 被带到其他主机；
- 设置连接、读取和总超时，响应正文上限 1 MiB；
- 401/403 不反复重试，429 遵守 `Retry-After`；
- 不调用 reset credit 兑换、购买、发信或任何写操作；
- 代码、测试、日志类别和权限与正式 Provider 隔离；
- 可通过构建开关或独立模块从发行版完全移除；
- 私有接口改变访问控制时，不尝试绕过。

是否把 Legacy 能力带入 1.0 正式包，应在安全审查后单独批准。没有批准即保持不可用。

## 13. 供应链与发布安全

- npm 与 Cargo 使用锁文件，CI 使用确定性安装；
- GitHub Actions 或其他 CI action 固定到经过审查的版本，关键步骤优先固定提交；
- 定期执行 npm、Cargo 依赖审计和 secret scan；
- 构建脚本变更按安全敏感文件审查；
- fork Pull Request 不接触发布 Secret；
- sidecar 不使用浮动 latest，发布前核对固定版本的 Apache-2.0 LICENSE/NOTICE；
- Windows 应用、sidecar、安装器完成 Authenticode；
- macOS 嵌套 sidecar 和应用完成 Developer ID 签名、公证与 stapling；
- updater 强制验证签名，私钥与平台签名密钥分离；
- 正式产物保存 SHA-256、SBOM、测试报告和审批记录；
- 默认拒绝版本降级，同一版本号不得替换二进制。

## 14. 安全验证门槛

以下清单保持未勾选，表示发布级验证尚未完成。开发期已有单元测试或代码证据的项目，也必须在真实运行、权限审查和发布供应链审计完成后才能勾选。

- [ ] 正常路径对 `auth.json` 和 keyring 没有代码访问；
- [ ] 前端事件与命令序列化模型不包含 Token、邮箱和账号 ID；
- [ ] App Server 方法白名单没有任何写操作；
- [ ] `experimentalApi` 未启用；
- [ ] WebView 无通用网络、文件系统和 Shell 权限；
- [ ] CSP、外部导航和 HTML 渲染经过审查；
- [ ] JSONL 大小、超时、并发和子进程退出用例通过；
- [ ] sidecar 替换、哈希错误、架构错误和路径注入用例通过；
- [ ] 日志哨兵测试证明疑似 Token、邮箱和路径不会落盘；
- [ ] Legacy 未经用户明确启用不可达，专项负向测试通过；
- [ ] 依赖审计没有未接受的高危问题；
- [ ] Windows/macOS 与 updater 的签名、篡改拒绝测试通过；
- [ ] 隐私说明与实际数据流一致。

任何凭据泄露、任意命令执行、未签名更新可安装或 sidecar 来源不可追溯的问题都阻断发布。

## 15. 漏洞报告

项目尚未建立独立安全工单系统。当前请通过 maorongkang@gmail.com 私下报告，并在标题中注明“QuotaGlance 安全问题”。

报告建议包含：

- 受影响版本、平台和架构；
- 可重复的最小步骤；
- 预期结果与实际结果；
- 已脱敏的日志或截图；
- 可能影响。

请不要发送真实 Token、API Key、`auth.json`、Cookie、个人额度响应或签名私钥。若问题可能被公开利用，请先私下沟通，在修复版本可用前避免公开完整利用细节。

## 16. 事件响应

### 16.1 基本流程

1. **确认**：记录时间、版本、平台、来源和初步影响，保存只读证据；
2. **遏制**：暂停更新清单和问题版本下载，隔离相关 CI、密钥或构建机；
3. **评估**：确认受影响版本、资产、用户范围和是否存在实际利用；
4. **修复**：消除根因，补回归测试和供应链检查；
5. **恢复**：以更高补丁版本重新签名、公证和发布，验证覆盖升级；
6. **沟通**：用可操作、不过度推测的语言说明影响和用户措施；
7. **复盘**：记录时间线、根因、遗漏控制和负责人，更新本文。

### 16.2 凭据疑似泄露

- 停止收集和传播相关日志或 artifact；
- 确认泄露来自 QuotaGlance、Legacy 还是外部环境；
- 通知受影响用户撤销或重新登录相关 Codex 凭据；
- 删除公开副本，但保留受控取证副本和访问记录；
- 禁用问题功能，修复脱敏和字段白名单；
- 在重新发布前加入哨兵值回归测试。

### 16.3 updater 或签名密钥疑似泄露

- 立即撤下更新元数据并停止签名任务；
- 隔离、吊销或冻结对应密钥与 CI 身份；
- 审计泄露时间窗内的所有签名和发布记录；
- updater 私钥轮换需考虑旧客户端内置公钥，必要时使用仍可信更新链或系统签名完整安装器迁移；
- 不要求用户关闭签名校验完成恢复。

### 16.4 sidecar 供应链异常

- 撤下所有包含异常哈希的产物；
- 对照锁定记录确认版本、来源和构建链；
- 检查 sidecar 是否访问过凭据或输出过敏感内容；
- 从可信固定来源重建，以更高版本完整替换应用和 sidecar；
- 发布受影响版本和哈希清单。

## 17. 官方参考

- [Codex 认证与凭据存储](https://developers.openai.com/codex/auth)
- [Codex App Server](https://developers.openai.com/codex/app-server)
- [Tauri Capabilities](https://v2.tauri.app/security/capabilities/)
- [Tauri Updater](https://v2.tauri.app/plugin/updater/)
- [Tauri Windows 代码签名](https://v2.tauri.app/distribute/sign/windows/)
- [Tauri macOS 代码签名与公证](https://v2.tauri.app/distribute/sign/macos/)
- [OpenAI Codex LICENSE](https://github.com/openai/codex/blob/main/LICENSE)
