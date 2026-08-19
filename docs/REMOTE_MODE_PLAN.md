# Familiar 本地一体化与远程订阅模式改造方案

## 1. 背景与目标

Familiar 当前以本地一体化方式运行：Agent 调用 `familiar-cli hook`，事件经本机
UDS 或 loopback TCP 进入 Tauri Rust 后端，由 EventBus 和 StateMachine 聚合，
最终通过 Tauri event 推送给 WebView。

本方案在保留默认本地体验的前提下，增加远程订阅模式，使 Agent 和 Familiar
服务端可以运行在远程服务器，本地机器只运行轻量桌面客户端并订阅渲染状态。

目标：

- 默认继续使用本地一体化模式，已有用户无需迁移配置。
- 通过配置切换为远程订阅模式。
- 本地与远程模式共享相同的状态模型和 UI 渲染链路。
- 远程状态同步允许丢失中间更新，以最后收到的有效完整快照为准。
- 尽量减少网络消息数量和单条消息体积。
- 用户可以明确配置是否启用 TLS。
- 服务端命令统一为 `familiar-cli serve`，不保留 `headless` 兼容别名。
- 不新增遥测，不默认向远端暴露原始 prompt、Hook payload、文件内容或命令输出。

非目标：

- 首版不支持同时混合本地 Agent 和远端 Agent 状态。
- 首版不支持多服务端聚合。
- 首版不实现可靠事件投递、历史重放或断点续传。
- 首版不实现增量 patch 协议。
- 首版不提供完整的云账号或设备管理系统。

## 2. 运行拓扑

### 2.1 本地一体化模式（默认）

```text
本机 Agent
  -> familiar-cli hook
  -> UDS / loopback TCP
  -> HookIngestServer
  -> EventBus
  -> StateMachine
  -> LocalStateProvider
  -> Tauri event
  -> 桌面 UI
```

该模式不建立任何远程连接，保持现有的本地优先行为。

### 2.2 远程订阅模式

```text
远程服务器
  Agent
    -> familiar-cli hook
    -> UDS / loopback TCP
    -> familiar-cli serve
    -> EventBus
    -> StateMachine
    -> WebSocket State Stream
                          |
                          | WS / WSS
                          v
本地机器
  Tauri Desktop Shell
    -> RemoteStateProvider
    -> Tauri event
    -> 桌面 UI
```

远程模式下，本地 Tauri Shell 仍负责窗口、托盘、透明、置顶、通知、本地显示偏好和
凭证安全存取。WebView 不直接连接远端 WebSocket。

## 3. 运行模式配置

新增运行模式配置：

```toml
[runtime]
# local | remote
mode = "local"
```

兼容规则：旧配置缺少 `[runtime]` 时，自动视为 `mode = "local"`。

| 能力 | `local` | `remote` |
|---|---:|---:|
| 本地 Hook listener | 启动 | 不启动 |
| 本地 StateMachine | 启动 | 不作为 UI 状态源 |
| 远程 WebSocket client | 不启动 | 启动 |
| Tauri 窗口与托盘 | 启动 | 启动 |
| 桌面 UI | 本地状态 | 远端状态 |
| 本地显示配置 | 生效 | 生效 |
| Agent 行为和状态规则 | 本地配置 | 服务端配置 |

远程快照中的 Agent 统计（交互数、完成任务数、来源计数）来自服务端；CPU、内存、磁盘
仪表仍然读取本地桌面机器，因为它们描述的是 UI 所在设备而不是 Agent 服务端。

`remote` 的语义必须保持单一：状态完全来自一个远端服务端。如果未来需要混合本地和
远端状态，应新增独立的 `hybrid` 模式并定义冲突规则。

## 4. 模块职责与抽象

### 4.1 StateProvider

Tauri UI 只依赖统一状态源接口，不直接持有具体的 StateMachine 或 WebSocket：

