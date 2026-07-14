# QuotaGlance 测试计划与测试报告

> 文档状态：测试计划与开发基线报告  
> 核对日期：2026-07-14
> 当前阶段：`0.1.6` 跨平台社区预览；前端自动化检查已通过，尚不满足正式稳定版条件
> 文档维护：maorongkang@gmail.com

## 1. 当前测试状态

QuotaGlance 已形成 Tauri 2、React、TypeScript 与 Rust 的可运行开发基线。2026-07-14 已复核前端检查、Rust 检查、额度展示投影、登录时启动路径、Windows 原生浮球拖拽和 App Server 协议边界，具体结果见第 14 节。

当前结果只证明本工作区开发基线可构建、核心解析与组件测试通过，不等同于正式候选版本验收：

- Windows 当前登录账号的 App Server 只读数据链路已完成 POC；通知、完整认证矩阵、可分发 sidecar 与其他平台仍未验证；
- 尚未执行 macOS/Linux 实机、平台签名、安装器升级和卸载验证；GitHub Runner 构建成功只能作为编译与打包证据；
- `cargo-audit 0.22.2` 已执行：未发现漏洞，记录 17 个允许的维护或不健全警告，仍需持续跟踪；
- 调试可执行文件不是签名安装包，也没有作为候选产物记录 SHA-256；
- 参考项目的测试结果仍不得计入 QuotaGlance 的测试报告。

每次形成新的发布候选版本后，应在本文末尾追加一份包含版本号、提交号、平台、产物哈希和证据链接的测试报告。

## 2. 测试目标

测试优先保证以下行为：

1. 正确区分真实额度、未知状态和旧数据，不用假 `0%` 掩盖错误；
2. 正常路径不读取或暴露 Token、账号标识和原始协议消息；
3. App Server 缺失、过旧、拒绝执行、异常退出或返回畸形数据时可控降级；
4. 通知、定时重同步、缓存和手动刷新不会产生请求风暴；
5. Windows 与 macOS 的窗口、托盘、休眠恢复、签名安装和更新行为可验证；
6. 发布产物中的 sidecar 版本、架构、哈希和许可材料可追溯。
7. 套餐、可选短周期/5 小时额度、周额度、重置机会与到期时间按服务端实际字段展示，不制造缺失数据。

## 3. 分层测试矩阵

| 层级 | 主要对象 | 测试方式 | 是否允许真实账号 |
|---|---|---|---|
| Rust 单元测试 | 解析、领域转换、时间、错误映射、脱敏 | 内存数据和合成 fixture | 否 |
| App Server 契约测试 | JSONL 握手、方法、通知、兼容字段 | 可控假进程或录制后人工脱敏的 fixture | 否 |
| 服务层测试 | 缓存、SingleFlight、退避、调度、状态机 | 假 Provider、可控时钟、确定性随机源 | 否 |
| 前端组件测试 | 快照展示、交互、i18n、可访问性 | jsdom 或等价测试环境 | 否 |
| Tauri IPC 集成测试 | 命令白名单、序列化、事件、权限 | 测试应用和假 Provider | 否 |
| 桌面 E2E | 窗口、托盘、单实例、多屏、休眠恢复 | Windows/macOS 实机或受控虚拟机 | 原则上否 |
| App Server 实机 POC | 官方登录态和额度读取兼容性 | 专用测试环境，人工执行 | 仅经批准的专用账号 |
| 安装与更新测试 | 签名、安装、覆盖升级、回滚恢复 | 正式候选产物 | 否 |
| 安全测试 | 权限、CSP、日志、sidecar、更新供应链 | 静态检查与运行时负向用例 | 否 |

## 4. Rust 单元与契约测试

### 4.1 协议层

- `initialize` 响应后再发送 `initialized`，并校验请求 ID 匹配；
- 请求正常响应、错误响应、超时、重复响应和未知请求 ID；
- 一行一条 JSONL 消息，覆盖空行、畸形 JSON、超长消息和非 UTF-8 输出；
- 未知通知被安全忽略，已知通知不因字段扩展而崩溃；
- 子进程标准错误不进入结构化协议解析；
- App Server 缺失、版本不兼容、无执行权限、异常退出和重启退避；
- `clientInfo.name` 如实使用 `quota_glance`，1.0 不声明 `experimentalApi`。

