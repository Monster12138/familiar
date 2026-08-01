# Familiar

> *"The familiar that never sleeps"* — Your local-first desktop companion for coding agents.

一个模块化、高性能的开源桌面伴侣系统，实时连接各种 AI Agent 工具（如 Claude Code、Codex CLI、Google Antigravity），以桌面宠物、菜单栏等多种形态实时展示 agent 工作状态与活动数据。

---

> [!NOTE]
> **成熟度说明 (Project Maturity)**: 当前项目处于 **Alpha / 早期开发阶段**。核心 Rust 逻辑、Tauri 桌面宠物与 Hook 采集层已可用，更多高级面板与跨平台编译打包持续迭代中。

---

## 🌟 特性 (Features)

- 🔌 **多 Agent 支持** — 支持 Claude Code、Codex CLI 及 Antigravity，基于官方 Hook API 精准采集事件。
- 🧩 **模块化三层架构** — `familiar-hooks` 采集层、`familiar-core` 事件与状态层、`app` 桌面渲染层相互分离。
- 🐱 **桌面伴侣/宠物** — 基于 Canvas 的像素风动画，实时呈现 Agent 的思考、执行命令、闲置与休眠状态。
- 📊 **本机状态面板** — 当前显示 CPU、内存和磁盘使用情况；Agent 历史统计与持久化仍在规划中。
- 🎨 **自定义皮肤包 (Sprite Pack)** — 开放的资源包格式，轻松引入更多桌面伴侣样式。
- 🔒 **隐私优先 (Privacy First)** — Hook 事件只在本机处理，当前不发送远程遥测或用户活动数据。详见 [PRIVACY.md](docs/PRIVACY.md)。

---

## 🛠️ 技术栈 (Tech Stack)

| 组件 | 技术选择 |
|---|---|
| 桌面框架 | Tauri 2.0 |
| 后端语言 | Rust (Tokio, Axum, Rusqlite) |
| 前端渲染 | Vanilla ES Modules + Canvas API + Vite |
| 本地事件传输 | Unix Domain Socket / TCP loopback；REST/WebSocket 仍为实验性 scaffold |
| 采集 CLI | `familiar-cli` |

---

## 🚀 快速开始与开发指南

### 前置要求
- **Rust**: 1.88+
- **Node.js**: v18+ & npm
- **C++ 构建工具** (Windows) / **Xcode Command Line Tools** (macOS)

### 1. 验证后端与 Hook 库
```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

### 2. 运行桌面端开发服务器
```bash
cd app
npm ci
npm run build
npm run tauri dev
```

---

## 🔒 隐私与安全 (Privacy & Security)

- **本地处理**: 为了显示当前任务，Familiar 会从 hook payload 或本地 transcript 中提取最近的任务说明、命令和文件活动，并仅保存在进程内存中。当前版本不会把 Agent 事件写入 SQLite，也不会向远程服务上报这些数据。
- **日志最小化**: 持久化运行日志只记录事件种类、会话标识和 mood，不记录提示词、命令、文件路径或原始 hook payload。
- **完整说明**: 具体读取范围、配置修改和清理方式见 [PRIVACY.md](docs/PRIVACY.md)。
- **安全漏洞报告**: 如发现安全漏洞，请阅读 [SECURITY.md](SECURITY.md) 获取私密报告渠道。

---

## 🤝 贡献与社区 (Contributing)

欢迎提交 Issue 和 Pull Request！在开始之前，请阅读：
- [CONTRIBUTING.md](CONTRIBUTING.md) — 贡献流程与代码规范
- [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) — 社区行为准则

---

## 📄 开源协议 (License)

本项目采用 **[MIT](LICENSE-MIT)** 与 **[Apache-2.0](LICENSE-APACHE)** 双重开源协议（Dual License）。
您可以根据需要自由选择在 MIT 或 Apache 2.0 许可下使用、修改与分发本软件。
仓库内置的 sprite 与应用图标采用相同许可证，详见 [ASSETS.md](ASSETS.md)。