```rust
#[async_trait]
pub trait StateProvider: Send + Sync {
    async fn start(&self) -> anyhow::Result<()>;
    async fn current_state(&self) -> anyhow::Result<RenderState>;
    fn subscribe(&self) -> broadcast::Receiver<StateUpdate>;
    async fn shutdown(&self);
}
```

提供两种实现：

- `LocalStateProvider`：包装本地 EventBus 和 StateMachine。
- `RemoteStateProvider`：管理 WebSocket、最新快照缓存、连接状态和自动重连。

Tauri 从 provider 订阅后，继续向 WebView 发出现有的 `state_changed` 事件。前端渲染
逻辑不需要感知状态来自本地还是远端。

另增加连接状态事件：

```text
connection_status_changed
```

### 4.2 HookIngestServer

将当前 Tauri 主程序中的 UDS/TCP 监听、换行 JSON 解码、Agent 来源识别和事件解析
抽取为可复用的 `HookIngestServer`。桌面本地模式和 `familiar-cli serve` 使用同一实现，
避免两套 Agent 解析行为发生偏差。

Hook ingest 继续只监听 UDS 或 loopback TCP，不作为公网协议开放。

### 4.3 建议代码归属

```text
familiar-core
  EventBus / StateMachine / RenderState
  StateProvider 公共接口
  State Stream DTO 与版本规则

familiar-hooks
  Agent Hook 配置与 payload 解析
  HookIngestServer

familiar-api
  HTTP 健康检查
  WebSocket State Stream Server
  鉴权、连接管理和协议处理

familiar-cli
  hook
  serve

app/src-tauri
  LocalStateProvider 装配
  RemoteStateProvider
  窗口、托盘、通知和本地显示配置
```

## 5. 服务端命令

服务端统一使用：

```bash
familiar-cli serve
```

删除 `headless` 子命令，不提供隐藏 alias 或兼容别名。相关代码、测试和文档均更新为
`serve`。

建议支持：

```bash
familiar-cli serve
familiar-cli serve --config /etc/familiar/server.toml
familiar-cli serve --bind 0.0.0.0:19528
```

配置优先级：

```text
CLI 参数 > 配置文件 > 默认值
```

Token 和 TLS 私钥内容不通过普通命令行参数传入，避免出现在 shell history 或进程列表。

`serve` 必须启动完整的 HookIngestServer、Agent 解析器、EventBus、StateMachine 和
State Stream，而不是保留当前只处理部分 Agent 的原型行为。

## 6. 网络与 TLS 配置

### 6.1 服务端配置

```toml
[server]
bind = "0.0.0.0:19528"

[server.tls]
enabled = true
cert_path = "/etc/familiar/tls/server.crt"
key_path = "/etc/familiar/tls/server.key"

[server.auth]
enabled = true
token_env = "FAMILIAR_SERVER_TOKEN"

[server.state_stream]
max_updates_per_second = 10
max_task_summary_chars = 160
max_activity_summary_chars = 160
```

行为约束：

- `server.tls.enabled = true` 时提供 HTTPS/WSS。
- `server.tls.enabled = false` 时提供 HTTP/WS。
- TLS 开启但证书或私钥无效时，服务端启动失败，不得静默降级到明文。
- TLS 和 token 鉴权相互独立；关闭 TLS 不等于关闭鉴权。
- TLS 关闭时，日志明确说明当前使用明文连接。

### 6.2 客户端配置

```toml
[runtime]
mode = "remote"

[remote]
endpoint = "familiar.example.com:19528"
path = "/api/v1/state-stream"
tls = true
token_env = "FAMILIAR_REMOTE_TOKEN"
connect_timeout_secs = 10
reconnect_initial_secs = 1
reconnect_max_secs = 30
```

客户端根据 `tls` 生成连接协议：

```text
tls = true  -> wss://<endpoint><path>
tls = false -> ws://<endpoint><path>
```