### 4.2 账号与额度契约

- `account/read` 的未登录、ChatGPT、API Key 及未来未知认证类型；
- 单桶 `rateLimits` 和多桶 `rateLimitsByLimitId`；
- `primary`、`secondary` 和未来新增窗口；
- 缺失 `limitName`、可选 `planType`、空 `credits` 和空 reset credits；
- `usedPercent` 到 `remainingPercent` 的边界值与异常值；
- Unix 秒级时间戳、过期重置时间和时区展示所需转换；
- `account/updated`、`account/rateLimits/updated` 的部分载荷、重复、乱序和合并刷新；
- 只实现读取操作；对 `account/rateLimitResetCredit/consume` 等写操作没有可达调用路径。

### 4.3 领域与错误模型

- 动态 `buckets[]`、`windows[]` 的稳定排序和去重；
- `ok → stale → recovered`、`loading → error` 等状态转换；
- 刷新失败保留最后成功快照，不把已有额度覆盖成零；
- 未知值保持未知，不通过本地使用量推算服务端额度；
- 用户可见错误不包含账号、Token、原始响应或本机绝对路径。

## 5. 服务层与调度测试

- 30 秒缓存命中、过期和主动失效；
- 同类读取的 SingleFlight 并发合并；
- 30 秒内多次手动刷新最多触发一次同类读取；
- 可见时 5 分钟、隐藏时 10 分钟的安全重同步；
- 指数退避、上限、成功后复位和随机抖动；
- 通知风暴的防抖与完整重读；
- 睡眠唤醒、网络离线恢复、系统时间跳变和窗口重新显示；
- App Server 重启期间继续展示最后成功值；
- 更新检查失败不影响额度读取；
- 使用可控时钟和确定性随机源，不用真实长时间等待完成测试。

## 6. 前端测试

### 6.1 展示与交互

- 首次加载、正常、旧数据、离线、未登录、App Server 缺失和协议不兼容；
- 单桶、多桶、可选 credits、缺失次级窗口和未知重置时间；
- 主桶投影中的套餐、短周期/5 小时额度、周额度、重置机会总数及到期时间去重排序；
- 健康、提醒、危险、耗尽和未知状态；
- 浮球、展开卡片、设置页和托盘触发后的状态同步；
- 登录时启动切换、保存失败回滚和设置面板状态同步；
- 浮球右键阻止 WebView 默认菜单，并且原生菜单只包含“设置”和“退出”；
- 刷新冷却、失败回滚和重复点击；
- 简体中文与英文文案、复数、时区和相对时间；
- 跟随系统、极光、石墨、纸白、日落珊瑚、蜂蜜琥珀和玫瑰铜夜主题；
- 空状态、加载状态和错误状态。

### 6.2 可访问性

- 全部交互均可通过键盘完成；
- 焦点顺序稳定，弹层关闭后焦点返回触发点；
- 图标按钮具备可理解名称；
- 状态不只依赖颜色表达；
- 常用文本和控件达到 WCAG AA 对比度目标；
- 屏幕阅读器不会重复朗读装饰性内容；
- 减少动态效果设置得到尊重。

## 7. Fixture 与测试数据规范

计划目录：

```text
tests/
  fixtures/
    app-server/
    legacy/
  e2e/
src-tauri/
  tests/
```

所有 fixture 必须满足：

- 优先手工编写最小合成数据，不直接保存真实响应；
- 不包含 access token、API Key、Cookie、Authorization Header、刷新令牌；
- 不包含真实邮箱、账号 ID、workspace ID、用户名、主目录或项目路径；
-  opaque ID 使用明显的测试值，例如 `test-credit-001`；
- 时间戳固定，测试不得依赖当前真实时间；
- fixture 文件头或相邻说明记录来源场景、脱敏人和复核日期；
- 新增 fixture 必须经过 secret scan 和人工复核；
- 禁止在失败快照、CI artifact 和测试报告中附上原始个人响应。

若实机 POC 必须观察真实响应，只记录字段名称、类型、可选性和脱敏后的最小样例。原始内容在受控会话结束后立即删除，不进入 Git、聊天记录或工单。

## 8. 安全负向用例

