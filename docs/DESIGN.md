# Familiar 架构设计文档

> 版本: v0.1 | 更新时间: 2026-07-19

## 1. 项目定位

**Familiar** 是一个开源（MIT/Apache-2.0）的跨平台桌面伴侣系统，通过 AI Agent 官方 Hooks API 实时采集 agent 工作状态，以桌面宠物、菜单栏、仪表盘等多种形态展示，并提供编码数据统计。

### 差异化

| 对比项 | Comnyang | Familiar |
|---|---|---|
| 开源 | ❌ 闭源 | ✅ 开源 + 社区驱动 |
| Agent 集成深度 | 思考/完成 两种状态 | 精确到读文件/写代码/跑命令 |
| Agent 范围 | 仅 Coding | Coding + Workflow + DevOps |
| 渲染形态 | 仅桌面宠物 | 桌面宠物 / 菜单栏 / 仪表盘 / 远程推送 |
| 数据统计 | 无 | 编码行数、commit、Agent 使用统计 |

---

## 2. 架构总览

```
┌─────────────────────────────────────────────────────────────────┐
│                    Familiar 进程（单进程）                        │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                  Rust 后端                               │    │
│  │                                                         │    │
│  │  ┌──────────┐    ┌───────────┐    ┌──────────────────┐  │    │
│  │  │  Hooks   │───▶│ Event Bus │───▶│  State Machine   │  │    │
│  │  │  Layer   │    │(broadcast)│    │ (聚合 RenderState)│  │    │
│  │  └──────────┘    └─────┬─────┘    └────────┬─────────┘  │    │
│  │       ▲                │                   │            │    │
│  │       │                ▼                   │            │    │
│  │  Unix Socket    ┌───────────┐              │            │    │
│  │  (监听)         │Statistics │              │            │    │
│  │                 │ Engine    │              │            │    │
│  │                 └─────┬────┘              │            │    │
│  │                       ▼                   │            │    │
│  │                 ┌──────────┐              │            │    │
│  │                 │  SQLite  │              │            │    │
│  │                 └──────────┘              │            │    │
│  │                                           │            │    │
│  │  ┌──────────┐                             │            │    │
│  │  │  axum    │◀────────────────────────────┘            │    │
│  │  │  API     │ (REST/WS for 外部客户端)                  │    │
│  │  └──────────┘                                          │    │
│  │                                                         │    │
│  │  ┌──────────┐                                          │    │
│  │  │  System  │ (托盘图标)                                │    │
│  │  │  Tray    │                                          │    │
│  │  └──────────┘                                          │    │
│  └─────────────────────────────────────────────────────────┘    │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                  WebView (Tauri 内嵌)                    │    │
│  │                                                         │    │
│  │  ┌────────────┐  ┌─────────────┐  ┌──────────────┐    │    │
│  │  │ 宠物窗口    │  │ 仪表盘窗口   │  │ 设置窗口      │    │    │
│  │  │ (Canvas)   │  │ (HTML/JS)   │  │ (HTML/JS)    │    │    │
│  │  └────────────┘  └─────────────┘  └──────────────┘    │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
        ▲                                          │
        │ familiar-hook CLI                        │ WebSocket
        │ (被 Agent 调用)                           ▼
  ┌───────────┐                            ┌──────────────┐
  │ Claude    │                            │ 手机 / 硬件   │
  │ Code /   │                            │ 屏幕 / 浏览器  │
  │ Codex    │                            └──────────────┘
  └───────────┘
```

### 运行模式

| 模式 | 场景 | 组件 |
|---|---|---|
| **桌面模式**（默认） | 日常使用 | Rust 后端 + WebView + Tray |
| **Server 模式** | 远程服务器 / 硬件接入 | `familiar-cli serve` = Rust 后端 + State Stream，无 UI |

### IPC 通信

```
Rust → JS:  tauri::Emitter  (emit 事件推送 RenderState)
JS → Rust:  tauri::command   (invoke 请求查询数据)
```

---