使用独立的 `endpoint`、`path` 和 `tls` 字段，避免 URL scheme 与 TLS 配置冲突。

允许用户关闭 TLS，不增加第二个 `allow_insecure` 开关。设置界面和日志需要明确提示：
关闭 TLS 后 token 和状态数据为明文传输，适用于 localhost、SSH tunnel、Tailscale 或
WireGuard 等已有可信加密通道；直接跨公网连接建议开启 TLS。

Token 首版通过环境变量读取，后续可接入 macOS Keychain、Windows Credential
Manager 和 Linux Secret Service。Token 不进入 WebView JavaScript，也不得写入普通日志。

### 6.3 端口分离

Hook ingest 和 UI State Stream 使用不同端口：

```text
19527：Hook ingest，仅 UDS/loopback
19528：HTTP/WebSocket API
```

避免现有 `hooks.tcp_port` 和 `api.port` 使用同一默认值所造成的冲突。

## 7. State Stream 同步语义

State Stream 定义为：

> 无历史、允许丢失中间更新、最终以最新完整快照为准的状态流。服务端和客户端均不
> 保证交付每一个中间状态；每条快照独立完整，客户端以同一服务实例中 revision 最大的
> 已接收快照为准。

这一协议不提供事件确认、补发、历史重放或断点续传。

### 7.1 为什么使用完整快照

允许丢失消息时，增量 patch 会造成客户端状态不完整。每条消息必须自包含，使客户端
即使漏掉任意数量的中间 revision，也能直接采用下一条快照。

首版使用 JSON，先测量真实负载再决定是否引入 MessagePack、CBOR 或 WebSocket 压缩。

### 7.2 Hello 消息

连接建立后，服务端首先发送：

```json
{
  "type": "hello",
  "v": 1,
  "server_id": "d07193be-e62b-4bc0-a356-b256ad5246b9",
  "server_version": "1.3.0",
  "heartbeat_secs": 30
}
```

- `v` 是网络协议版本，与应用版本分开。
- `server_id` 每次服务进程启动时生成。
- 客户端使用 `(server_id, revision)` 共同判断新旧，允许服务端重启后 revision 从较小值
  重新开始。

### 7.3 状态快照

连接成功后立即发送当前完整快照，之后仅在可渲染状态发生变化时推送：

```json
{
  "type": "state",
  "v": 1,
  "server_id": "d07193be-e62b-4bc0-a356-b256ad5246b9",
  "revision": 43,
  "timestamp": 1787131200000,
  "agents": [
    {
      "id": "session-1",
      "source": "codex",
      "category": "coding",
      "status": "working",
      "task_summary": "分析订单同步失败的原因",
      "activity_summary": "正在检查支付回调代码",
      "last_event_at": 1787131199500
    }
  ],
  "mood": "focused"
}
```

时间使用 Unix 毫秒。空的可选字段不发送。

网络协议使用独立、版本化 DTO，不直接把内部 Rust `RenderState` 或 enum 作为永久网络
协议。新增字段保持向后兼容；客户端忽略未知消息类型和未知可选字段；不支持的主协议
版本应断开并提示升级。

### 7.4 用户输入摘要和活动摘要

桌面气泡需要：

- `task_summary`：适合 UI 展示的用户输入或任务摘要。
- `activity_summary`：Agent 当前活动摘要。

它们属于当前可渲染状态，必须包含在每一份完整快照中，不能作为一次性消息单独发送。
因此即使用户输入发生在 revision 41，客户端只收到 revision 43，仍能正确展示当前任务。

当前内部字段可按以下方式映射：

```text
AgentState.user_instruction -> task_summary
AgentState.current_activity -> activity_summary
```

本地 Tauri 可以将远程 DTO 映射回现有字段，避免立即修改所有前端代码。

首版不调用外部模型生成语义摘要。“摘要”采用确定性的本地处理：合并空白、移除换行、
按 Unicode 字符边界截断并添加省略号。推荐默认上限均为 160 个字符。

