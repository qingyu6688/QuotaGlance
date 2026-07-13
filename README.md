# QuotaGlance

> 额度一览——额度一眼可见，编码不再撞线。

QuotaGlance 是一款面向 Codex 高频用户的本地桌面额度助手。应用通过展开卡片、悬浮球和系统托盘展示额度桶、剩余比例、重置时间、Credits、数据新鲜度与异常状态。

项目坚持本地优先和只读原则：正常数据链路由 Codex App Server 处理认证并读取额度；QuotaGlance 不读取 `auth.json`、不访问系统凭据库、不持有 Token，也不执行登录、登出、购买额度等账号写操作。

## 当前状态

当前工程版本为 **`0.1.0`**，Tauri 2、React、TypeScript、Vite 与 Rust 工程已经建立。前端、Rust 和 Tauri 桌面壳均可构建，但项目仍处于 M0/M1 实施阶段，不是可发布的正式版本。

| 范围 | 当前结果 |
|---|---|
| React 界面 | 冷色发光边框卡片、四层信息球形悬浮球、两项右键菜单、七主题设置面板和异常状态已形成可运行实现 |
| Rust 核心 | 动态额度模型、JSONL 协议、常驻 App Server 会话、pending request map、窗口与托盘骨架已实现 |
| 自动化检查 | 前端 14 项测试、Rust 44 项测试通过 |
| 构建验证 | 前端生产构建、Tauri debug `--no-bundle` 构建通过 |
| Release 预览 | 已生成 Windows x64 未签名 EXE、NSIS 和 MSI，并提供 SHA-256 清单 |
| 运行验证 | Tauri 进程烟测和浏览器界面 QA 通过 |
| 正式分发 | 尚未完成 bundled sidecar、安装包、签名、公证和双平台验收 |
| 目标正式版本 | `1.0.0` |

上述结果最近一次于 2026-07-13 在 Windows 10 开发机验证。Windows 10 不是当前正式支持承诺；Windows 11、macOS、代码签名和 macOS 公证仍需独立验证。

## 已实现的工程能力

- 动态建模 App Server 返回的额度桶和时间窗口，不把套餐写死为固定字段。
- 通过常驻 `stdio` JSONL 会话执行一次握手，并复用连接完成 `account/read` 和 `account/rateLimits/read`。
- 对协议消息设置大小限制、请求超时和受控错误映射，前端不接触原始凭据。
- 使用有界 pending request map 按 ID 路由乱序响应；超时会移除 pending，断连会失败全部在途请求。
- 订阅账号和额度通知，经过防抖后执行完整重读；未知通知不会修改快照。
- 使用仅测试特性编译的假 App Server 完成跨进程契约，覆盖握手、乱序响应、迟到响应、通知、错误、超时、异常退出和超长消息。
- 通过 Tauri Commands 与 Events 同步额度、刷新、偏好和窗口状态。
- 提供带主题光效边框的展开卡片和 128px 球形额度浮球；球内严格按“周额度、动态百分比、重置日期、状态”四层排版，横向水面会随周额度升降并惯性晃动。设置支持跟随系统、极光、石墨、纸白、日落珊瑚、蜂蜜琥珀、玫瑰铜夜七套主题，浮球右键菜单只保留“设置”和“退出”。
- 浏览器开发模式使用本地模拟数据，可独立检查布局、状态和交互。
- 提供 ESLint、TypeScript、Vitest、Cargo test 与 Clippy 检查基础。

## 当前限制

- **生产构建尚未携带 Codex App Server sidecar。** Windows Release 可复用已安装 Codex 桌面应用管理的本地运行副本和现有登录态，但固定版本、完整兼容矩阵与再分发方案仍未完成。
- 本地 `0.1.0` Release 目录只用于安装和启动预览；三个 Windows 产物均未签名，不是正式公开发行版。
- 常驻会话、通知驱动重读、30 秒自动刷新缓存、SingleFlight、可见/隐藏定时重同步、最后成功快照和退避恢复已经接入。
- 浏览器模式展示的是模拟数据，不代表已连接真实 Codex 账号。
- 窗口模式、置顶、鼠标穿透和主题偏好已经原子落盘并支持备份恢复；语言、窗口边界、安装器、自动更新、签名、公证及发布回滚仍未完成。
- 仅在 Windows 10 开发机完成构建和烟测；Windows 11 与 macOS 实机兼容性尚未验收。

