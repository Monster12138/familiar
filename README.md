# Familiar

> *"The familiar that never sleeps"* — Your desktop familiar for every agent.

一个模块化、高性能的开源桌面伴侣系统，实时连接各种 AI Agent 工具，以桌面宠物、菜单栏、硬件屏幕等多种形态展示 agent 工作状态和活动数据。

## Features

- 🔌 **多 Agent 支持** — Claude Code、Codex CLI，通过官方 Hooks API 精准采集
- 🧩 **三层模块化架构** — Hooks 采集层、Core 核心层、Renderer 渲染层完全解耦
- 🐱 **桌面宠物** — 像素风动画，实时展示 agent 正在做什么
- 📊 **编码仪表盘** — 编码行数、commit、Agent 使用统计
- 🖥️ **跨平台** — macOS / Windows / Linux
- ⚡ **高性能** — Rust 后端 + Tauri 2.0，< 30MB 内存
- 🎨 **社区皮肤** — 开放的 Sprite Pack 格式，自定义宠物样式
- 🔒 **隐私优先** — 纯本地数据，零遥测

## Tech Stack

| 组件 | 技术 |
|---|---|
| 桌面框架 | Tauri 2.0 |
| 后端语言 | Rust (tokio) |
| 前端渲染 | Vanilla JS + Canvas API |
| 数据库 | SQLite (rusqlite) |
| API 服务 | axum |
| 构建工具 | Vite |

## License

MIT / Apache-2.0 双协议