| 场景 | 预期结果 |
|---|---|
| WebView 尝试调用未授权 Tauri 命令 | 被 capability 或命令白名单拒绝 |
| WebView 尝试访问通用文件、Shell 或网络能力 | 无相应权限 |
| sidecar 路径包含参数注入字符 | 作为路径校验失败，不经过 Shell 拼接 |
| sidecar 哈希、版本或架构不匹配 | 拒绝启动并显示可操作错误 |
| 单条 JSONL 消息超过上限 | 中止该消息或连接，记录脱敏错误码 |
| 协议输出包含疑似 Token | 不进入日志、前端事件或诊断包 |
| 更新元数据被篡改 | Tauri 签名验证失败，不安装 |
| 更新版本低于当前版本 | 默认拒绝降级 |
| Legacy Provider 收到重定向 | 拒绝跟随 |
| Legacy Provider 访问非 `chatgpt.com` 主机 | 请求在发出前被拒绝 |
| Legacy 响应超过 1 MiB | 停止读取并返回受控错误 |

## 9. 双平台实机矩阵

| 范围 | Windows | macOS |
|---|---|---|
| 正式基线 | Windows 11 x64 | macOS 13+ |
| 尽力支持或补充 | Windows 10 22H2 ESU；ARM64 后续评估 | Intel 与 Apple Silicon 均为 1.0 范围 |
| App Server | 随包 sidecar、外部 CLI、缺失、过旧、拒绝执行、异常退出 | Universal 或分架构 sidecar、外部 CLI、缺失、过旧、异常退出 |
| 凭据模式 | `file`、`keyring`、`auto` | `file`、`keyring`、`auto` |
| 显示 | 单屏、双屏、屏幕拔插 | 单屏、双屏、屏幕拔插 |
| 缩放 | 100%、125%、150%、200% | Retina、非 Retina 外接屏 |
| 生命周期 | 单实例、关闭隐藏、托盘退出、睡眠唤醒 | 单实例、菜单栏、退出、睡眠唤醒 |
| 网络 | 直连、系统代理、离线与恢复 | 直连、系统代理、离线与恢复 |
| 安装 | NSIS 全新安装、覆盖升级、卸载；MSI 补充验证 | DMG 安装、覆盖升级、卸载 |
| 信任 | Authenticode、SmartScreen、签名链 | Developer ID、公证、stapling、Gatekeeper |
| 更新 | 正常更新、断网、下载中断、签名错误、恢复 | 正常更新、断网、下载中断、签名错误、恢复 |

Windows 10 和尚未列入正式范围的平台失败时，应记录结果但不冒充正式支持。macOS 测试必须在真实 Intel 与 Apple Silicon 设备或具有等价验证能力的环境中完成，不能只靠交叉编译替代。

## 10. 性能与稳定性目标

以下是 1.0 工程目标，不是当前实测结果：

| 指标 | 目标 | 测量方法 |
|---|---:|---|
| 冷启动至浮球可见 | 常见 SSD 设备 2 秒内 | 每平台至少 10 次，报告中位数与 P95 |
| 首次额度结果或明确错误 | 正常网络 15 秒内 | 从进程启动到最终状态 |
| 空闲 CPU | 平均低于 0.5% | 额度稳定后连续观察 10 分钟 |
| 空闲内存 | 应用与 App Server 合计不高于 150 MiB | 记录稳定值和峰值 |
| 同类并发读取 | 最多 1 个 | 注入并发刷新并统计 Provider 调用 |
| 长驻稳定性 | 24 小时无崩溃、无持续增长 | 覆盖刷新、隐藏、唤醒和断网恢复 |

若基线设备差异导致目标需要调整，应先记录测量环境和数据，再修改目标，不能直接删除失败项。

## 11. 当前测试命令

以下命令与当前工程配置一致，均在项目根目录执行：

```powershell
npm ci
npm run check
npm audit --audit-level=high --registry=https://registry.npmjs.org

# Rust 检查
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features

# Windows 调试构建；不生成安装包，不能作为正式发布产物
npm run tauri -- build --debug --no-bundle
```

`npm run check` 统一执行 lint、Vitest、TypeScript 类型检查和 Vite 生产构建。正式产物仍需在各目标平台分别构建，并补充签名、安装器、依赖审计和实机验收。测试命令不得隐式访问真实 Codex 账号或外部非公开接口。

## 12. 发布门槛