## 快速开始

### 前置条件

- Node.js `>= 20.19.0` 与 npm；
- Rust stable 与 Cargo；
- 当前平台对应的 [Tauri 开发前置条件](https://v2.tauri.app/start/prerequisites/)。

以下命令均在项目根目录执行。

```powershell
npm ci
```

启动浏览器界面。该模式使用本地模拟额度，不需要 Codex App Server：

```powershell
npm run dev
```

启动 Tauri 桌面开发版本：

```powershell
npm run tauri dev
```

运行前端完整检查：

```powershell
npm run check
```

运行 Rust 检查与测试：

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features
```

构建不生成安装包的 Tauri debug 可执行文件：

```powershell
npm run tauri -- build --debug --no-bundle
```

### 连接开发用 Codex App Server

调试构建可通过 `QUOTAGLANCE_CODEX_PATH` 指定 Codex 可执行文件。该值必须是**绝对文件路径**：

```powershell
$env:QUOTAGLANCE_CODEX_PATH = "C:\Tools\codex\codex.exe"
npm run tauri dev
```

未设置变量时，debug 构建会尝试从 `PATH` 查找 `codex`。该变量仅供 debug 使用，release 构建会忽略它；正式分发仍需完成受控、固定版本并可校验的 bundled sidecar 方案。不要把 Token、API Key 或 `auth.json` 内容写入 `.env`。

Windows 上若已安装并登录 Codex 桌面应用，QuotaGlance 会在 `%LOCALAPPDATA%\OpenAI\Codex\bin\<managed-id>\codex.exe` 中选择最新的受管运行副本。该路径只用于启动 `app-server`，应用不会读取 Codex 的认证文件或凭据内容。

## 技术架构

```text
React UI
  → Tauri Commands / Events
  → Rust 应用与领域层
  → Codex App Server Provider
  → stdio JSONL
  → codex app-server
```

首版不建设服务端，不使用 MySQL，不开放本地 REST 端口，也不提供云同步、多账号自动切换或其他 AI Provider。

## 项目文档

- [文档中心](docs/README.md)
- [项目实施计划](docs/project-plan.md)
- [未完成问题分析与解决方案](docs/issues-analysis.md)
- [首版界面设计基线](docs/ui-design.md)
- [需求规格说明](docs/requirements.md)
- [概要设计](docs/design.md)
- [详细设计](docs/detail-design.md)
- [IPC 与 App Server 接口设计](docs/api.md)
- [测试计划与报告](docs/test.md)
- [部署与发布](docs/deploy.md)
- [安全设计](docs/security.md)
- [隐私说明](docs/privacy.md)

参考项目 [change-42-yhmm/quota-float](https://github.com/change-42-yhmm/quota-float) 仅用于理解 Tauri 桌面壳、浮动窗口与托盘交互。QuotaGlance 不采用其凭据读取方式、私有 HTTP 接口或固定额度窗口模型。

## 安全与隐私

- WebView 不直接访问 Codex 远端服务。
- 前端不获得通用文件系统、Shell 或任意网络权限。
- 日志、错误和诊断信息不得包含 Token、账号 ID、原始协议响应或个人路径。
- 刷新失败时保留最后一次成功快照，不把未知值显示为 `0%`。
- 默认不启用遥测，不上传额度历史、账号信息、提示词或项目内容。

详见 [安全设计](docs/security.md) 和 [隐私说明](docs/privacy.md)。

## 品牌与许可

QuotaGlance 是独立第三方工具，与 OpenAI 不存在隶属、授权或背书关系。项目采用 [MIT License](LICENSE)。若后续安装包随附 Codex App Server 二进制，发布前必须完成再分发审查，并保留相应许可、NOTICE、固定版本和来源校验信息。

## 维护

维护邮箱：maorongkang@gmail.com
