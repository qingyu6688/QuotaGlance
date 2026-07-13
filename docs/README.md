# QuotaGlance 文档中心

本目录保存 QuotaGlance 的需求、计划、设计、测试、部署和用户文档。

当前工程版本为 `0.1.0`，项目已从纯文档阶段进入 **M0/M1/M2 实施阶段**：前端 14 项测试和 Rust 44 项测试通过，Windows Release 已通过当前 Codex 桌面运行副本读取真实账号额度。以上仍只是 Windows 10 开发机结果，不等于完成固定版本、完整认证矩阵、Windows 11、macOS、签名、公证或正式发布验收。

## 推荐阅读顺序

1. [项目分析与立项建议](QuotaGlance-项目分析.md)
2. [项目实施计划](project-plan.md)
3. [需求规格说明](requirements.md)
4. [概要设计](design.md)
5. [详细设计](detail-design.md)
6. [首版界面设计基线](ui-design.md)
7. [IPC 与 App Server 接口设计](api.md)
8. [安全设计](security.md)
9. [测试计划与报告](test.md)
10. [部署与发布](deploy.md)

## 当前工程基线

| 范围 | 当前事实 | 结论 |
|---|---|---|
| 工程骨架 | Tauri 2、React、TypeScript、Vite 与 Rust 工程已建立 | 已实现 |
| 前端 | 额度卡片、球形悬浮球、两项右键菜单、设置面板、模拟状态与 Tauri API 适配已落地 | 工程验证通过，产品验收未完成 |
| 协议与领域层 | 动态额度模型、JSONL 边界、只读 App Server 常驻会话与 pending request map 已落地 | 合成 POC 已建立，真实 sidecar 待验证 |
| 自动化测试 | 前端 14 项、Rust 44 项测试通过 | 当前开发机通过；含 9 项假服务跨进程协议契约和 1 项受管运行时定位测试 |
| 构建与运行 | 前端构建、Tauri debug `--no-bundle` 构建和进程烟测通过 | 仅开发构建证据 |
| 界面验证 | 浏览器模式完成视觉和交互 QA | 使用模拟数据，不代表真实账号联调 |
| sidecar | release 尚无 bundled sidecar | 正式数据链路不可发布 |
| 平台与发布 | Windows 10 仅作开发机；macOS、安装包、签名、公证未验证 | 禁止标记为已交付 |

## 文档索引

| 文档 | 用途 | 当前状态 |
|---|---|---|
| [项目分析与立项建议](QuotaGlance-项目分析.md) | 立项背景、技术选型、范围和风险 | 首版完成 |
| [项目实施计划](project-plan.md) | M0—M6 路线、任务、验收门槛和需求追踪 | 执行基线 |
| [未完成问题分析](issues-analysis.md) | 未完成项分级、根因、方案、状态和解除条件 | 2026-07-13 审计基线 |
| [环境说明](environment.md) | 工具链、端口、环境变量和平台验证边界 | 已按 `0.1.0` 更新 |
| [需求规格说明](requirements.md) | 用户、范围、功能和非功能需求 | 基线版 |
| [概要设计](design.md) | 系统上下文、分层、模块和架构决策 | 基线版 |
| [详细设计](detail-design.md) | 协议、状态机、缓存、调度和平台行为 | 基线版，随实现校准 |
| [首版界面设计基线](ui-design.md) | 概念图、令牌、排版、组件和交互规则 | M1 实施基线 |
| [数据库与本地存储设计](database.md) | MVP 无数据库决策、偏好 JSON 和未来 SQLite 边界 | 基线版 |
| [IPC 与 App Server 接口设计](api.md) | Commands、Events、错误码和外部协议映射 | 基线版，随 POC 校准 |
| [测试计划与报告](test.md) | 测试范围、矩阵、发布门槛和执行记录 | 持续更新 |
| [部署与发布](deploy.md) | 构建、签名、公证、更新和回滚 | 计划版；尚未发布 |
| [安全设计](security.md) | 威胁模型、凭据边界和安全控制 | 基线版 |
| [隐私说明](privacy.md) | 面向用户的数据处理说明 | 基线版 |
| [用户手册](user-manual.md) | 安装、界面、操作和状态解释 | 随产品实现更新 |
| [故障排查](troubleshooting.md) | 常见故障定位与脱敏诊断信息 | 持续更新 |
| [更新日志](changelog.md) | 版本与重要变更记录 | 持续维护 |

## 状态定义

- **计划版**：描述预期能力，尚无对应实现或完整验证。
- **基线版**：已形成评审基线，仍可能随 POC 或实现调整。
- **已实现**：存在对应代码，并通过相应自动化检查。
- **已验证**：在明确记录的平台、版本和命令下完成验证。
- **已验收**：在目标平台按验收标准完成实测，可进入下一发布门槛。

“已实现”“已验证”和“已验收”不能互相替代。当前 Windows 10 开发机上的测试与构建通过，不代表 Windows 11、macOS 或签名发行已经验收。

## 维护规则

- 功能范围变化时，同步更新 `requirements.md`、`api.md` 和测试用例。
- 架构或数据流变化时，同步更新 `design.md`、`detail-design.md` 和 `security.md`。
- 里程碑、优先级或验收门槛变化时，同步更新 `project-plan.md`。
- 环境变量、签名或发布流程变化时，同步更新 `environment.md` 和 `deploy.md`。
- 用户可见行为或文案变化时，同步更新 `ui-design.md`、`user-manual.md` 和 `troubleshooting.md`。
- 每次版本变更同步更新 `changelog.md`。
- 测试结果必须注明日期、平台、版本和执行命令；不得用参考项目结果代替本项目证据。
- 文档不得记录 Token、密码、账号 ID、签名私钥、证书内容、个人路径或原始账号响应。

文档维护：maorongkang@gmail.com
