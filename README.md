# QuotaGlance

[![CI](https://github.com/qingyu6688/QuotaGlance/actions/workflows/ci.yml/badge.svg)](https://github.com/qingyu6688/QuotaGlance/actions/workflows/ci.yml)
[![Release](https://github.com/qingyu6688/QuotaGlance/actions/workflows/release.yml/badge.svg)](https://github.com/qingyu6688/QuotaGlance/actions/workflows/release.yml)
[![GitHub Release](https://img.shields.io/github/v/release/qingyu6688/QuotaGlance?include_prereleases)](https://github.com/qingyu6688/QuotaGlance/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-2ea44f.svg)](LICENSE)
[![Platforms](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-2878ff)](#下载与平台支持)

> 额度一览——额度一眼可见，编码不再撞线。

QuotaGlance 是一款面向 Codex 高频用户的本地桌面额度助手。应用通过展开卡片、悬浮球和系统托盘展示额度桶、剩余比例、重置时间、Credits、数据新鲜度与异常状态。

项目坚持本地优先和只读原则：正常数据链路由 Codex App Server 处理认证并读取额度；QuotaGlance 不读取 `auth.json`、不访问系统凭据库、不持有 Token，也不执行登录、登出、购买额度等账号写操作。

![QuotaGlance 项目展示：发光额度卡片与动态水面悬浮球](docs/design-assets/quotaglance-ui-concept-v1.png)

<p align="center">七套主题、发光玻璃边框、周额度卡片与动态水面悬浮球</p>

## 当前状态

当前工程版本为 **`0.1.0`**，以 MIT License 完整开源。Tauri 2、React、TypeScript、Vite 与 Rust 工程已经建立，并通过 GitHub Actions 在 Windows、macOS 和 Linux 上执行原生构建。该版本仍是未签名的跨平台预览版，不代表已完成商店级签名、公证和全部实机验收。

| 范围 | 当前结果 |
|---|---|
| React 界面 | 冷色发光边框卡片、四层信息球形悬浮球、两项右键菜单、七主题设置面板和异常状态已形成可运行实现 |
| Rust 核心 | 动态额度模型、JSONL 协议、常驻 App Server 会话、pending request map、窗口与托盘骨架已实现 |
| 自动化检查 | 前端 14 项测试、Rust 45 项测试；CI 覆盖 Windows、macOS、Linux |
| 构建验证 | 前端生产构建、Tauri debug `--no-bundle` 构建通过 |
| Release 预览 | 标签触发 Windows、macOS、Linux 原生安装包并上传 GitHub Releases |
| 运行验证 | Tauri 进程烟测和浏览器界面 QA 通过 |
| 正式分发 | 尚未完成 bundled sidecar、代码签名、macOS 公证和全平台实机验收 |
| 目标正式版本 | `1.0.0` |

上述本地结果最近一次于 2026-07-13 在 Windows 10 开发机验证；其他系统的编译结果由 GitHub 托管运行器提供。Windows 11、macOS、Linux 桌面环境、代码签名和 macOS 公证仍需社区实机反馈与后续验收。

## 下载与平台支持

安装包统一发布到 [GitHub Releases](https://github.com/qingyu6688/QuotaGlance/releases)。请选择与系统和架构匹配的文件：

| 平台 | 架构 | Release 资产 | 当前级别 |
|---|---|---|---|
| Windows | x64 | NSIS `.exe`、MSI `.msi` | 未签名预览 |
| macOS | Apple Silicon、Intel | `.dmg` | ad-hoc 签名、未公证预览 |
| Linux | x64、ARM64 | `.AppImage`、`.deb` | 社区预览 |

所有平台都需要用户自行安装并登录 Codex。QuotaGlance 不捆绑、不重新分发 Codex App Server：Windows 优先发现 Codex 桌面应用受管运行时；macOS/Linux 从 `PATH`、`/usr/local/bin`、`/opt/homebrew/bin`、`~/.local/bin` 或 `~/.npm-global/bin` 查找 `codex`。

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

- **生产构建不携带 Codex App Server sidecar。** 用户需要自行安装并登录 Codex；固定兼容范围与再分发方案仍未完成。
- `0.1.0` 为公开预览版。Windows 产物未签名，macOS 仅使用 ad-hoc 签名且未公证，Linux 尚未覆盖全部发行版和桌面环境。
- 常驻会话、通知驱动重读、30 秒自动刷新缓存、SingleFlight、可见/隐藏定时重同步、最后成功快照和退避恢复已经接入。
- 浏览器模式展示的是模拟数据，不代表已连接真实 Codex 账号。
- 窗口模式、置顶、鼠标穿透和主题偏好已经原子落盘并支持备份恢复；语言、窗口边界、安装器、自动更新、签名、公证及发布回滚仍未完成。
- 本地仅完成 Windows 10 构建和烟测；Windows 11、macOS 与 Linux 实机兼容性需要继续验收。

## 快速开始

### 前置条件

- Node.js `>= 20.19.0` 与 npm；
- Rust stable 与 Cargo；
- 当前平台对应的 [Tauri 开发前置条件](https://v2.tauri.app/start/prerequisites/)。

以下命令均在项目根目录执行。

```bash
npm ci
```

启动浏览器界面。该模式使用本地模拟额度，不需要 Codex App Server：

```bash
npm run dev
```

启动 Tauri 桌面开发版本：

```bash
npm run tauri dev
```

运行前端完整检查：

```bash
npm run check
```

运行 Rust 检查与测试：

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features
```

构建不生成安装包的 Tauri debug 可执行文件：

```bash
npm run tauri -- build --debug --no-bundle
```

### 连接本机 Codex

先按照 [Codex CLI 官方说明](https://help.openai.com/en/articles/11096431) 安装并登录 Codex。调试构建可通过 `QUOTAGLANCE_CODEX_PATH` 指定 Codex 可执行文件，该值必须是**绝对文件路径**：

```powershell
$env:QUOTAGLANCE_CODEX_PATH = "C:\Tools\codex\codex.exe"
npm run tauri dev
```

macOS/Linux 可使用同名环境变量：

```bash
export QUOTAGLANCE_CODEX_PATH="$HOME/.local/bin/codex"
npm run tauri dev
```

未设置变量时，debug 构建会尝试从 `PATH` 查找 `codex`。Release 构建不会接受自定义环境变量覆盖，但会从系统 `PATH` 和受控的常见安装目录查找已安装 CLI；正式分发仍需完成固定版本兼容矩阵。不要把 Token、API Key 或 `auth.json` 内容写入 `.env`。

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
- [贡献指南](CONTRIBUTING.md)
- [安全漏洞报告](SECURITY.md)

参考项目 [change-42-yhmm/quota-float](https://github.com/change-42-yhmm/quota-float) 仅用于理解 Tauri 桌面壳、浮动窗口与托盘交互。QuotaGlance 不采用其凭据读取方式、私有 HTTP 接口或固定额度窗口模型。

## 安全与隐私

- WebView 不直接访问 Codex 远端服务。
- 前端不获得通用文件系统、Shell 或任意网络权限。
- 日志、错误和诊断信息不得包含 Token、账号 ID、原始协议响应或个人路径。
- 刷新失败时保留最后一次成功快照，不把未知值显示为 `0%`。
- 默认不启用遥测，不上传额度历史、账号信息、提示词或项目内容。

详见 [安全设计](docs/security.md) 和 [隐私说明](docs/privacy.md)。

## 参与贡献

欢迎提交 Issue、功能建议、文档改进和 Pull Request。开始前请阅读 [贡献指南](CONTRIBUTING.md) 与 [行为准则](CODE_OF_CONDUCT.md)。适合首次参与的方向包括：

- Windows 11、macOS 和主流 Linux 发行版的安装与实机反馈；
- Codex CLI/App Server 版本兼容性验证；
- 无障碍、国际化、主题和桌面交互改进；
- 自动化测试、签名、公证和安全审计。

所有提交都需要通过前端检查、Rust 格式检查、Clippy 和测试。参与者会由 GitHub 的 [Contributors](https://github.com/qingyu6688/QuotaGlance/graphs/contributors) 页面自动记录。

## 品牌与许可

QuotaGlance 是独立第三方工具，与 OpenAI 不存在隶属、授权或背书关系。项目采用 [MIT License](LICENSE)。若后续安装包随附 Codex App Server 二进制，发布前必须完成再分发审查，并保留相应许可、NOTICE、固定版本和来源校验信息。

## 维护

维护邮箱：maorongkang@gmail.com