以下条件全部满足后才可以把候选版本转为正式版本：

- [x] TypeScript 类型检查、lint、单元测试和生产构建通过；
- [x] `cargo fmt --check`、`cargo clippy -- -D warnings` 和 `cargo test` 通过；
- [ ] App Server 协议契约和服务层状态机测试通过；
- [ ] secret scan、npm 与 Rust 依赖审计没有未接受的高危问题；
- [ ] sidecar 版本、来源、SHA-256、架构、签名及 LICENSE/NOTICE 复核通过；
- [ ] Windows 11 x64 安装、启动、覆盖升级和卸载实测通过；
- [ ] macOS Intel 与 Apple Silicon 安装、启动、覆盖升级和卸载实测通过；
- [ ] Windows Authenticode 与 macOS Developer ID、公证、stapling 验证通过；
- [ ] Tauri updater 正常更新、篡改拒绝和失败恢复通过；
- [ ] 隐私、安全、部署、用户手册和 changelog 已同步；
- [ ] 测试报告包含版本、提交号、产物哈希、测试环境、失败豁免和负责人确认。

以下问题不得带病发布：Token 或账号数据泄露、任意命令执行、未签名更新可安装、sidecar 来源不可追溯、核心额度口径错误、刷新失败覆盖最后成功值、macOS 未公证、Windows 正式包未签名。

## 13. 测试报告模板

```markdown
## 测试报告：vX.Y.Z

- 测试日期：
- Git 提交：
- 候选产物及 SHA-256：
- App Server 版本及 SHA-256：
- Windows 环境：
- macOS 环境：
- 执行人：

### 自动化结果

| 检查 | 结果 | 证据 |
|---|---|---|
| 前端 lint / test / build | 未执行 | |
| Rust fmt / clippy / test | 未执行 | |
| 依赖与秘密扫描 | 未执行 | |

### 实机结果

| 平台与场景 | 结果 | 备注 |
|---|---|---|
| Windows 安装、启动、升级 | 未执行 | |
| macOS Intel 安装、启动、升级 | 未执行 | |
| macOS Apple Silicon 安装、启动、升级 | 未执行 | |
| 签名、公证与更新 | 未执行 | |

### 未解决问题与发布决定

- 阻断问题：
- 已接受风险：
- 结论：禁止发布 / 可发布
```

## 14. 当前报告

### 14.1 测试报告：0.1.1 跨平台预览基线

- 测试日期：2026-07-14
- 基线性质：`0.1.1` 开源跨平台 prerelease；不等同于正式稳定版本验收
- 调试产物：`src-tauri/target/debug/quota-glance.exe`
- Release 预览：`release/QuotaGlance-0.1.1-windows-x64-preview/`，包含未签名 EXE、NSIS、MSI 和 SHA-256 清单
- 测试环境：Windows 本机开发环境；浏览器内 UI 验证使用 1200 × 800 与 400 × 720 视口
- 正式候选产物及 SHA-256：无
- App Server / sidecar 版本及 SHA-256：未验证

#### 自动化结果

