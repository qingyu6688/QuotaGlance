# QuotaGlance 构建、签名与发布说明

> 文档状态：发布方案初稿  
> 核对日期：2026-07-12  
> 当前阶段：0.1.0 可执行本地调试构建，不能作为正式版本发布  
> 文档维护：maorongkang@gmail.com

## 1. 当前状态

仓库当前已有 Tauri 2、React/TypeScript 工程、前后端锁文件、严格检查脚本和基础 CI，并已生成 Windows x64 未签名 Release 预览 EXE、NSIS 与 MSI。当前产物不包含可合法分发的 production sidecar，也未完成 Windows 11 安装/升级/卸载、更新器、macOS 构建、平台签名或公证验收，因此不能据此声称已有正式发布版。

正式发布必须由受控 CI 或专用构建机完成。本地开发包只用于调试，不得作为公开下载提供。

在项目根目录可执行当前开发基线检查和不打包调试构建：

```powershell
npm ci
npm run check
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features
npm run tauri -- build --debug --no-bundle
```

该命令只用于验证应用主程序可以构建，不生成可交付安装包，也不验证真实账号、production sidecar、签名或跨平台兼容性。

## 2. 目标产物

| 平台 | 1.0 目标 | 主要产物 | 补充产物 |
|---|---|---|---|
| Windows | Windows 11 x64 | 已签名 NSIS `setup.exe` | 已签名 MSI，供企业部署评估 |
| Windows 10 | 22H2 且仍获 ESU 的设备 | 尽力兼容，不单独承诺 | 在测试报告中明确结果 |
| macOS | macOS 13+，Intel 与 Apple Silicon | 已签名、公证并 stapling 的 DMG | Universal 优先；必要时分架构 DMG |

Windows ARM64 计划在 1.1.x 评估，不得把未实测产物列为 1.0 正式支持。macOS 的 Universal 构建必须在 macOS 构建机或 macOS CI runner 上完成。

每个发布目标都应同时生成：

- 用户安装包；
- Tauri updater 产物及 `.sig`；
- SHA-256 校验清单；
- SBOM 或依赖清单；
- 第三方许可和 NOTICE；
- 对应版本的测试报告与 changelog。

## 3. 版本与发布约定

- 应用版本使用语义化版本，例如 `1.0.0`；
- `tauri.conf.json`、前端包信息、Cargo 包信息和更新清单必须保持一致；
- 正式发布由受保护的 `vX.Y.Z` 标签触发；
- 标签必须指向已经通过 Pull Request 检查的提交；
- 同一个版本号的产物一旦公开，不得替换二进制内容；
- 重新构建必须递增版本号，并在 changelog 中说明原因；
- 1.0 只提供 Stable 通道，不做静默强制更新。

## 4. Codex App Server sidecar 管理

### 4.1 来源与固定版本

1.0 优先随应用携带固定版本的 Codex App Server。不得：

- 从 Codex 桌面应用的 WindowsApps 内部目录取用二进制；
- 在构建时下载“latest”或未固定提交的产物；
- 从不受信任的镜像、个人网盘或临时 URL 获取；
- 仅凭文件名判断版本或架构；
- 在运行时用 Shell 字符串拼接 sidecar 路径和参数。

发布前应维护一份可机器校验的 sidecar 锁定记录。实际文件名在工程初始化后确定，至少包含：

```json
{
  "name": "codex-app-server",
  "version": "待固定",
  "source": "待记录的官方来源或源码提交",
  "license": "发布前重新核对",
  "artifacts": [
    {
      "target": "x86_64-pc-windows-msvc",
      "sha256": "待填写"
    },
    {
      "target": "x86_64-apple-darwin",
      "sha256": "待填写"
    },
    {
      "target": "aarch64-apple-darwin",
      "sha256": "待填写"
    }
  ]
}
```