## 8. 减少网络通信量

### 8.1 只发送实际变化

原始 AgentEvent 不直接触发网络消息。只有最终可渲染状态与上一份不同，才提高 revision
并发布新快照。多个事件得到相同 UI 状态时不发送消息。

### 8.2 合并高频状态

`max_updates_per_second` 是最大推送频率，而不是固定轮询频率。默认值建议为 10，即最短
发送间隔为 100ms。窗口内发生多次变化时，只发送窗口结束时的最新状态。

值为 `0` 时表示不限制频率，但仍使用 latest-only 队列。

### 8.3 Latest-only 背压

服务端每个客户端只保留一个待发送状态，建议基于：

```rust
tokio::sync::watch::Sender<Arc<EncodedSnapshot>>
```

而不是有容量累积的事件队列。慢客户端只观察最新值，中间状态可被覆盖，不会导致应用层
队列无限增长。已经进入操作系统 TCP 缓冲区的旧 WebSocket frame 无法撤回，因此该保证
针对应用层排队。

每个 WebSocket writer：

1. 等待 `watch.changed()`。
2. 读取当前最新 snapshot。
3. 比较该连接上次成功发送的 revision。
4. 只发送更高 revision。
5. 发送期间若发生多次更新，下一轮直接读取最新值。

### 8.4 单次序列化、多客户端复用

同一 revision 只构建和序列化一次：

```rust
pub struct EncodedSnapshot {
    pub server_id: Uuid,
    pub revision: u64,
    pub bytes: Arc<[u8]>,
}
```

所有客户端共享序列化后的字节。

### 8.5 消除冗余字段

远程协议不照搬完整 `RenderState`：

- 只传一次 `agents`，不传复制 AgentState 的 `agents_by_category`。
- `active_agent_count` 可由 `agents.length` 得到时不传。
- 仅用于展示的 source/category 统计尽量在客户端计算。
- 不发送原始 Hook payload、完整 transcript 或文件内容。

完整快照不能因为字段与上一帧相同就省略 `task_summary` 等当前状态字段，否则消息会退化
成依赖前序消息的增量协议。

### 8.6 心跳与压缩

使用 WebSocket 原生 ping/pong，不增加 JSON 心跳消息。建议 30 秒发送一次 ping，连续
60 秒没有收到 pong 时关闭连接。

首版不强制启用 `permessage-deflate`。先记录不含内容的本地运行指标：平均/P95 消息字节、
每分钟推送数、每客户端流量和被合并的状态数量，再决定是否值得增加压缩支持。这些指标
只用于本地诊断，不构成遥测。

## 9. 客户端状态与重连

```rust
pub enum RemoteConnectionStatus {
    Disabled,
    Connecting,
    Connected,
    Reconnecting { attempt: u32 },
    AuthenticationFailed,
    IncompatibleProtocol,
    Offline,
}
```

客户端保存：

```text
current_server_id
last_revision
latest_state
```

处理规则：

- 同一 `server_id` 下，revision 小于或等于已接收值的消息直接丢弃。
- revision 不连续属于正常情况，直接接受更大的 revision，不请求补发。
- `server_id` 改变时清空 revision，并接受新服务端快照。
- UI 启动不等待网络连接。
- 短暂断线时保留最后快照，但明确标记为离线，不能继续表示状态仍然实时。
- 自动重连采用 `1s -> 2s -> 4s -> 8s -> 16s -> 30s` 指数退避，并加入约 ±20% 抖动。

可选磁盘缓存必须包含 `cached_at`、`server_id` 和 `revision`。从缓存恢复的状态必须明确
显示为历史快照，不能将缓存中的 Working 状态展示为仍在实时工作。

## 10. 配置职责拆分

### 10.1 服务端配置

影响事件语义、状态聚合、数据暴露和统计结果：