| 检查 | 结果 | 证据摘要 |
|---|---|---|
| `npm run check` | 通过 | ESLint 通过；15 个 Vitest 用例通过；TypeScript 类型检查通过；Vite 生产构建通过 |
| `cargo fmt --all -- --check` | 通过 | Rust 格式检查无差异 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 通过 | 无 Clippy warning |
| `cargo test --all-targets --all-features` | 通过 | 45 个 Rust 测试通过：36 个单元测试、9 个假 App Server 跨进程契约 |
| 假 App Server 契约 | 通过 | 覆盖完整只读握手、常驻会话、乱序响应按 ID 匹配、已知/未知通知、超时 pending 清理、迟到响应忽略、全部在途请求异常退出、未知服务端请求 `-32601`、远端错误脱敏、畸形 JSON 和 1 MiB 上限 |
| `npm audit` | 通过 | 使用 npm 官方 registry，报告 0 vulnerabilities |
| RustSec 依赖审计 | 通过并有警告 | `cargo-audit 0.22.2` 扫描 468 个依赖，0 个漏洞、17 个允许警告；GTK3/glib 警告来自 Tauri Linux 目标间接依赖，UNIC 等停止维护警告继续跟踪 |
| Tauri 调试与 Release 构建 | 通过 | `--debug --no-bundle`、优化版 EXE、NSIS 和 MSI 均构建成功 |
| Windows 进程烟测 | 通过 | 调试可执行文件启动后持续运行 3 秒，未发生启动即退出 |
| 当前 Codex 真实只读 POC | 通过 | `codex-cli 0.144.0-alpha.4`，ChatGPT Pro，多桶/双窗口/Credits/banked reset 可读取；未输出邮箱、账号 ID、Token 或原始响应 |
| Release 外部运行时发现 | 通过 | Release 启动 Codex 桌面受管 `codex.exe` 子进程；其自身及 `conhost.exe` 的主窗口句柄均为 0，主程序退出后 8 个后代进程全部回收 |
| 球形浮球与右键菜单 | 通过 | 1200 × 720 极光/石墨/纸白验证；球体 128 × 128px、圆角 50%，标题、96%、重置日期和状态均位于球壳内；菜单仅含“设置”和“退出”；控制台无警告/错误 |
| 浮球原生拖拽 | 通过 | `scripts/smoke-orb-drag.ps1` 驱动真实鼠标输入；136 × 136 浮球移动 110px 且尺寸不变，拖后可展开为 320 × 320 卡片并恢复浮球 |
| Release 原生启动与回收 | 通过 | 优化版 Release 主窗口句柄非零；8 个 WebView/App Server 后代进程窗口句柄均为 0；右键菜单退出后残留为 0；用户原始偏好已恢复 |

#### 浏览器内 UI 验证

| 场景 | 结果 | 备注 |
|---|---|---|
| 1200 × 720 预览 | 通过 | 卡片为 320 × 320 px、浮球窗口为 136 × 136 px，未发现水平或垂直溢出 |
| 窄视口 | 通过 | 320 × 320 卡片、完整设置面板与 136 × 136 px 浮球模式均完整显示；提醒/危险低额度状态的重置日期和状态保持在液体区域内 |
| 周额度筛选 | 通过 | 卡片、浮球和设置只显示 `weekly` 窗口，未出现 5 小时或月度 mock 窗口 |
| 动态水面 | 通过 | 96% 时水面横穿数字基线下方，危险 7% 时液体降至球底；动画名为 `liquid-slosh`、周期 3.4s，并遵守减少动态效果设置 |
| 控制台 | 通过 | 验证期间无 console warning 或 error |
| 交互 | 通过 | 危险态切换后语义色和水位同步更新；浏览器拖动手势后组件稳定，原生 Windows 拖拽、展开/收起和右键退出通过 |
| 键盘与焦点 | 通过 | `Escape` 关闭设置后焦点返回触发按钮 |
| 主题切换 | 通过 | 设置面板提供七个单选项；组件测试覆盖极光、石墨、纸白及三套暖色主题回调；浏览器逐项验证冷色发光边框、球壳和水体联动，三套暖色仍保持可选 |
| 参考图同输入对比 | 通过 | 原始浮球参考图与最终极光主题局部截图在同一次视觉输入中成对复核；修复水线穿字、字重偏重和液体过暗后，无剩余 P0/P1/P2 差异 |
| 圆角与内部滚动 | 通过 | 320 × 320 卡片使用完整 36px 圆角；主题控件获得焦点后卡片 `scrollTop` 保持为 0，设置面板无内部溢出 |
| 设置面板尺寸 | 通过 | 1200 × 800 视口中设置面板实测 306 × 254px，`scrollHeight` 与 `clientHeight` 均为 252px；主题区 280 × 87px，第一行四项、第二行三项居中，无内部滚动或页面溢出 |

浏览器内验证覆盖 WebView 前端的布局与交互，不代替原生窗口、托盘、多屏、缩放、辅助技术或桌面 E2E 验收。

#### 未验证范围与发布决定

- 当前 ChatGPT Pro 账号的额度读取和登录态已验证；真实通知、失败恢复、`file/keyring/auto` 与其他认证模式仍未完成。
- 未验证随包 sidecar 的来源、版本、架构、哈希、签名和再分发许可。
- macOS Intel、Apple Silicon 与 Linux 由 GitHub Runner 执行原生构建，但尚未完成对应系统实机安装和运行测试。
- 已生成跨平台未签名预览安装器，但未完成 Authenticode、Developer ID、公证、Windows 11 安装/升级/卸载和 Linux 桌面环境兼容性验收。
- RustSec 审计已执行且未发现漏洞；17 个维护或不健全警告已登记，后续依赖升级和发布 CI 仍需持续复核。
- 未执行完整桌面 E2E、长驻稳定性、性能和多屏测试。

