# Hooks 管理重构方案

## 目标

将 Hooks 注入统一收敛到 `familiar-cli`，同时保留本地模式下的前端便捷入口：

- 本地模式：前端可以查看、预览、注入、卸载和测试本机 Hooks。
- 远程模式：前端只能查看远程服务端 Hooks 状态，不能远程修改 Hooks 配置。
- 远程服务器：由部署人员或服务器上的自动化脚本执行 `familiar-cli hooks`。
- Agent 实际运行在哪台机器，Hooks 就在哪台机器上安装和执行。

## 总体架构

```text
本地模式
前端 → Tauri command → 本机 familiar-cli hooks ... → 本机 Agent 配置
Agent → familiar-cli hook → 本机 Hook ingest → 本地状态机 → UI

远程模式
服务器：familiar-cli hooks install
Agent → familiar-cli hook → 服务器 Hook ingest → familiar-cli serve
                                               ↓
                                        State Stream
                                               ↓
                                      本地桌面端只读展示
```

State Stream 只负责状态同步；Hooks 管理属于配置变更和有副作用的操作，不能通过 State Stream 处理。

## CLI 设计

现有的 `familiar-cli hook` 继续作为 Agent 调用的事件上报入口：

```bash
familiar-cli hook --source claude-code --event UserPromptSubmit
```

新增 `familiar-cli hooks` 管理子命令：

```bash
familiar-cli hooks status --json [--config /path/server.toml]
familiar-cli hooks preview --agent claude-code [--config /path/server.toml]
familiar-cli hooks install --agent claude-code [--config /path/server.toml]
familiar-cli hooks uninstall --agent claude-code [--config /path/server.toml]
familiar-cli hooks test --agent claude-code --event UserPromptSubmit [--config /path/server.toml]
familiar-cli hooks install --all [--config /path/server.toml]
```

`hooks` 是运维/部署入口，`hook` 是 Agent 运行时入口，两者保持职责分离。

注入操作必须以运行 Agent 的用户身份执行，不能默认使用 root 修改其他用户的 Agent 配置。

## 共享实现

将现有 `familiar-hooks` 注入、卸载、预览、状态和测试逻辑抽象为共享的 `HookManager`：

```rust
HookManager {
    status(agent)
    preview_inject(agent)
    inject(agent)
    uninstall(agent)
    test(agent, event)
}
```

CLI 直接调用 `HookManager`。本地桌面端通过 Tauri command 调用本机 `familiar-cli hooks`，不在 Tauri 中维护第二套注入逻辑。

## 本地模式行为

当 `[runtime].mode = "local"` 时，设置页面保留完整 Hooks 管理入口：

- 查看注入状态；
- 预览配置变更；
- 注入 Hooks；
- 卸载 Hooks；
- 测试 Hook 点；
- 查看本机配置路径。

Tauri 后端可以封装以下 CLI 调用：

```bash
familiar-cli hooks status --json
familiar-cli hooks preview --agent claude-code
familiar-cli hooks install --agent claude-code
familiar-cli hooks uninstall --agent claude-code
familiar-cli hooks test --agent claude-code --event UserPromptSubmit
```

前端仍然通过 `invoke` 使用稳定的 Tauri command，但这些 command 只在本地模式执行本机 CLI。

## 远程模式行为

当 `[runtime].mode = "remote"` 时，前端切换为只读 Hooks 面板：

- 显示远程服务端连接状态；
- 显示服务端各 Agent 的注入状态；
- 显示 Agent 名称和注入状态，不从服务端返回配置路径；
- 提示管理员在服务端执行 `familiar-cli hooks install`；
- 不显示或禁用注入、卸载、预览和测试按钮。

远程模式下不应注册或允许以下远程写操作：

- `inject_hook`
- `uninstall_hook`
- `preview_inject_hook`
- `test_hook_point`

这样可以从 Tauri command 和 HTTP API 两层保证客户端不能远程修改服务端 Hooks。

## 远程只读状态接口

远程服务端增加只读接口：

```text
GET /api/v1/hooks/status
```

返回最小必要信息，不默认暴露完整服务器目录：

```json
{
  "claude-code": { "injected": true },
  "codex": { "injected": false }
}
```

本地模式的状态来源是本机 `familiar-cli hooks status --json`；远程模式的状态来源是该只读接口。Hooks 状态不放进高频 State Stream 快照，避免无关的网络传输。

## 服务器部署流程

```bash
familiar-cli hooks install --all \
  --config /etc/familiar/server.toml

familiar-cli serve \
  --config /etc/familiar/server.toml
```

服务器配置中的 `[hooks]` 决定 UDS/TCP ingest 位置；Agent 触发的 `familiar-cli hook` 必须能够访问同一个 ingest listener。

## 安全边界

- 不增加远程 Hooks 写 API。
- 远程客户端只访问 Hooks 状态读接口。
- State Stream 和状态接口仍应使用既有 TLS/认证配置保护。
- 状态接口默认只返回注入布尔值和 Agent 名称，不返回完整路径或 Hook payload。
- CLI 参数中的 Agent 名称使用白名单，不允许客户端传入任意文件路径或命令。
- 注入和卸载继续保留配置备份、幂等合并和只删除 Familiar 自有条目的约束。
- CLI 操作记录必要的审计日志，但不记录 prompt、文件内容或完整 Hook payload。

## 与另一种远程场景的边界

如果 Agent 在本地运行、State Stream 服务端在远程机器，那么远程服务端注入并不能让本地 Agent 上报事件。这个场景需要后续单独设计“远程 Hook Relay”：

```text
本地 Agent
  → familiar-cli hook
  → 远程 Hook ingest/relay endpoint
  → 远程 StateMachine
  → State Stream
  → 本地 UI
```

当前方案优先覆盖 Agent 与 `familiar-cli serve` 同机的远程服务器部署方式。

## 实施顺序

1. 抽取共享 `HookManager`，复用现有 Agent 注入实现。
2. 增加 `familiar-cli hooks` 管理命令及 JSON 输出。
3. 将本地 Tauri Hooks 操作改为调用本机 CLI。
4. 增加服务端 `GET /api/v1/hooks/status` 只读接口。
5. 前端按运行模式切换可操作/只读界面。
6. 删除或禁用远程模式下的所有 Hooks 写操作入口。
7. 增加 CLI 注入、卸载、重复注入、备份、状态查询和模式切换测试。

## 验收标准

- 本地模式可以从设置页完成 Hook 预览、注入、卸载和测试。
- 本地设置页执行的实际逻辑与 CLI 完全一致。
- 远程模式设置页只能查看服务端 Hooks 状态，不能修改服务端配置。
- 服务器执行 `familiar-cli hooks install` 后，Agent 事件可以进入 `familiar-cli serve` 并通过 State Stream 到达客户端。
- 未配置远程认证或 TLS 时，不开放超出本地可信范围的状态访问。
- 注入、卸载和重复执行不会破坏用户已有的 Agent Hooks 配置。