当前 OpenAI Codex 官方仓库采用 Apache-2.0，但每次升级 sidecar 都必须重新检查目标版本的仓库许可、发行附件和 NOTICE，不能永久依赖本文件的结论。随包分发时保留适用的 LICENSE、NOTICE 和归属信息，并记录二进制与源码版本的对应关系。

### 4.2 构建期校验

CI 在打包前必须完成：

1. 从固定来源获取或从固定提交构建；
2. 校验 SHA-256，与锁定记录逐字匹配；
3. 校验目标三元组与应用架构一致；
4. 执行版本探测和最小 `initialize` 握手；
5. 核对只使用稳定协议能力，1.0 不启用 `experimentalApi`；
6. 扫描恶意软件和已知漏洞；
7. 检查 LICENSE、NOTICE 是否进入最终安装包；
8. 生成最终产物清单，并再次计算安装包内二进制的哈希。

任意一步失败都应停止构建，不允许退回 PATH 中的任意 `codex` 继续发布。

### 4.3 运行时约束

- 默认只从应用资源目录中的固定位置启动 sidecar；
- 参数固定为 `app-server`，使用默认 `stdio` JSONL；
- 不经过 `cmd.exe`、PowerShell、`sh` 或 `bash`；
- 用户选择外部 Codex CLI 时，必须规范化路径、校验文件类型、版本和兼容性；
- sidecar 缺失、被替换或版本不兼容时给出修复提示，不自动读取 `auth.json` 降级；
- 应用升级与 sidecar 升级作为同一原子版本发布。

## 5. 构建机与密钥准备

### 5.1 通用要求

- 干净、可复现的 Windows 与 macOS runner；
- 固定 Node.js、npm、Rust、Tauri CLI 和依赖锁文件；
- 只使用 `npm ci`，正式构建不隐式改写锁文件；
- 构建任务使用最小仓库权限；
- 未受信任的 Pull Request 不能访问签名、公证和发布 Secret；
- 日志禁止输出证书、私钥、密码、Token 和完整环境变量；
- 构建产物从构建到签名、上传全程保留哈希记录。

### 5.2 Windows 签名材料

Windows 正式包使用 Authenticode。实际方案需在 M0/M1 阶段从硬件保护证书、可信云签名服务或 Tauri 支持的自定义 `signCommand` 中确定。无论采用哪种方案，都必须：

- 私钥不可导出或只保存在受保护的签名环境；
- CI 仅获得执行签名所需的短期权限；
- 使用可信时间戳；
- 签名应用主程序、随包 sidecar 和最终安装器；
- 验证签名主题、证书链、时间戳和文件哈希；
- 不把 PFX、密码、PIN 或云服务密钥提交到仓库。

### 5.3 macOS 签名与公证材料

需要 Apple Developer Program、Developer ID Application 身份和公证凭据。Tauri 支持通过 App Store Connect API 或 Apple ID 进行公证。CI 中只选择一种经过验证的方式。

常见变量名称如下，只记录名称，不在文档、仓库或本地 `.env` 中保存值：

- `APPLE_SIGNING_IDENTITY`；
- App Store Connect 方式：`APPLE_API_ISSUER`、`APPLE_API_KEY`、`APPLE_API_KEY_PATH`；
- Apple ID 方式：`APPLE_ID`、`APPLE_PASSWORD`、`APPLE_TEAM_ID`；
- CI 导入证书时所需的受保护证书和临时 keychain 密码。

### 5.4 Tauri updater 密钥

更新签名与 Windows/macOS 系统代码签名是两套独立机制。

