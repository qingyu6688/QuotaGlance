# QuotaGlance

[![CI](https://github.com/qingyu6688/QuotaGlance/actions/workflows/ci.yml/badge.svg)](https://github.com/qingyu6688/QuotaGlance/actions/workflows/ci.yml)
[![Release](https://github.com/qingyu6688/QuotaGlance/actions/workflows/release.yml/badge.svg)](https://github.com/qingyu6688/QuotaGlance/actions/workflows/release.yml)
[![GitHub Release](https://img.shields.io/github/v/release/qingyu6688/QuotaGlance?include_prereleases)](https://github.com/qingyu6688/QuotaGlance/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-2ea44f.svg)](LICENSE)
[![Platforms](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-2878ff)](#下载)

> 额度一眼可见，编码不再撞线。

QuotaGlance 是一款面向 Codex 用户的本地桌面额度助手。它用展开卡片和可拖拽悬浮球显示周额度剩余比例、重置时间与数据状态，让额度信息始终保持在手边。

![QuotaGlance 发光额度卡片与动态水面悬浮球](docs/design-assets/quotaglance-ui-concept-v1.png)

## 功能亮点

- **只看周额度**：界面聚焦当前周额度，不再展示已经取消的五小时额度。
- **动态水面悬浮球**：水位随剩余额度变化，水面带有轻微晃动、回弹和主题光效。
- **自由拖动**：按住悬浮球左键即可移动；5px 启动阈值可避免轻点误触。
- **快捷交互**：双击悬浮球展开或收起额度卡片；右键菜单仅保留“设置”和“退出”。
- **七套主题**：支持跟随系统、极光、石墨、纸白、日落珊瑚、蜂蜜琥珀和玫瑰铜夜。
- **本地优先**：通过本机 Codex App Server 只读获取额度，不读取 `auth.json`，不持有 Token。
- **跨平台发布**：提供 Windows、macOS 和 Linux 安装包，并附带 SHA-256 校验清单。

## 下载

当前版本：**[`v0.1.2`](https://github.com/qingyu6688/QuotaGlance/releases/tag/v0.1.2)**

请前往 [GitHub Releases](https://github.com/qingyu6688/QuotaGlance/releases) 下载与系统和架构匹配的文件。

| 平台 | 架构 | 安装包 |
|---|---|---|
| Windows | x64 | `windows_x64-setup.exe` 或 `windows_x64.msi` |
| macOS | Apple Silicon | `darwin_aarch64.dmg` |
| macOS | Intel | `darwin_x64.dmg` |
| Linux | x64 | `linux_amd64.AppImage` 或 `linux_amd64.deb` |
| Linux | ARM64 | `linux_aarch64.AppImage` 或 `linux_arm64.deb` |

发布页同时提供 `SHA256SUMS.txt`，可用于验证下载文件是否完整。

> [!IMPORTANT]
> 当前版本仍是未签名的公开预览版。Windows 首次运行可能显示 SmartScreen 提示；macOS 产物尚未公证；Linux 尚未覆盖所有发行版和桌面环境。请仅从本仓库 Release 页面下载。

## 使用方法

1. 在本机安装并登录 Codex。
2. 安装并启动 QuotaGlance。
3. 等待应用连接本机 Codex App Server，额度卡片会自动更新。
4. 使用卡片右上角的按钮刷新额度、切换置顶状态或打开设置。

悬浮球支持以下操作：

| 操作 | 结果 |
|---|---|
| 按住左键拖动 | 移动悬浮球 |
| 双击 | 展开或收起额度卡片 |
| 右键 | 打开“设置 / 退出”菜单 |

在设置页面可以切换主题、始终置顶和开机启动。主题偏好会保存在本机，重新启动后自动恢复。

### Codex 发现规则

- Windows 会优先发现 Codex 桌面应用的受管运行时。
- macOS 和 Linux 会从 `PATH` 及常见用户安装目录查找 `codex`。
- QuotaGlance 不捆绑或重新分发 Codex App Server，也不会读取 Codex 的认证文件。

如果应用没有找到 Codex，请先确认命令行中可以正常运行：

```bash
codex --version
```

## 本地开发

需要准备：

- Node.js `>= 20.19.0`
- npm
- Rust stable 与 Cargo
- 当前平台对应的 [Tauri 2 开发前置条件](https://v2.tauri.app/start/prerequisites/)

安装依赖并启动浏览器预览：

```bash
npm ci
npm run dev
```

浏览器预览使用模拟额度数据。连接真实本机 Codex 需要启动 Tauri 桌面版本：

```bash
npm run tauri dev
```

运行完整检查：

```bash
npm run check
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features
```

构建当前平台安装包：

```bash
npm run tauri build
```

技术栈：Tauri 2、React、TypeScript、Vite、Rust、Vitest。

## 安全与隐私

- 所有额度读取都在本机完成，前端不直接访问 Codex 远端服务。
- 应用不读取 `auth.json`，不访问系统凭据库，也不保存 Token 或 API Key。
- 应用只调用账号和额度只读接口，不执行登录、登出、购买额度等写操作。
- 默认不启用遥测，不上传额度历史、提示词、账号信息或项目内容。
- 刷新失败时会保留最后一次成功快照，不会把未知状态误显示为 `0%`。

发现安全问题时，请不要创建公开 Issue，按照 [安全策略](SECURITY.md) 中的方式联系维护者。

## 参与贡献

欢迎提交 Bug、平台兼容性反馈、界面改进和 Pull Request。贡献前请阅读 [贡献指南](CONTRIBUTING.md) 与 [行为准则](CODE_OF_CONDUCT.md)。

目前尤其需要以下帮助：

- Windows 11、macOS 和主流 Linux 发行版实机验证；
- Codex CLI / App Server 版本兼容性测试；
- 安装包签名、macOS 公证和发布流程改进；
- 无障碍、国际化和主题细节优化。

## 许可与声明

项目采用 [MIT License](LICENSE) 开源。

QuotaGlance 是独立第三方工具，与 OpenAI 不存在隶属、授权或背书关系。任何 Codex 相关商标与产品名称归其各自权利人所有。

维护邮箱：maorongkang@gmail.com