## 3. Hooks Layer（采集层）

### 设计原则

**仅使用 Agent 官方 Hooks API 进行采集**。不使用日志解析、进程监控、文件系统监控等非精确方案。未来新增 Agent 时，只接入有官方 Hook/Plugin API 的 Agent。

### 核心链路

```
Agent 官方 Hook 触发
  → 调用 familiar-hook CLI
  → Agent 通过 stdin 传入 JSON 事件数据
  → familiar-hook 通过 Unix Socket 转发给 Familiar 主进程
  → CliAgentHookAdapter 统一解析
  → 生成 AgentEvent → 发布到 Event Bus
```

### 统一事件协议

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvent {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub source: AgentSource,
    pub category: AgentCategory,
    pub event_type: AgentEventType,
    pub metadata: EventMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentSource {
    // Coding Agents
    ClaudeCode,
    Codex,
    // Workflow Agents (未来)
    // DevOps (未来)
    // 社区插件
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentCategory {
    Coding,
    Workflow,
    DevOps,
    General,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentEventType {
    // Agent 生命周期
    AgentStarted,
    AgentStopped,

    // 思考/处理
    Thinking,
    Processing { description: String },

    // 编码活动
    ReadingFile { path: String },
    WritingFile { path: String },
    RunningCommand { cmd: String },
    SearchingCode { query: String },
    BrowsingWeb { url: String },

    // 工作流活动（未来）
    WorkflowStarted { workflow: String },
    WorkflowCompleted { workflow: String, duration_secs: u64 },
    WorkflowFailed { workflow: String, error: String },

    // DevOps 活动（未来）
    PipelineTriggered { pipeline: String },
    PipelineSucceeded { pipeline: String },
    PipelineFailed { pipeline: String, error: String },

    // 通用结果
    TaskCompleted { summary: String },
    TaskFailed { error: String },
    WaitingForInput,

    // 子 Agent
    SubagentStarted { agent_type: String },
    SubagentStopped { agent_type: String },
}
```

### AgentHook Trait

```rust
#[async_trait]
pub trait AgentHook: Send + Sync {
    fn name(&self) -> &str;
    fn category(&self) -> AgentCategory;
    async fn start(&self, sender: mpsc::Sender<AgentEvent>) -> Result<()>;
    async fn stop(&self) -> Result<()>;
}
```

### 通用 CLI Agent Hook 适配器

Claude Code 和 Codex 的 Hooks 机制几乎一致，共用一个适配器：

```rust
pub struct CliAgentHookAdapter {
    agent_source: AgentSource,
}

impl CliAgentHookAdapter {
    /// 解析 Agent stdin JSON，映射为统一 AgentEvent
    pub fn parse_hook_input(&self, stdin_json: &Value) -> AgentEvent {
        let event_name = stdin_json["hook_event_name"].as_str().unwrap();
        let event_type = match event_name {
            "SessionStart"      => AgentEventType::AgentStarted,
            "Stop"              => AgentEventType::AgentStopped,
            "PreToolUse"        => self.parse_pre_tool_use(stdin_json),
            "PostToolUse"       => self.parse_post_tool_use(stdin_json),
            "PermissionRequest" => AgentEventType::WaitingForInput,
            "SubagentStart"     => AgentEventType::SubagentStarted { /* ... */ },
            "SubagentStop"      => AgentEventType::SubagentStopped { /* ... */ },
            _ => AgentEventType::Processing { description: event_name.to_string() },
        };
        // ...
    }

    fn parse_pre_tool_use(&self, json: &Value) -> AgentEventType {
        match json["tool_name"].as_str().unwrap_or("") {
            "Bash" => AgentEventType::RunningCommand {
                cmd: json["tool_arguments"]["command"]
                    .as_str().unwrap_or("").to_string(),
            },
            "apply_patch" | "Edit" | "Write" => AgentEventType::WritingFile {
                path: /* 从 tool_arguments 解析 */,
            },
            name if name.starts_with("mcp__") => AgentEventType::Processing {
                description: format!("MCP: {}", name),
            },
            other => AgentEventType::Processing {
                description: other.to_string(),
            },
        }
    }
}
```

### familiar-hook CLI

极轻量的二进制，被 Agent Hooks 调用：

```
用法: familiar-hook --source <agent-name>

- 从 stdin 读取 Agent 传入的 JSON
- 通过 Unix Socket (/tmp/familiar.sock) 发送给 Familiar 主进程
- 立即退出（不阻塞 Agent）
```

### Agent Hook 配置示例

**Claude Code** (`~/.claude/settings.json`):
```json
{
  "hooks": {
    "SessionStart": [{
      "hooks": [{ "type": "command", "command": "familiar-hook --source claude-code --event SessionStart" }]
    }],
    "UserPromptSubmit": [{
      "hooks": [{ "type": "command", "command": "familiar-hook --source claude-code --event UserPromptSubmit" }]
    }],
    "PreToolUse": [{
      "matcher": "*",
      "hooks": [{ "type": "command", "command": "familiar-hook --source claude-code --event PreToolUse" }]
    }],
    "PostToolUse": [{
      "matcher": "*",
      "hooks": [{ "type": "command", "command": "familiar-hook --source claude-code --event PostToolUse" }]
    }],
    "PostToolUseFailure": [{
      "matcher": "*",
      "hooks": [{ "type": "command", "command": "familiar-hook --source claude-code --event PostToolUseFailure" }]
    }],
    "PermissionRequest": [{
      "matcher": "*",
      "hooks": [{ "type": "command", "command": "familiar-hook --source claude-code --event PermissionRequest" }]
    }],
    "SubagentStart": [{
      "hooks": [{ "type": "command", "command": "familiar-hook --source claude-code --event SubagentStart" }]
    }],
    "SubagentStop": [{
      "hooks": [{ "type": "command", "command": "familiar-hook --source claude-code --event SubagentStop" }]
    }],
    "Stop": [{
      "hooks": [{ "type": "command", "command": "familiar-hook --source claude-code --event Stop" }]
    }],
    "SessionEnd": [{
      "hooks": [{ "type": "command", "command": "familiar-hook --source claude-code --event SessionEnd" }]
    }]
  }
}
```

All hooks report passively: the CLI always exits `0` and prints nothing, so Familiar
never blocks the agent and never overrides the user's own permission rules.

**Codex** (`~/.codex/config.toml` 或项目 `.codex/hooks.json`):
```json
{
  "hooks": {
    "SessionStart": [{
      "matcher": "startup|resume",
      "hooks": [{ "type": "command", "command": "familiar-hook --source codex" }]
    }],
    "PreToolUse": [{
      "matcher": ".*",
      "hooks": [{ "type": "command", "command": "familiar-hook --source codex" }]
    }],
    "PostToolUse": [{
      "matcher": ".*",
      "hooks": [{ "type": "command", "command": "familiar-hook --source codex" }]
    }],
    "Stop": [{
      "hooks": [{ "type": "command", "command": "familiar-hook --source codex" }]
    }]
  }
}
```

---

## 4. Core Layer（核心层）

### 4a. Event Bus（事件总线）

```rust
pub struct EventBus {
    sender: broadcast::Sender<AgentEvent>,
    history: Arc<RwLock<VecDeque<AgentEvent>>>,  // 环形缓冲区
}
```

基于 `tokio::broadcast` 实现发布-订阅。保留近期事件历史，供新连接的 renderer 回放。

### 4b. State Machine（状态机）

将原始 `AgentEvent` 聚合为 `RenderState`：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub source: AgentSource,
    pub category: AgentCategory,
    pub status: AgentStatus,
    pub current_activity: Option<String>,
    pub progress: Option<f32>,
    pub started_at: Option<DateTime<Utc>>,
    pub last_event: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentStatus {
    Idle, Thinking, Working, Pending, Completed, Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderState {
    pub agents: Vec<AgentState>,
    pub active_agent_count: usize,
    pub agents_by_category: HashMap<AgentCategory, Vec<AgentState>>,
    pub today_stats: DailyStats,
    pub mood: FamiliarMood,
    pub notifications: Vec<Notification>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FamiliarMood {
    Happy, Thinking, Busy, Sleepy, Alarmed, Celebrating, Watching,
}
```

### 4c. Statistics Engine（统计引擎）

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyStats {
    pub date: NaiveDate,
    pub lines_added: u64,
    pub lines_removed: u64,
    pub commits: u32,
    pub files_changed: u32,
    pub coding_active_seconds: u64,
    pub agent_tasks_completed: u32,
    pub agent_tasks_failed: u32,
    pub agent_thinking_seconds: u64,
    pub workflows_completed: u32,
    pub workflows_failed: u32,
    pub pipelines_succeeded: u32,
    pub pipelines_failed: u32,
    pub deployments: u32,
    pub by_source: HashMap<String, SourceStats>,
}
```

存储：**SQLite** (via `rusqlite` + `tokio::task::spawn_blocking`)

| 表名 | 用途 |
|---|---|
| `daily_stats` | 每日聚合数据 |
| `events` | 原始事件日志（可配置保留天数） |
| `sessions` | Agent 会话记录 |
| `achievements` | 成就解锁记录 |

### 4d. API Server

内置 `axum` HTTP/WebSocket 服务器：

```
GET  /health                    → 服务健康检查
WS   /api/v1/state-stream       → 远程桌面的脱敏完整状态流
WS   /api/v1/display-stream     → ESP8266 等硬件的紧凑显示状态流
GET  /api/v1/hooks/status       → 远程 Hooks 只读状态
```

远程路由共享服务端 Bearer Token 鉴权。硬件显示流只包含协议版本、服务端 ID、
revision、mood 和活跃 Agent 数，不发送任务摘要、通知或 Agent 标识。ESP8266 +
ST7789 参考实现及小虎素材转换流程见 `hardware/esp8266/README.md`。

---

## 5. Renderer Layer（渲染层）

### Renderer Trait

```rust
#[async_trait]
pub trait Renderer: Send + Sync {
    fn name(&self) -> &str;
    async fn render(&self, state: &RenderState) -> Result<()>;
    fn is_supported(&self) -> bool;
    async fn start(&self) -> Result<()>;
    async fn stop(&self) -> Result<()>;
}
```

### 前端渲染器接口

```typescript
interface SpriteRenderer {
  init(canvas: HTMLCanvasElement): void;
  loadSpritePack(manifest: SpritePack): Promise<void>;
  playAnimation(name: string): void;
  showBubble(text: string, duration: number): void;
  setPosition(x: number, y: number): void;
  destroy(): void;
}

// 不同实现（设计上预留，MVP 只实现第一个）
class PixelSpriteRenderer implements SpriteRenderer { /* Canvas 2D */ }
class VectorSpriteRenderer implements SpriteRenderer { /* SVG/Canvas */ }
class Live2DRenderer implements SpriteRenderer { /* Live2D SDK */ }
```

### 渲染形态

| 形态 | 实现方式 | 阶段 |
|---|---|---|
| **桌面宠物** | Tauri 透明窗口 + Canvas 2D | MVP |
| **菜单栏图标** | Tauri system tray API | MVP |
| **仪表盘** | Tauri 窗口 / Web 页面 + ECharts | Phase 3 |
| **远程客户端** | 通过 API Server WebSocket | Phase 4 |
| **硬件屏幕** | 通过 API Server WebSocket | Phase 4 |

### Sprite Pack 格式

```json
{
  "name": "pixel-cat",
  "author": "community",
  "version": "1.0.0",
  "sprite_sheet": "sprites.png",
  "tile_size": [32, 32],
  "animations": {
    "idle":         { "frames": [0,1,2,3],    "fps": 4, "loop": true },
    "thinking":     { "frames": [4,5,6,7],    "fps": 3, "loop": true },
    "working":      { "frames": [8,9,10,11],  "fps": 6, "loop": true },
    "happy":        { "frames": [12,13,14],   "fps": 5, "loop": false },
    "alarmed":      { "frames": [15,16,17],   "fps": 8, "loop": true },
    "sleeping":     { "frames": [18,19],      "fps": 1, "loop": true },
    "celebrating":  { "frames": [20,21,22,23],"fps": 8, "loop": false },
    "watching":     { "frames": [24,25],      "fps": 2, "loop": true }
  },
  "bubbles": {
    "reading":   "📖 Reading {file}",
    "writing":   "✍️ Writing {file}",
    "command":   "⚡ Running {cmd}",
    "workflow":  "⚙️ {workflow}",
    "pipeline":  "🚀 {pipeline}",
    "error":     "❌ Error!"
  }
}
```

---

## 6. 配置系统

TOML 格式，支持热重载。

```toml
[general]
language = "zh-CN"
auto_start = true
data_retention_days = 90

# ─── Hooks ────────────────────────────
[hooks]
enabled = ["claude-code", "codex"]
socket_path = "/tmp/familiar.sock"

# ─── Renderer ─────────────────────────
[renderer]
enabled = ["desktop-pet", "menu-bar"]

[renderer.desktop-pet]
sprite = "pixel-cat"
scale = 2
position = "bottom-right"
always_on_top = true
opacity = 0.95

[renderer.menu-bar]
show_active_count = true
show_today_stats = true

[renderer.dashboard]
port = 9527

# ─── API ──────────────────────────────
[api]
enabled = true
port = 19527

# ─── Notifications ────────────────────
[notifications]
dnd_start = "22:00"
dnd_end = "08:00"
min_level = "info"

# ─── Achievements ─────────────────────
[achievements]
enabled = true
```

---

## 7. 技术选型

| 组件 | 技术 | 理由 |
|---|---|---|
| 桌面框架 | **Tauri 2.0** | 跨平台、低内存（~10MB）、Rust 原生 |
| 异步运行时 | **tokio** | Rust 异步标准 |
| 事件总线 | **tokio::broadcast** | 轻量、零外部依赖 |
| 本地存储 | **SQLite (rusqlite)** | 嵌入式、无需额外服务 |
| API 服务 | **axum** | 高性能、tokio 生态 |
| HTTP 客户端 | **reqwest** | 未来 Workflow/DevOps Hook 用 |
| 配置 | **TOML (toml crate)** | Rust 社区标准 |
| 序列化 | **serde + serde_json** | 标准 |
| 前端构建 | **Vite** | 快、HMR、Tauri 推荐 |
| 宠物渲染 | **Canvas 2D API** | 像素风足够，无需 WebGL |
| 仪表盘图表 | **ECharts** | 图表类型丰富 |
| 日志 | **tracing** | 结构化日志 |
| CI/CD | **GitHub Actions + tauri-action** | 跨平台构建 + 发布 |
| 插件（Phase 4） | **WASM** | 沙箱安全、跨平台 |

---

## 8. 项目结构

```
familiar/
├── Cargo.toml                       # workspace 根
├── LICENSE-MIT
├── LICENSE-APACHE
├── README.md
├── docs/
│   └── DESIGN.md                    # 本文档
│
├── crates/
│   ├── familiar-core/               # 核心库
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── event.rs             # AgentEvent 定义
│   │       ├── event_bus.rs         # 事件总线
│   │       ├── state.rs             # RenderState / AgentState / FamiliarMood
│   │       ├── state_machine.rs     # 状态聚合
│   │       ├── stats.rs             # 统计引擎
│   │       ├── storage.rs           # SQLite
│   │       ├── config.rs            # 配置管理
│   │       └── plugin.rs            # 插件接口
│   │
│   ├── familiar-hooks/              # Hook 实现
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── hook_trait.rs        # AgentHook trait
│   │       ├── adapter.rs           # CliAgentHookAdapter（通用适配器）
│   │       ├── claude_code.rs       # Claude Code hook 配置
│   │       └── codex.rs             # Codex hook 配置
│   │
│   ├── familiar-api/                # HTTP/WebSocket API
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── routes.rs
│   │       └── ws.rs
│   │
│   └── familiar-cli/                # CLI 工具 + familiar-hook
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs              # familiar-cli 主入口
│           └── hook_reporter.rs     # familiar-hook 子命令
│
├── app/                             # Tauri 桌面应用
│   ├── src-tauri/
│   │   ├── Cargo.toml
│   │   ├── tauri.conf.json
│   │   └── src/
│   │       ├── main.rs
│   │       ├── tray.rs
│   │       └── commands.rs
│   │
│   └── src/                         # Web 前端
│       ├── index.html
│       ├── main.js
│       ├── pet/
│       │   ├── PetCanvas.js         # Canvas sprite 渲染引擎
│       │   ├── SpriteLoader.js
│       │   └── BubbleOverlay.js
│       ├── dashboard/
│       │   ├── Dashboard.js
│       │   ├── Charts.js
│       │   └── AgentList.js
│       └── styles/
│           ├── pet.css
│           └── dashboard.css
│
├── sprites/                         # 内置素材
│   └── pixel-cat/
│       ├── manifest.json
│       └── sprites.png
│
└── config/
    └── default.toml
```

---

## 9. 补充特性

### 隐私与安全
- 纯本地数据，不上传任何服务器
- 可配置路径前缀过滤、命令内容脱敏
- 无遥测
- API Token 支持环境变量注入

### 智能免打扰
- 全屏模式自动隐藏宠物
- 用户自定义静默时段

### 成就系统
- "连续 7 天编码"、"Agent 完成 100 个任务"等成就徽章
- 宠物可根据成就解锁新动画/皮肤

### 远程推送（未来）
- 初期通过 Telegram Bot / Discord Bot
- 后续考虑 Flutter 轻量 App

---

## 10. 开发路线

### Phase 1: 基础骨架（2~3 周）
- [ ] 项目初始化（Tauri 2.0 + Cargo workspace）
- [ ] `familiar-core`：事件定义、事件总线、状态机
- [ ] `familiar-hooks`：CliAgentHookAdapter + Claude Code hook
- [ ] `familiar-cli`：familiar-hook 子命令
- [ ] Menu Bar Renderer（系统托盘）
- [ ] 基础 TOML 配置

### Phase 2: 桌面宠物（2~3 周）
- [ ] Desktop Pet Renderer：透明窗口 + Canvas sprite
- [ ] 内置像素猫 sprite pack
- [ ] 状态气泡（显示 agent 当前活动）
- [ ] 宠物拖拽 + 基础交互
- [ ] FamiliarMood 心情系统
- [ ] Codex hook 支持

### Phase 3: 统计 + 仪表盘（2 周）
- [ ] Statistics Engine + SQLite 存储
- [ ] Dashboard 仪表盘（图表）
- [ ] 编码热力图、Agent 使用统计

### Phase 4: 社区与扩展（2 周）
- [ ] Sprite Pack 加载系统
- [ ] WASM 插件系统
- [ ] API Server（WebSocket 推送）
- [ ] 文档 + 贡献指南

---

## 11. 验证计划

### 自动化测试
```bash
cargo test --package familiar-core
cargo test --package familiar-hooks
cargo test --package familiar-api
cargo test --workspace
```

### 手动验证
- macOS 启动 Claude Code → 观察宠物状态变化
- macOS 启动 Codex → 观察宠物状态变化
- 仪表盘数据准确性
- 多 Agent 并发场景
- 资源占用（目标：< 30MB 内存，< 1% CPU idle）

### 注意事项
- Linux 透明窗口：Wayland 上可能不稳定，需检测并 fallback
- Agent 日志格式变化：Hook 内做版本检测 + 容错解析
- 多显示器：用 Tauri `monitor` API 获取屏幕信息