结论：开发基线允许作为明确标注风险的 `0.1.1` 社区 prerelease 公开发布；在完成受控 App Server/sidecar、全平台实机、签名、公证和安装升级验收前，**禁止标记为正式稳定版本**，也不得将本报告作为 `1.0.0` 验收依据。

### 14.2 测试报告：0.1.4 参考行为与 macOS 统一应用适配

- 测试日期：2026-07-14
- 基线性质：`0.1.4` 社区预览；不等同于正式稳定版本验收
- 当前执行环境：Windows 10 x64 本机开发环境
- 数据来源：本机 Codex App Server 只读协议；未读取 `auth.json`、Token 或系统凭据库，未调用非公开 `wham` HTTP 接口

#### 自动化结果

| 检查 | 结果 | 证据摘要 |
|---|---|---|
| `npm run check` | 通过 | ESLint 通过；20 个 Vitest 用例通过；TypeScript 类型检查和 Vite 生产构建通过 |
| 额度展示投影 | 通过 | 覆盖套餐、短周期/5 小时额度、周额度、两类单窗口降级、跨桶隔离、禁止按槽位猜周期、重置机会总数和到期时间去重排序 |
| 设置组件 | 通过 | 覆盖七套主题、窗口置顶状态与登录时启动开关 |
| 浏览器界面回归 | 通过 | 1280 × 720 展示板与 390 × 844 窄视口均完整渲染；设置面板全部开关可见，主题与登录启动交互状态更新，控制台无警告或错误 |
| Windows Release 烟测 | 通过 | 136 × 136 浮球真实拖动 110px 后保持尺寸，可展开为 320 × 320 卡片并恢复；原生右键菜单出现，无可见子进程窗口，退出后 8 个后代进程全部回收 |
| `cargo fmt` | 通过 | Rust 源码格式化完成 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 通过 | 当前目标无 Clippy warning |
| `cargo test --all-targets --all-features` | 通过 | 当前 Windows 目标共运行 45 项 Rust 测试：36 项默认目标测试与 9 项假 App Server 跨进程契约测试 |
| macOS 运行时发现测试 | 已加入、未在本机执行 | 条件测试覆盖 `ChatGPT.app` 优先、旧 `Codex.app` 回退、仿冒名称/非可执行文件忽略及应用包/运行文件符号链接逃逸拒绝；需由 macOS CI 与实机执行 |

#### 行为对齐结论

- 参考 `change-42-yhmm/quota-float` 的可见数据与常驻体验，当前实现支持套餐、可选短周期/5 小时窗口、周额度、重置机会和到期时间。
- 缓存与恢复口径保持为 30 秒内存缓存、5 分钟可见态安全重同步、隐藏态 10 分钟重同步、通知后完整重读、失败退避和最后成功快照保留。
- 悬浮球继续由周额度驱动，短周期和重置机会只进入展开卡片；服务端未返回时不伪造字段。
- macOS 可发现统一版 `ChatGPT.app` 和旧版 `Codex.app` 的固定资源路径，但尚无 Intel/Apple Silicon 实机运行、登录态和启动项验证。

#### 发布限制

- `0.1.4` 仍不捆绑或重新分发 Codex sidecar，固定版本、来源、SHA-256、许可与分发授权未完成。
- macOS DMG 尚无 Developer ID 签名、公证与 stapling；Windows 安装包尚无 Authenticode 签名。
- GitHub Runner 的构建成功只能作为编译与打包证据，不能替代 Windows 11、macOS Intel/Apple Silicon 和 Linux 桌面环境实机验收。

结论：当前本地自动化基线允许进入 `0.1.4` 社区预览 Release 构建；在取得 sidecar 合法分发、全平台实机、签名、公证和安装升级证据前，**不得标记为正式稳定版本**。

### 14.3 测试报告：0.1.5 macOS CI 门禁修复