- Agent hook 开关。
- Event 到状态的映射。
- Agent 状态超时。
- 完成动画对应的服务端状态保留时间。
- 数据保留期限和统计。
- 摘要的清理、截断和隐私级别。

### 10.2 客户端本地配置

只影响当前设备如何展示：

- 精灵包、缩放、位置、透明度、置顶和多桌面行为。
- 窗口边框和 dashboard 布局。
- 语言和本地通知偏好。
- 是否显示任务气泡。
- 本设备隐藏哪些 session。

远程模式的设置面板建议分为“本机显示”“远程连接”“服务端设置”。首版实现前两组；
服务端设置暂时只读或不展示，不能伪装成本地修改已同步到服务器。

## 11. 隐私与认证

Hook ingest 的换行 JSON 协议没有远程认证能力，只允许 UDS 或 loopback，不得直接暴露
到公网。

State Stream 只发送 UI 所需的最小状态。服务端提供内容暴露配置：

```toml
[server.state_stream.content]
task_summary = "truncated"      # hidden | truncated | full
activity_summary = "sanitized" # hidden | sanitized | full
max_task_summary_chars = 160
max_activity_summary_chars = 160
```

语义：

- `hidden`：字段不发送。
- `truncated`：清理空白并截断，不进行语义改写。
- `sanitized`：仅发送“正在运行命令”“正在读取文件”等活动类型，不发送具体命令或路径。
- `full`：发送完整字段，由用户显式选择。

建议默认使用 `task_summary = "truncated"` 和
`activity_summary = "sanitized"`。

认证失败、协议不兼容、TLS 错误和普通离线必须区分呈现。认证 token 不得进入前端 JS、
普通日志、错误 toast 或状态快照。

## 12. 实施阶段

### 阶段一：内部解耦

1. 提取 `HookIngestServer`，统一 UDS/TCP 解析逻辑。
2. 引入 `StateProvider` 和 `LocalStateProvider`。
3. Tauri 统一从 provider 订阅，并继续发出 `state_changed`。
4. 增加 `[runtime] mode = "local"` 及旧配置默认兼容。
5. 梳理 Tauri commands，避免状态查询绕过 provider 直接访问 StateMachine。

验收：

- 现有所有 Agent hook 集成继续工作。
- 本地模式行为和 UI 不变。
- 本地模式不建立远程网络连接。
- 旧配置无需修改即可启动。

### 阶段二：完整服务端

1. 删除 `headless`，新增 `serve`。
2. `serve` 复用完整 HookIngestServer 和所有 Agent parser。
3. 实现 `/health` 和 `/api/v1/state-stream`。
4. 实现版本化 hello、完整状态快照、latest-only 广播和最大推送频率。
5. 实现 token 鉴权、消息大小限制、连接数限制和可配置 TLS。

验收：

- 所有支持的 Agent 在服务端与桌面本地模式产生相同状态。
- 新客户端连接后无需等待新事件即可获得当前快照。
- 多个客户端可以同时订阅。
- 慢客户端不会导致状态队列增长或阻塞 StateMachine。
- TLS 配置损坏时拒绝启动，不发生静默降级。
- 未认证连接被拒绝。

### 阶段三：桌面远程订阅

1. 实现 `RemoteStateProvider`。
2. 增加鉴权、TLS、心跳、重连和最新快照缓存。
3. 根据 `runtime.mode` 选择 provider。
4. 远程模式不启动本地 Hook listener。
5. 设置页增加远程 endpoint、TLS、token 环境变量名和连接测试。
6. 通过全局 toast 和持久连接状态展示连接结果。

验收：

- 修改配置并重启后可在 local/remote 间切换。
- 远端状态可以驱动宠物、任务摘要、当前活动和 dashboard。
- 网络中断不导致应用崩溃或窗口冻结。
- 网络恢复后自动重连并接受最新完整快照。
- TLS、认证失败和协议不兼容有明确且不同的错误提示。
- token 不进入 WebView 和普通日志。