- 公钥写入应用配置，用于客户端校验；
- 私钥只用于 CI 生成更新签名；
- 使用 `TAURI_SIGNING_PRIVATE_KEY` 和可选的 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`；
- 私钥保存在 CI Secret 和加密离线备份中；
- Tauri 官方说明 updater 私钥不从 `.env` 读取；
- 丢失私钥会导致已安装客户端无法信任后续更新，因此必须验证备份可恢复；
- 公私钥轮换必须提前设计，不能在私钥丢失后临时覆盖客户端内置公钥。

## 6. Pull Request 流水线

未签名的 Pull Request 流水线计划执行：

```text
检出固定提交
→ npm ci
→ TypeScript 类型检查、lint、测试、构建
→ cargo fmt --check
→ cargo clippy -- -D warnings
→ cargo test
→ secret scan 与依赖审计
→ sidecar 锁定记录、哈希、架构、许可检查
→ Windows x64 桌面构建
→ macOS Universal 或双架构桌面构建
→ 上传短期 QA 产物
```

Pull Request 构建不使用正式签名私钥，也不能自动发布 Release。来自 fork 的代码不得运行在持有发布 Secret 的工作流中。

## 7. Windows 构建与验证

### 7.1 构建

Windows 安装器应在 Windows runner 上构建。NSIS 是普通用户的主要产物，MSI 作为企业部署补充。Tauri 官方文档说明 MSI 依赖 WiX，并且 MSI 构建只能在 Windows 上完成。

当前工程的检查命令如下；正式安装器命令仍须在签名、sidecar 和发布配置完成后重新验证：

```powershell
# 在项目根目录执行
npm ci
npm run lint
npm test -- --run
cargo fmt --check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features

# 示例：生成 Windows 安装器
npm run tauri -- build --bundles nsis,msi
```

WebView2 采用 Evergreen 策略还是离线运行时，必须在企业离线场景测试后写入配置。不能假定所有目标机器都已有可用版本。

### 7.2 签名顺序

建议顺序：

1. 校验并签名 sidecar；
2. 构建并签名应用主程序及需要签名的动态库；
3. 生成安装器；
4. 签名 NSIS/MSI；
5. 重新验证所有嵌套文件和最终安装器；
6. 在干净 Windows 11 x64 环境执行安装冒烟测试。

### 7.3 验证

```powershell
# 示例，实际参数按签名方案确定
Get-AuthenticodeSignature .\path\to\QuotaGlance.exe
Get-AuthenticodeSignature .\path\to\codex-sidecar.exe
Get-AuthenticodeSignature .\path\to\QuotaGlance-setup.exe

Get-FileHash .\path\to\QuotaGlance-setup.exe -Algorithm SHA256
```

如果 CI 安装了 Windows SDK，还应使用 `signtool verify /pa /all /v <file>` 检查证书链和时间戳。验证结果必须保存为发布证据，但日志中不得包含 Secret。

## 8. macOS 构建、签名与公证

### 8.1 架构策略

首选同时构建应用和 sidecar 的 Universal 二进制。若 sidecar 无法稳定合并或 Universal 产物签名、公证失败，则分别构建：

- `x86_64-apple-darwin`，面向 Intel；
- `aarch64-apple-darwin`，面向 Apple Silicon。

不得将只含单一架构的二进制标记为 Universal。使用 `lipo -info` 或 `file` 验证应用主程序和每个 sidecar。

### 8.2 预期构建命令

```bash
# 在项目根目录执行；工程初始化后再验证
npm ci
npm run lint
npm test -- --run
cargo fmt --check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features