- 测试日期：2026-07-14
- 基线性质：`0.1.5` 社区预览补丁；不改变额度读取、运行时发现或界面行为
- 问题证据：`v0.1.4` 五个平台 Release 构建与软件资产发布成功，但随后独立 `main` CI 在 macOS 的 `cargo clippy --all-targets --all-features -- -D warnings` 步骤发现仅供测试使用的辅助函数在普通库目标中触发 `dead_code`
- 修复方式：把 `find_macos_desktop_runtime` 限定为 `#[cfg(all(target_os = "macos", test))]`，生产路径继续通过 `find_macos_desktop_runtime_in_roots` 工作
- 发布门禁：本地前端与 Rust 全量检查、Windows Release 烟测和版本一致性必须通过；再次推送 `main` 后，macOS、Windows、Linux CI 全部成功才允许创建 `v0.1.5` 标签

该修复只收紧测试辅助代码的编译范围，不解除 bundled sidecar 来源、签名、公证和全平台实机验收阻塞。

### 14.4 测试报告：0.1.6 界面层级与交互优化

- 测试日期：2026-07-14
- 基线性质：`0.1.6` 社区预览；优化卡片、设置与浮球界面，不改变 App Server 认证、安全边界和额度口径
- 当前执行环境：Windows 10 x64 本机开发环境

#### 自动化结果

| 检查 | 结果 | 证据摘要 |
|---|---|---|
| `npm run lint` | 通过 | ESLint 无 warning 或 error；浏览器错误场景恢复使用可清理的异步状态同步，不在 effect 内触发同步级联渲染 |
| `npm test` | 通过 | 4 个测试文件、26 项 Vitest 用例全部通过：QuotaCard 16 项、QuotaOrb 5 项、额度投影 4 项、useQuotaGlance 1 项 |
| `npm run build` | 通过 | TypeScript 严格类型检查和 Vite 生产构建成功；6210 个模块完成转换 |
| `npm run release:check-version` | 通过 | package、package-lock 根包、Cargo、Cargo.lock 项目包与 Tauri 版本统一为 `0.1.6` |
| Rust 格式、Clippy 与测试 | 通过 | `cargo fmt --check`、全目标全特性 Clippy `-D warnings` 通过；45 项 Rust 测试全部成功 |
| Tauri Release 构建 | 通过 | `npm run tauri -- build --no-bundle` 生成 Windows x64 优化程序 `quota-glance.exe` |
| Windows 浮球烟测 | 通过 | 136 × 136 浮球真实拖动 110px 后尺寸不变，可展开为 320 × 320 卡片并恢复；原生右键菜单可见，无黑色子进程窗口，退出后无残留进程 |
| 浏览器视觉回归 | 通过 | 320 × 320 卡片与设置面板无内部溢出；136 × 136 正常、提醒、危险浮球均完整显示；控制台 0 warning、0 error |

#### 本轮回归范围

- 320 × 320 卡片保持周额度主读数，短周期额度和重置机会改为整行详情；操作反馈与底部数据新鲜度可同时存在。
- 设置面板使用对话框语义，Tab / Shift+Tab 焦点不会逃出面板；七主题支持方向键、Home 与 End，Escape 关闭后归还焦点。
- 136 × 136 浮球窗口保持 128 × 128 正圆球体，外发光不超出窗口安全边距；暖色主题的球壳、水体、水线、气泡和状态文字使用对应主题令牌。
- 浮球右键、菜单键与 Shift+F10 进入同一精简菜单；菜单仍只包含“设置”和“退出”，拖拽与双击展开行为不变。
- 浏览器错误场景重新读取成功后恢复正常场景，避免数据已更新但控制状态仍停留在错误态。

#### 发布决定

前端、Rust、版本一致性、Windows Release 构建、浮球拖拽/右键菜单烟测与浏览器视觉门禁已通过。正式创建 `v0.1.6` 前仍必须等待 GitHub 跨平台 CI 全部成功，标签触发后还需核对 8 个安装包与 `SHA256SUMS.txt`；上述结果不能替代 macOS Intel / Apple Silicon、Linux 桌面环境和 Windows 11 实机安装验收。

`0.1.6` 不解除 bundled sidecar 来源与分发许可、Windows 商业签名、macOS Developer ID 签名、公证和 stapling 阻塞，因此仍不得标记为 `1.0.0` 正式稳定版。