### 阶段四：配置和统计完善

1. 完成本地显示配置与服务端行为配置的边界拆分。
2. 增加只读服务端信息接口。
3. 明确统计数据由服务端展示还是同步到本地。
4. 根据实际需要增加设备配对、token 轮换和系统凭证库支持。
5. 根据实测流量决定是否启用二进制协议或压缩。

## 13. 测试计划

### 13.1 单元测试

- 旧配置缺少 `runtime` 时默认为 `local`。
- local/remote、TLS、endpoint 和路径配置校验。
- TLS 开启时缺少或损坏证书会失败。
- State Stream DTO 的序列化快照测试。
- Unicode 摘要清理和截断。
- 未知可选字段和未知消息类型兼容。
- 同一 server ID 下旧 revision 被忽略。
- server ID 改变后 revision 可以重新开始。
- 重连退避和最大间隔。
- `headless` 不再是合法子命令。

### 13.2 集成测试

使用真实 WebSocket 测试服务端和 `RemoteStateProvider`：

- 连接后立即收到最新状态。
- Token 正确、错误和缺失。
- TLS 开启和关闭。
- 状态更新和高频合并。
- 中间 revision 丢失后客户端直接接受最新快照。
- 服务端主动断开和客户端自动重连。
- 服务端重启导致 server ID 和 revision 重置。
- 超大消息与慢消费者不会拖住广播。
- 多客户端共享同一份序列化快照。

### 13.3 回归验证

- Claude Code、Codex、Qoder、Antigravity 和 DeepSeek Harness 的本地 hook 链路。
- `cargo test --workspace`。
- `cargo clippy --workspace --all-targets -- -D warnings`。
- `cd app && npm run build`。
- 仅格式化本次修改的 Rust 文件并运行 `git diff --check`。
- 在 macOS、Linux 和 Windows 上验证 loopback、证书路径和进程生命周期差异。

## 14. 主要风险与约束

### 表面抽象

如果 Tauri commands 和设置页继续直接访问 StateMachine，StateProvider 只会成为表面封装。
所有状态读取必须统一经过 provider；本地 UI 操作和服务端管理操作需要明确分开。

### 网络协议绑定内部类型

直接序列化内部 RenderState 虽然实现快，但会让内部字段调整破坏旧客户端。首版必须建立
独立、版本化的 DTO。

### 摘要造成隐私外泄

用户任务摘要、路径、命令和错误内容可能敏感。服务端必须在进入 State Stream DTO 前
完成清理和隐私策略应用，客户端不能承担服务端脱敏责任。

### 明文远程连接

用户可以配置关闭 TLS，但 UI 和日志必须明确显示连接为明文。该选择适合已有安全隧道，
不应被包装成等价于 WSS 的安全连接。

### 慢客户端与伪实时状态

latest-only 队列可以避免应用层积压，但客户端离线后保存的最后状态不再实时。UI 必须将
连接状态与 Agent 状态同时展示，不能在离线时继续无提示地显示 Working。

## 15. 首版交付范围

首版包含：

- `local` 和 `remote` 两种模式。
- `familiar-cli serve`，删除 `headless`。
- Rust 侧单 WebSocket 连接管理。
- Token 鉴权和用户可配置 TLS。
- 完整、自包含、latest-only 的 JSON 状态快照。
- 用户任务摘要和当前活动摘要随快照传输。
- 最大推送频率、状态合并、单次序列化多客户端复用。
- 自动重连和明确的在线/离线状态。
- 本地显示配置与服务端状态配置的职责分离。

首版不包含：

- 增量 patch 或可靠事件流。
- 历史重放和断点续传。
- 多服务器聚合。
- 本地与远端混合状态。
- 云账号体系和复杂设备授权后台。
- 外部模型摘要服务。

该方案保持默认安装即用的本地模式，同时让远程 Agent 场景建立在独立、可版本化且符合
隐私原则的状态订阅边界上。
