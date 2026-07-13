# QuotaGlance 开发环境说明

> 文档状态：`0.1.0` 工程基线  
> 核对日期：2026-07-13  
> 当前阶段：M0/M1 实施  
> 文档维护：maorongkang@gmail.com

## 1. 当前结论

QuotaGlance 已建立 Tauri 2、Rust、React、TypeScript 与 Vite 工程。当前 Windows 10 开发机已完成以下验证：

- `npm ci` 可安装锁定依赖；
- 前端 lint、类型检查、14 项测试和生产构建通过；
- Rust 45 项测试通过，其中包含 9 项假 App Server 跨进程契约，以及 Windows 受管运行时与跨平台 `PATH` 定位测试；
- Tauri debug `--no-bundle` 构建通过；
- Tauri 进程烟测通过；
- 浏览器模式的布局与交互 QA 通过。

这些结果只说明 `0.1.0` 工程骨架在当前开发机可用，不代表正式发布条件已经满足。生产构建尚无 bundled Codex App Server sidecar；Windows 11、macOS、安装器、代码签名和 macOS 公证均未完成验证。

MVP 不使用 MySQL，也不启动本地 REST 服务。额度查询由 Rust 进程通过 `stdio` JSONL 与 Codex App Server 通信。

## 2. 当前 Windows 开发机

| 项目 | 已核对结果 | 用途或说明 |
|---|---|---|
| 操作系统 | Microsoft Windows 10 专业版 10.0.19045，64 位 | 仅作当前开发机；不是 1.0 正式支持验收结论 |
| Node.js | 24.3.0 | 前端工具链，项目最低要求为 `>= 20.19.0` |
| npm | 11.4.2 | 包管理器，使用 `package-lock.json` 锁定依赖 |
| pnpm | 未安装 | 当前工程统一使用 npm |
| Java | 17.0.8 LTS | 项目运行和构建不依赖 Java |
| Python | 3.12.3 | 可用于辅助脚本，不是应用运行时 |
| pip | 25.2 | Python 包管理器 |
| Git | 2.47.1.windows.1 | 版本管理 |
| Rust | 1.96.0 stable | Tauri 后端和桌面能力；清单最低版本为 1.85 |
| Cargo | 1.96.0 | Rust 构建与测试 |
| MySQL | 8.0.46 | 当前项目 MVP 不使用 |

环境复核命令：

```powershell
node -v
npm -v
pnpm -v
java -version
python --version
pip --version
git --version
rustc --version
cargo --version
mysql --version
```

Tauri debug 构建已证明当前机器具备本轮所需的基础 Windows 编译条件，但尚未记录 Microsoft C++ Build Tools、Windows SDK 和 WebView2 的精确版本。NSIS、WiX、Windows 代码签名、时间戳服务、SmartScreen，以及 Windows 11 上的托盘、多屏、DPI 和休眠恢复仍需单独核对。

## 3. 安装与启动

以下命令均在项目根目录执行。

### 3.1 安装依赖

```powershell
npm ci
```

### 3.2 浏览器开发模式

```powershell
npm run dev
```

Vite 默认监听 `http://localhost:1420`。浏览器模式使用本地模拟数据，用于界面开发和 QA，不会读取真实 Codex 额度。

### 3.3 Tauri 桌面开发模式

```powershell
npm run tauri dev
```

debug 模式会使用显式配置的 Codex 可执行文件，或尝试从 `PATH` 查找 `codex`。未找到合法可执行文件时，界面应显示受控错误，不能绕过系统权限或读取 Codex 桌面应用内部文件。

## 4. 构建与测试命令

### 4.1 前端

```powershell
npm run lint
npm test
npm run build
```

也可以一次运行全部前端检查：

```powershell
npm run check
```

当前记录：14 项前端测试通过，lint、类型检查和生产构建通过。