# 示例：Universal DMG
npm run tauri -- build --target universal-apple-darwin --bundles dmg
```

### 8.3 签名与公证

嵌套可执行文件应先签名，再签名外层应用包。sidecar 必须包含在签名检查范围内。正式发布使用 Developer ID Application、Hardened Runtime 和经审查的最小 entitlements。

Tauri 获取 Apple 公证凭据后可在构建/打包流程中提交公证并 stapling。公证成功不代替本地验证。

```bash
# 示例验证命令，路径以实际产物为准
codesign --verify --deep --strict --verbose=2 "/path/to/QuotaGlance.app"
codesign -dv --verbose=4 "/path/to/QuotaGlance.app"
spctl --assess --type execute --verbose=4 "/path/to/QuotaGlance.app"
xcrun stapler validate "/path/to/QuotaGlance.app"
xcrun stapler validate "/path/to/QuotaGlance.dmg"
shasum -a 256 "/path/to/QuotaGlance.dmg"
```

还应把 DMG 下载到未导入开发证书的干净 Mac，验证 Gatekeeper、首次启动、托盘、sidecar 启动和退出清理。

## 9. 自动更新

### 9.1 信任链

更新必须同时满足：

1. 通过 HTTPS 获取更新元数据和产物；
2. 元数据中的目标平台、版本、URL 与签名完整；
3. Tauri updater 使用内置公钥验证产物签名；
4. 下载的 Windows/macOS 应用仍具有有效系统代码签名；
5. sidecar 版本与应用版本对应，不能跨版本拼装。

Tauri 的静态更新 JSON 中，`signature` 必须是 `.sig` 文件内容，而不是签名文件 URL。所有已列出的平台项都要完整有效。

### 9.2 用户策略

- 应用启动稳定后延迟检查一次，此后每天最多自动检查一次；
- 默认只提示更新，由用户确认下载、安装和重启；
- 更新失败不影响当前版本的额度查询；
- 1.0 只使用 Stable 通道；
- 默认拒绝降级，不开启 `allowDowngrades`；
- 更新界面展示版本、发布日期和自然语言变更摘要；
- 不把 Token、账号信息或额度值作为更新请求参数。

### 9.3 更新验收

- 从上一正式版升级到候选版；
- 无更新时正确处理空结果；
- 元数据缺字段、错误平台、错误版本和错误签名；
- 下载中断、磁盘空间不足、代理异常和超时；
- 安装前退出与重启恢复；
- 更新后应用版本、sidecar 版本和哈希一致；
- Windows 与 macOS 分别完成签名复验。

## 10. 正式发布流程

### 10.1 开发预览产物同步规则

在项目进入正式版本发布前，工作区内每次代码、样式、配置或正式文档修改完成后，都必须同步更新 `release/QuotaGlance-0.1.0-windows-x64-preview/`：

1. 运行前端检查和生产构建；
2. 重新执行 Tauri Release 构建；
3. 替换优化版 EXE、NSIS 安装器和 MSI 安装器；
4. 重新计算并更新 `SHA256SUMS.txt`；
5. 同步更新 `RELEASE-NOTES.md` 中的检查结果和限制；
6. 至少完成一次主程序启动、App Server 子进程和退出回收烟测。

开发预览目录允许在版本号尚未递增时覆盖本地产物，但不得将被覆盖的产物对外宣称为不可变正式 Release。创建公开版本、Git 标签或更新清单后，必须遵守后文的版本不可覆盖规则，并通过更高版本号发布修复。

```text
冻结版本与 changelog
→ 受保护标签触发独立平台构建
→ 固定并校验 sidecar
→ 执行完整自动化测试与安全扫描
→ Windows Authenticode 签名
→ macOS Developer ID 签名、公证、stapling
→ 生成并签名 Tauri updater 产物
→ 计算 SHA-256、生成许可清单与测试报告
→ 在干净设备执行安装和升级冒烟测试
→ 创建草稿 Release
→ 两人或维护者复核产物、签名、更新清单
→ 发布 Stable
→ 观察更新与启动错误
```

Release 页面必须明确：

- 支持的平台和架构；
- 安装包文件名、大小和 SHA-256；
- 是否为正式签名与公证产物；
- App Server 固定版本及第三方许可入口；
- 已知问题和回滚/恢复方式；
- QuotaGlance 是独立第三方工具，与 OpenAI 无隶属、授权或背书关系。

## 11. 回滚与故障恢复

### 11.1 发布前

发现阻断问题时直接停止草稿发布，撤销更新清单上传，不创建或移动正式标签。修复后使用新的候选构建；不得在相同版本号下替换产物。

### 11.2 发布后尚未大规模安装

1. 立即从更新清单撤下问题版本，使未更新用户不再收到它；
2. 暂停 Release 的推广入口，但保留内部取证所需产物和哈希；
3. 评估是否存在安全事件，必要时进入安全事件响应；
4. 从最后稳定提交创建更高补丁版本，例如用 `1.0.2` 恢复 `1.0.0` 的稳定代码；
5. 重新完成全部签名、公证、更新和安装测试。

正常回滚采用“向前修复”，不向已安装客户端推送较低版本。这样可保留默认的防降级检查，也避免版本状态混乱。

### 11.3 已安装版本无法启动

- 发布系统签名的完整安装器，允许用户覆盖安装更高补丁版本；
- 在用户手册提供保留或清除本地偏好的明确步骤；
- 不要求用户关闭 Gatekeeper、SmartScreen 或禁用签名校验；
- 不要求用户手工替换 sidecar；
- 确认修复包能从损坏版本覆盖升级，并保留必要的设置。

### 11.4 密钥或供应链事件

- updater 私钥疑似泄露：立即撤下更新清单、隔离密钥、保存日志并评估已发布签名；旧客户端无法凭空信任新公钥，必要时通过仍可信的旧密钥更新或系统签名完整安装器迁移；
- Windows/macOS 签名身份疑似泄露：联系证书颁发方或 Apple 撤销，停止发布，重新建立签名链；
- sidecar 来源或哈希异常：停止所有包含该二进制的下载，确定受影响版本，发布更高版本替换并公开说明；
- 不删除取证材料，不用新产物覆盖旧版本文件。

## 12. 产物保留与审计

至少保留：

- 每个正式版本的源代码提交和标签；
- 原始构建清单、依赖锁文件和 sidecar 锁定记录；
- 未签名前与签名后产物哈希；
- 签名、公证、stapling 和 Gatekeeper/SmartScreen 验证证据；
- updater JSON、签名和发布时的公钥指纹；
- LICENSE、NOTICE、SBOM、测试报告和发布审批记录；
- 最近两个稳定版本的完整安装器，供灾难恢复。

保留期和访问权限在 CI/CD 落地后确定。签名私钥不得作为普通构建 artifact 保存。

## 13. 发布检查清单

- [ ] 版本号、标签、changelog 一致；
- [ ] 依赖锁文件未在构建中变化；
- [ ] 全部自动化检查通过；
- [ ] sidecar 来源、版本、架构、哈希和许可复核通过；
- [ ] Windows 主程序、sidecar、NSIS/MSI 签名有效；
- [ ] macOS 应用、sidecar、DMG 签名、公证和 stapling 有效；
- [ ] 两个 macOS 架构均已实测；
- [ ] updater 私钥来自受保护环境，签名验证通过；
- [ ] 更新篡改与失败恢复测试通过；
- [ ] 安装、覆盖升级、卸载、单实例和首次启动通过；
- [ ] 隐私、安全、部署、测试、用户手册和 changelog 已更新；
- [ ] 草稿 Release 已人工复核；
- [ ] 已准备停止发布和向前修复方案。

## 14. 官方参考

- [Tauri 分发概览](https://v2.tauri.app/distribute/)
- [Tauri Windows 安装器](https://v2.tauri.app/distribute/windows-installer/)
- [Tauri Windows 代码签名](https://v2.tauri.app/distribute/sign/windows/)
- [Tauri macOS 代码签名与公证](https://v2.tauri.app/distribute/sign/macos/)
- [Tauri macOS 应用包](https://v2.tauri.app/distribute/macos-application-bundle/)
- [Tauri Updater](https://v2.tauri.app/plugin/updater/)
- [Codex App Server](https://developers.openai.com/codex/app-server)
- [OpenAI Codex LICENSE](https://github.com/openai/codex/blob/main/LICENSE)
