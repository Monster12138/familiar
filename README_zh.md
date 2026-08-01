# Familiar

> *“A desktop pet that reacts to your world.”* —— 一只会回应你工作世界的桌面宠物。

[English](README.md)

> [!NOTE]
> Familiar 当前处于 **Alpha** 阶段。核心 Rust 事件流水线、桌面伴侣和 Hook 集成已经可用；更丰富的数据面板、持久化能力以及跨平台正式发布包仍在持续迭代。

## 为什么做 Familiar

Familiar 是一只桌面宠物。它以轻量、持续的陪伴感存在于你的工作空间，并回应身边的工作节奏：Agent 开始任务、工作流等待输入，或你进入长时间的专注状态。Agent 状态联动只是让它“活起来”的一种方式，把本机活动转化为无需打开其他窗口也能感知的细微变化。

它不局限于编程工具。Familiar 的 Hook 层可以接入编程 Agent、其他本地 Agent，以及非编程场景中的本地工作流，让桌面宠物回应更多真实的工作方式。事件在本地完成标准化，由 Tauri 桌面应用渲染；默认不会把数据发送到遥测服务。

## 特性

- **会回应的桌面宠物** —— 像素风动画随工作活动变化，不打断你的工作流。
- **Agent 状态联动** —— 通过 Hook API 连接 Claude Code、Codex CLI 和 Google Antigravity，让桌面宠物反映它们的活动。
- **Hook 扩展能力** —— 通过同一套事件流水线接入其他 Agent 和本地工作流，也支持非编程场景。
- **模块化架构** —— Hook 解析、状态管理、传输和渲染彼此分层。
- **本机状态视图** —— 在桌面伴侣旁显示当前 CPU、内存和磁盘状态。
- **Sprite Pack 皮肤包** —— 使用文档化的 `.fpack` 格式导入和打包更多伴侣形象。
- **隐私优先** —— Hook 数据只在本机处理，运行日志保持最小化，不包含远程遥测。

## 支持的集成

| 集成 | 传输方式 | 状态 |
| --- | --- | --- |
| Claude Code | 通过本地 Familiar 通道上报 Hook 事件 | 待验证 |
| Codex CLI | 通过本地 Familiar 通道上报 Hook 事件 | 可用 |
| Google Antigravity | 支持 transcript 提取的原生 Hook 适配器 | 可用 |

## 快速开始

Familiar 目前还没有可直接安装的正式发布包，请使用 Rust、Node.js 和对应平台的桌面开发依赖从源码构建。

### 前置要求

- **Rust** 1.88 或更高版本，包括 Cargo、rustfmt 和 Clippy
- **Node.js** 18 或更高版本，以及 npm
- **macOS** Xcode Command Line Tools
- **Windows** 包含 C++ 桌面开发组件的 Visual Studio Build Tools
- **Linux** GTK3 和 WebKitGTK 开发包

### 构建与运行

在仓库根目录执行：

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

在 `app/` 目录构建桌面应用：

```bash
cd app
npm ci
npm run build
npm run tauri dev
```

设置窗口可以安装或移除 Familiar 自己管理的 Hook。修改已有 Agent 配置前，程序会先创建备份。

## 架构

| 层 | 职责 |
| --- | --- |
| `familiar-hooks` | 解析、预览、安装和移除不同厂商的 Hook 集成 |
| `familiar-core` | 标准化事件、管理状态、加载配置并提供 Sprite Pack 抽象 |
| `familiar-api` | 本地传输以及实验性的 REST/WebSocket 路由 |
| `familiar-cli` | 供支持的集成使用的轻量 Hook 上报 CLI |
| `app/` | Tauri 组合、托盘集成、桌面窗口和 Vanilla JavaScript 渲染 |

主要本地传输在类 Unix 系统上使用 Unix Domain Socket，必要时使用回环 TCP。Agent 活动当前只保存在进程内存中；SQLite 存储抽象仍处于实验阶段，尚未接入桌面应用。

## 隐私

为了展示当前状态，Familiar 可能读取最近的任务说明、命令文本、工具名称、文件路径以及其他本地 Hook 数据。当 Hook 明确引用 Antigravity transcript 文件时，程序也可能读取该文件。

当前版本不会持久化完整 transcript 或 Agent 事件历史，也不会把采集到的数据发送到远程服务。运行日志只包含事件类型、会话标识和 mood 等最小元数据，不记录原始提示词、命令、路径或 Hook payload。

完整的数据流、配置修改、日志和清理说明见 [docs/PRIVACY.md](docs/PRIVACY.md)。

## 文档

- [隐私与数据处理](docs/PRIVACY.md) —— Familiar 读取、保存和记录什么
- [设计说明](docs/DESIGN.md) —— 架构与协议背景
- [后端工作流](docs/BACKEND_WORKFLOW.md) —— Rust 开发流程
- [前端工作流](docs/FRONTEND_WORKFLOW.md) —— UI 开发流程
- [Sprite Pack 制作指南](docs/SPRITE_PACK_CREATION_GUIDE.md) —— 创建和打包桌面伴侣

## 项目状态

当前已实现：

- 支持集成的本地 Hook 解析与上报
- 事件标准化和桌面状态迁移
- Tauri 桌面伴侣与 Sprite Pack 加载
- Hook 安装和移除设置流程
- CPU、内存和磁盘指标

规划中或仍处于实验阶段：

- Agent 历史记录和保留策略
- 完整的统计与活动数据面板
- 稳定的公共 API 兼容性承诺
- 签名发布包以及经过验证的 Windows/Linux 打包流程

## 参与贡献

欢迎提交 Issue 和 Pull Request。开始之前请阅读：

- [CONTRIBUTING.md](CONTRIBUTING.md) —— 开发环境与贡献流程
- [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) —— 社区行为准则
- [SECURITY.md](SECURITY.md) —— 私密漏洞报告方式

请保持项目的隐私优先特性：未经明确的产品决策，不要加入遥测，也不要持久化用户提示词、transcript、文件内容或原始 Hook payload。

## 开源协议

Familiar 采用 [MIT License](LICENSE-MIT) 或 [Apache License 2.0](LICENSE-APACHE) 双重许可，你可以选择其中一种使用、修改和分发。

内置 sprite、Sprite Pack 压缩包和应用图标使用相同的项目许可证。素材范围及第三方贡献要求见 [ASSETS.md](ASSETS.md)。