### 4.2 Rust

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features
```

当前记录：45 项 Rust 测试通过。`test-support` 特性只用于构建假 App Server 和跨进程契约，默认 Tauri 构建不会携带该测试程序。

### 4.3 Tauri debug 构建

```powershell
npm run tauri -- build --debug --no-bundle
```

该命令已在当前 Windows 10 开发机通过。`--no-bundle` 不生成安装包，因此不能作为安装、签名或发布验收证据。

## 5. 开发端口

| 场景 | 端口 | 约束 |
|---|---:|---|
| Vite 开发服务器 | `1420` | `strictPort` 已启用；端口占用时直接失败 |
| Vite HMR | `1421` | 仅在配置 `TAURI_DEV_HOST` 的 Tauri 开发场景使用 |
| Tauri 生产应用 | 无 | 生产应用不启动本地 Web 服务 |
| Codex App Server | 无 | 使用子进程标准输入/输出，不监听网络端口 |

开发服务器默认不应向局域网公开。如需修改端口，必须同时更新 Vite 和 Tauri 开发配置。

## 6. 环境变量

### 6.1 `QUOTAGLANCE_CODEX_PATH`

该变量用于在 **debug 构建**中显式指定 Codex 可执行文件：

```powershell
$env:QUOTAGLANCE_CODEX_PATH = "C:\Tools\codex\codex.exe"
npm run tauri dev
```

约束如下：

- 值必须是绝对路径；
- 路径必须指向现有普通文件，不能只填目录；
- 该变量仅在 debug 构建中生效；
- release 构建会忽略该变量；
- 不得把 WindowsApps 内部文件复制出来或修改其权限来绕过访问限制；
- 路径可能包含个人信息，不应写入日志、提交记录或共享诊断材料。

未设置时，Windows 构建会先查找 Codex 桌面应用在 `%LOCALAPPDATA%\OpenAI\Codex\bin\<managed-id>` 下管理的运行副本，再从 `PATH` 查找 `codex.exe`；macOS/Linux 会从 `PATH`、`/usr/local/bin`、`/opt/homebrew/bin`、`~/.local/bin` 和 `~/.npm-global/bin` 查找 `codex`。候选路径会规范化并校验为普通可执行文件。当前公开 Release 仍不包含 bundled sidecar，正式 `1.0.0` 前必须完成固定版本、来源、哈希、许可和签名方案。

### 6.2 其他非敏感变量

| 变量 | 是否必需 | 用途 | 注意事项 |
|---|---|---|---|
| `CODEX_HOME` | 可选 | Codex App Server 使用的配置目录 | 路径可能包含用户名，诊断输出必须脱敏 |
| `TAURI_DEV_HOST` | 可选 | Tauri 开发服务器主机和 HMR 配置 | 仅用于受控开发环境，不应无意公开到局域网 |
| `RUST_LOG` | 开发可选 | 控制 Rust 日志级别 | 不得输出协议原文、Token 或账号数据 |
| `RUST_BACKTRACE` | 开发可选 | 本地诊断 Rust 异常 | 生产默认关闭；共享前必须删除个人路径 |

## 7. 敏感信息规则

以下内容不得写入项目 `.env`、源码、文档或提交记录：

- Codex access token、API Key、刷新令牌和 `auth.json` 内容；
- Apple 证书、App Store Connect 私钥或 Apple 专用密码；
- Windows 代码签名证书私钥、PIN 或云签名凭据；
- Tauri updater 私钥及其密码；
- 真实账号响应、账号 ID 或包含个人路径的诊断样本。

签名和公证密钥只允许进入受保护的 CI Secret、硬件密钥、可信签名服务或离线备份。`.env` 已列入忽略规则，但这不意味着可以在其中保存 Codex 凭据。

## 8. Codex App Server 现状

当前实现会启动并复用一个只读 `codex app-server` 会话：每个连接只完成一次初始化，后续通过 pending request map 读取账号状态和额度。协议限制、消息大小、乱序响应、迟到响应、通知、超时和错误映射已有自动化测试。

仍待实施的能力包括：

- 随应用分发的受控 bundled sidecar；
- 进程崩溃后的指数退避与自动重建；
- 可见/隐藏定时安全重同步；
- 崩溃重启、退避和兼容性检查；
- sidecar 固定版本、哈希、许可、签名和多架构产物；
- Windows 11 与 macOS 实机联调。

2026-07-13 已使用当前登录账号和 `codex-cli 0.144.0-alpha.4` 完成真实只读 POC，确认 ChatGPT Pro、两个动态额度桶、主/次窗口、Credits 和 banked reset 字段可读取。测试过程未读取或保存 `auth.json`、Token、Cookie、邮箱或账号 ID；具体额度值不写入文档，避免形成额度历史。

该结果证明当前 Windows 机器可通过外部已安装 Codex 工作，不代表版本兼容范围、`file/keyring/auto` 矩阵、macOS 或 sidecar 再分发已经验收。

## 9. macOS 待核对项

当前没有真实 macOS 构建或运行证据。以下内容全部保持待验证：

| 待核对项 | 建议证据 |
|---|---|
| macOS 版本与硬件架构 | `sw_vers`、`uname -m` |
| Xcode 与 Command Line Tools | `xcodebuild -version`、`xcode-select -p` |
| macOS SDK | `xcrun --sdk macosx --show-sdk-version` |
| Node.js、npm、Rust、Cargo | 对应 `--version` 命令 |
| Intel 与 Apple Silicon 构建 | 两个目标架构的构建日志或 Universal 产物 |
| 签名身份 | `security find-identity -v -p codesigning` |
| 公证与 stapling | `notarytool` 和 `stapler` 验证结果 |
| 安装、托盘、多屏和休眠恢复 | 目标系统实机测试记录 |

在上述验证完成前，只能将 macOS 标记为未签名、未公证的社区预览，不得宣称已经达到正式支持或商店分发标准。

## 10. 下一阶段环境任务

- [x] 建立 npm 锁文件并验证干净安装。
- [x] 验证前端 lint、测试和构建。
- [x] 验证 Rust 测试。
- [x] 验证 Tauri debug `--no-bundle` 构建和进程烟测。
- [x] 验证开发服务器固定端口和浏览器界面。
- [ ] 记录 Windows 11 验收机的完整工具链与实机结果。
- [ ] 在 macOS Intel 与 Apple Silicon 环境完成构建和运行验证。
- [ ] 完成 Codex App Server sidecar 的固定版本、来源、哈希与许可审查。
- [ ] 验证 Windows 与 macOS 安装器、签名、公证和更新流程。
- [ ] 将最终平台结果同步到测试、部署和发布文档。

## 11. 官方参考

- [Tauri 开发前置条件](https://v2.tauri.app/start/prerequisites/)
- [Tauri Windows 安装器](https://v2.tauri.app/distribute/windows-installer/)
- [Codex App Server](https://developers.openai.com/codex/app-server)
- [Codex 认证与凭据存储](https://developers.openai.com/codex/auth)
