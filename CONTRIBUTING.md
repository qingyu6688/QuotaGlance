# 贡献指南

感谢参与 QuotaGlance。项目接受代码、测试、文档、设计、翻译和跨平台兼容性反馈。

## 开始之前

- 遵守 [行为准则](CODE_OF_CONDUCT.md)。
- 安全漏洞不要公开提交 Issue，请按 [SECURITY.md](SECURITY.md) 私下报告。
- 较大的功能或行为变更先提交 Issue，说明使用场景、平台范围和兼容性影响。
- 项目不接收 Token、Cookie、`auth.json`、真实账号响应、个人路径或其他敏感数据。

## 本地开发

需要 Node.js `>= 20.19.0`、npm、Rust stable，以及当前平台对应的 [Tauri 2 前置依赖](https://v2.tauri.app/start/prerequisites/)。

```bash
git clone https://github.com/qingyu6688/QuotaGlance.git
cd QuotaGlance
npm ci
npm run check
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features
```

浏览器预览使用模拟数据：

```bash
npm run dev
```

桌面开发模式需要本机已安装并登录 Codex：

```bash
npm run tauri dev
```

## 提交要求

1. 从最新 `main` 创建功能分支。
2. 保持改动聚焦，不重构与 Issue 无关的代码。
3. 新功能和 Bug 修复补充对应测试。
4. 用户可见行为、配置、接口或发布流程发生变化时同步更新 Markdown 文档和 `docs/changelog.md`。
5. 提交信息使用 `<类型>: <中文简述>`，例如 `fix: 修复 Linux 下 Codex 路径发现`。
6. Pull Request 中写明影响平台、验证命令、已知限制和必要截图。

## Pull Request 检查项

- [ ] 没有提交凭据、账号数据、个人路径或构建缓存。
- [ ] `npm run check` 通过。
- [ ] `cargo fmt`、`cargo clippy` 和 `cargo test` 通过。
- [ ] Windows、macOS、Linux 的条件编译分支保持可构建，或已说明无法验证的平台。
- [ ] 文档和测试已随行为更新。

维护联系：maorongkang@gmail.com。
