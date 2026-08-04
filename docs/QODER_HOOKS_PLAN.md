# Qoder Hooks 接入方案

## 1. 目标与范围

为 Familiar Hooks 增加对 Qoder Desktop IDE、JetBrains 插件和 CLI 共用 Hooks 协议的支持，使 Qoder Agent 的会话生命周期和工具调用状态能够同步到 Familiar 桌面宠物。

本方案基于 Qoder 官方文档（访问日期：2026-08-03）：

<https://docs.qoder.com/zh/extensions/hooks>

首版覆盖 Qoder 当前公开的五类事件：

| Qoder 事件 | Familiar 事件 | 用途 |
| --- | --- | --- |
| `UserPromptSubmit` | `AgentStarted` | 创建/唤醒会话并记录用户任务 |
| `PreToolUse` | 工具对应的工作状态 | 展示 Agent 即将执行的操作 |
| `PostToolUse` | `Processing` | 工具成功完成，Agent 继续工作 |
| `PostToolUseFailure` | `Processing` | 展示单次工具失败，不结束整个任务 |
| `Stop` | `TaskCompleted` | 展示任务完成状态 |

Qoder 当前没有 `SessionStart` 或 `SessionEnd`。因此首版以 `UserPromptSubmit` 作为会话开始，以 `Stop` 作为任务结束；如果首个事件不是 Prompt，沿用状态机现有的“首个工作事件隐式创建 Agent”行为。

## 2. Qoder Hooks 协议摘要

Qoder 从以下配置文件加载 Hooks，并按用户级、项目级、项目本地级合并：

```text
~/.qoder/settings.json
.qoder/settings.json
.qoder/settings.local.json
```

Familiar 首版只管理用户级 `~/.qoder/settings.json`，不自动修改项目文件，避免无意污染项目仓库。Qoder 修改配置后需要重启 IDE/插件才能生效；当前官方文档说明暂不支持热加载。

每个 Hook 通过标准输入接收 JSON，使用退出码和标准输出控制行为：

| 退出码 | 含义 |
| --- | --- |
| `0` | 成功，继续执行 |
| `2` | 阻断（仅支持阻断的事件生效） |
| 其他 | 非阻断错误，继续执行并展示错误 |

Familiar 的 Hook 只负责本地状态上报，不应阻断 Qoder 的工作。即使 Familiar 守护进程离线，CLI 也应返回允许/成功结果，不能影响 Qoder Agent。

## 3. 配置注入设计

新增 `QoderHook`，向 `~/.qoder/settings.json` 注入以下配置。`<familiar-cli>` 使用现有二进制路径发现逻辑生成。

```json
{
  "hooks": {
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "<familiar-cli> hook --source qoder --event UserPromptSubmit"
          }
        ]
      }
    ],
    "PreToolUse": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "<familiar-cli> hook --source qoder --event PreToolUse"
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "<familiar-cli> hook --source qoder --event PostToolUse"
          }
        ]
      }
    ],
    "PostToolUseFailure": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "<familiar-cli> hook --source qoder --event PostToolUseFailure"
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "<familiar-cli> hook --source qoder --event Stop"
          }
        ]
      }
    ]
  }
}
```

`QoderHook` 的注入、预览和卸载行为应与现有 Claude Code/Codex Hook 保持一致：

- 保留用户已有的 Hook 配置，只追加 Familiar 自己的命令。
- 注入前创建带时间戳的备份。
- 重复注入幂等，不重复添加相同命令。
- 卸载只删除同时包含 `familiar-cli` 和 `--source qoder` 的命令。
- 卸载后清理空 Hook 数组和空的 `hooks` 对象。
- 非法 JSON、不可写文件和目录创建失败时返回可操作错误，不覆盖原文件。
- 配置命令路径需要正确处理路径中的空格，避免直接拼接未经转义的 Shell 参数。

## 4. 核心数据模型与事件映射

### 4.1 Agent 来源

在 `crates/familiar-core/src/event.rs` 的 `AgentSource` 中增加显式枚举值：

```rust
pub enum AgentSource {
    ClaudeCode,
    Codex,
    Antigravity,
    Qoder,
    Custom(String),
}
```

在 `crates/familiar-hooks/src/adapter.rs` 中将 `Qoder` 归类为 `AgentCategory::Coding`。不建议使用 `Custom("qoder")`，因为当前 `Custom` 会被归类为 `General`，也不利于 UI Badge 和统计维度保持稳定。

### 4.2 通用输入字段

Qoder 所有事件都可能包含：

```json
{
  "session_id": "abc-123",
  "cwd": "/path/to/project",
  "hook_event_name": "PreToolUse",
  "transcript_path": "/path/to/transcript.json"
}
```

适配器使用 `session_id` 作为 Agent ID 来源。对非 UUID 的 session ID，继续使用现有的确定性 UUID 转换，以保证同一会话的事件落到同一个 Agent。

首版不主动读取 `transcript_path`。Qoder 的 `UserPromptSubmit.prompt` 和事件本身已提供状态展示所需信息，避免扩大敏感文件读取范围。

### 4.3 UserPromptSubmit

输入示例：

```json
{
  "session_id": "abc-123",
  "cwd": "/path/to/project",
  "hook_event_name": "UserPromptSubmit",
  "prompt": "帮我写一个排序函数"
}
```

映射为：

```rust
AgentEventType::AgentStarted {
    instruction: Some(prompt),
}
```

若 Prompt 缺失或为空，仍发送 `AgentStarted { instruction: None }`，确保会话被创建。

### 4.4 PreToolUse

输入示例：

```json
{
  "session_id": "abc-123",
  "hook_event_name": "PreToolUse",
  "tool_name": "Bash",
  "tool_input": {
    "command": "npm test"
  }
}
```

Qoder 支持原生工具名和 Claude Code 兼容工具名，适配器需要同时识别：

| Qoder 原生名 | 兼容名 | Familiar 映射 |
| --- | --- | --- |
| `run_in_terminal` | `Bash` | `RunningCommand` |
| `read_file` | `Read` | `ReadingFile` |
| `create_file` | `Write` | `WritingFile` |
| `search_replace` | `Edit` | `WritingFile` |
| `delete_file` | — | `WritingFile` 或通用 `Processing` |
| `grep_code` | `Grep` | `SearchingCode` |
| `search_file` | `Glob` | `SearchingCode` |
| `search_web` | `WebSearch` | `BrowsingWeb` |
| `fetch_content` | `WebFetch` | `BrowsingWeb` |
| `mcp__<server>__<tool>` | 同左 | MCP `Processing` |

现有通用适配器需要优先从 `tool_input` 读取参数，并补充 Qoder 常用字段：

- 命令：`command`、`cmd`。
- 文件路径：`file_path`、`path`、`filePath`。
- 搜索词：根据工具输入提取 `query` 或关键词字段。

当前适配器未读取 `tool_input`，如果不修正，Qoder 事件只能显示通用工具名，无法展示具体命令和文件路径。

### 4.5 PostToolUse

映射为：

```rust
AgentEventType::Processing {
    description: "Tool finished".into(),
}
```

该事件不可阻断。`tool_response` 可以保留在事件元数据中供当前状态链路使用，但不应写入持久化日志。

### 4.6 PostToolUseFailure

映射为：

```rust
AgentEventType::Processing {
    description: "Tool failed".into(),
}
```

不建议首版映射为 `TaskFailed`：工具失败不等于 Agent 任务失败，Qoder 可能随后自动修正并继续执行。若未来需要单独的失败动画，可新增 `ToolFailed` 领域事件，但不应阻塞本次接入。

### 4.7 Stop

输入包含 `last_assistant_message` 和 `stop_hook_active` 等字段，映射为：

```rust
AgentEventType::TaskCompleted {
    summary: "Task finished".into(),
}
```

Qoder 当前 Stop 不支持通过 Hook 阻断 Agent 停止，因此 Familiar 只消费事件，不尝试返回 `decision: block`。

## 5. 代码改动范围

### Rust 核心与 Hook 层

- `crates/familiar-core/src/event.rs`
  - 增加 `AgentSource::Qoder`。
- `crates/familiar-hooks/src/lib.rs`
  - 导出 `qoder` 模块。
- `crates/familiar-hooks/src/qoder.rs`
  - 实现 `QoderHook`。
  - 配置路径、注入 payload、状态检测、备份、预览和卸载。
- `crates/familiar-hooks/src/adapter.rs`
  - 增加 `AgentSource::Qoder` 分类。
  - 支持 `tool_input`、`file_path` 和 Qoder 原生工具名。
  - 增加 `PostToolUseFailure` 映射。
- `crates/familiar-hooks/tests/`
  - 增加 Qoder 注入、卸载、解析和幂等测试。

### CLI 与桌面端

- `crates/familiar-cli/src/hook_reporter.rs`
  - 确认 Qoder 五个事件的返回 JSON 均为非阻断成功结果。
- `app/src-tauri/src/commands.rs`
  - 注册 `QoderHook`。
- `app/src-tauri/src/main.rs`
  - 在 UDS 和 TCP 两条事件入口都将 `qoder` 映射为 `AgentSource::Qoder`。
- `app/src/settings.html`
  - 增加 Qoder Hook 状态、注入、查看配置和卸载操作。
- `app/src/settings.js`
  - 将 `qoder` 加入 Hook Agent 列表及事件绑定。
- `app/src/i18n.js`
  - 增加 Qoder 相关中英文文案（若新增提示语）。
- `app/src/pet/BubbleOverlay.js`、`app/src/settings.js`
  - 增加 Qoder 来源 Badge。

### 文档与隐私

- `docs/PRIVACY.md`
  - 增加 `~/.qoder/settings.json` 配置路径。
  - 说明 Qoder Hook payload 仅在本机传输。
- 可选：在 `docs/DESIGN.md` 的 Hook 适配器和支持 Agent 列表中补充 Qoder。

## 6. 隐私与安全边界

Qoder Hook payload 可能包含 Prompt、命令、文件路径、工具响应、错误信息和最后一条 Assistant 消息。实现应遵守 Familiar 的本地优先设计：

- 只通过本地 UDS 或 loopback TCP 传输，不增加遥测或远程请求。
- 不主动读取或保存 `transcript_path` 指向的会话文件。
- 日志只记录稳定的事件类型和来源，不记录 Prompt、命令、文件内容、响应或错误全文。
- Familiar 不对 Qoder 的工具操作做安全决策，不返回阻断结果。
- 守护进程离线时，Hook 命令仍成功退出，不能阻塞 Qoder。
- 将原始 payload 放入事件元数据时，确认现有持久化/日志链路不会把敏感字段落盘；如存在落盘路径，应先做字段裁剪或脱敏。

## 7. 测试与验收

### 单元和集成测试

至少覆盖：

- 注入 payload 包含五个事件。
- 用户已有配置和已有 Hook 不被覆盖。
- 重复注入不重复添加 Familiar 命令。
- 卸载只删除 `--source qoder`，不影响其他 Hook。
- 非法 JSON 和不可写配置返回错误且不破坏原文件。
- `session_id` 能稳定关联同一 Agent。
- `tool_input.command`、`tool_input.file_path` 能正确提取。
- Qoder 原生工具名和 Claude 兼容工具名得到相同状态。
- `PostToolUseFailure` 不会错误产生 `TaskFailed`。
- UDS 和 TCP 两条入口都能识别 `qoder`。

### 建议验证命令

```bash
cargo test -p familiar-hooks
cargo test -p familiar-core
cargo check -p familiar-hooks
cargo check -p familiar-cli
cd app && npm run build
git diff --check
```

### 手工验收

1. 在 Familiar 设置页注入 Qoder Hooks。
2. 重启 Qoder IDE 或插件。
3. 提交 Prompt，确认创建会话并显示任务描述。
4. 依次触发读文件、写文件、终端命令和搜索操作。
5. 触发一次失败命令，确认状态仍为工作中而不是任务失败。
6. 完成任务，确认显示完成状态并按现有策略清理。
7. 卸载 Qoder Hooks，确认用户自定义配置保持不变。

## 8. 分阶段实施

### Phase 1：协议和 Rust 链路

- 新增 `AgentSource::Qoder` 和 `QoderHook`。
- 完成配置注入、状态检测、预览和卸载。
- 修正通用适配器对 `tool_input` 和 Qoder 工具名的支持。
- 打通 CLI、UDS、TCP 的 `qoder` 来源映射。
- 添加 Rust 测试。

### Phase 2：桌面端管理

- 设置页增加 Qoder Hook 管理卡片。
- 增加中英文文案和来源 Badge。
- 完成 `npm run build` 和手工验收。

### Phase 3：兼容性增强

- 根据真实 Qoder payload 补充工具字段和异常事件。
- 评估是否需要独立的工具失败状态或 Stop 阻断能力。
- 若 Qoder 后续支持热加载或新的配置作用域，再扩展配置管理策略。

## 9. 风险与决策

| 风险 | 影响 | 应对 |
| --- | --- | --- |
| Qoder 文档中的字段与实际版本存在差异 | 工具名称或参数解析不完整 | 以兼容字段读取为主，保存脱敏样例并补充回归测试 |
| Qoder 不支持热加载 | 用户注入后看不到效果 | 注入成功后明确提示重启 Qoder |
| 缺少 SessionStart | 首个 Prompt 丢失时会话信息不完整 | 使用 `UserPromptSubmit`，并保留首个工具事件隐式创建兜底 |
| 工具失败被误认为任务失败 | UI 状态错误 | `PostToolUseFailure` 映射为继续工作状态 |
| 配置路径含空格 | Hook 命令无法执行 | 统一处理命令路径 Shell 转义或使用安全参数包装 |
| 原始 payload 包含敏感信息 | 隐私风险 | 不读 transcript，不写敏感日志，并审查元数据持久化链路 |

总体判断：Qoder 接入属于中等规模改动，配置协议可以复用现有 Claude Code 结构；真正需要优先处理的是 `tool_input` 解析、Qoder 原生工具名映射、`PostToolUseFailure` 语义，以及全链路来源注册。

## 10. 实施后本地配置审计

检查日期：2026-08-03

检查目标：`/Users/sam.gl/.qoder/settings.json`

依据：[Qoder Hooks 官方文档](https://docs.qoder.com/zh/extensions/hooks)。本次检查只读配置和运行结果，没有修改该文件。

### 10.1 总体结论

配置文件 JSON 格式有效，Familiar 注入的五个官方事件均已存在，Familiar CLI 路径存在且可执行；但当前配置不能称为“完全严格符合”官方协议，存在一个不在官方支持清单中的事件、一个当前版本不支持的字段，以及一个需要在 CLI 输出层收敛的协议差异。

### 10.2 已确认符合的部分

| 检查项 | 结果 |
| --- | --- |
| `UserPromptSubmit` | 存在，未设置 matcher，匹配所有 Prompt |
| `PreToolUse` | 存在，`matcher: "*"` 匹配所有工具 |
| `PostToolUse` | 存在，`matcher: "*"` 匹配所有工具 |
| `PostToolUseFailure` | 存在，`matcher: "*"` 匹配所有工具 |
| `Stop` | 存在，未设置 matcher，匹配所有停止事件 |
| Hook 类型 | 均使用官方要求的 `type: "command"` |
| 命令路径 | `/Applications/Familiar.app/Contents/Resources/bin/familiar-cli` 存在且可执行 |
| JSON 语法 | 已通过 `jq` 校验 |
| 事件运行结果 | 五个事件模拟调用均以退出码 `0` 返回，不阻断 Qoder |

配置中的 `Bash|bash|terminal|...|run_in_terminal` matcher 是合法正则。一个事件下存在多个 matcher 分组也符合官方的执行模型；对于终端工具，Familiar 通配 Hook 和已有的 `qoder-cli-hook.sh` 会按顺序都执行。

### 10.3 发现的问题

| 级别 | 位置 | 发现 | 影响 |
| --- | --- | --- | --- |
| 高 | `hooks.SessionStart` | Qoder 官方当前支持清单只有 `UserPromptSubmit`、`PreToolUse`、`PostToolUse`、`PostToolUseFailure`、`Stop`，未列出 `SessionStart` | 该 Hook 的行为未受官方保证，可能被忽略或在版本变化时失效 |
| 中 | 多个 Hook 的 `timeout: 15` | 官方配置表虽然列出 `timeout`，但明确说明当前版本不支持自定义超时 | `15` 秒不应被视为生效值，实际可能仍使用默认的 30 秒 |
| 中 | `familiar-cli` 的 `PreToolUse` 输出 | 当前 CLI 输出顶层 `{"decision":"allow"}`；官方精细控制格式是 `hookSpecificOutput.permissionDecision` | 退出码 `0` 仍可放行，但 stdout 不是官方推荐的精细控制结构 |

`SessionStart` 和 `bash /Users/sam.gl/.r2c/scripts/qoder-cli-hook.sh` 这组配置不是 Familiar 当前 `QoderHook` 注入 payload 的内容；Familiar 注入的五组命令与官方事件结构一致，应视为本地已有的其他 Hook。

### 10.4 建议收敛动作

1. 删除 `SessionStart`，或不要把它作为 Qoder 会话生命周期能力的依赖；Familiar 使用 `UserPromptSubmit` 创建会话。
2. 删除自定义 `timeout`，或者在脚本内部自行实现超时，不依赖 Qoder 当前未开放的配置字段。
3. 将 `PreToolUse` 的成功响应改为官方结构：

   ```json
   {
     "hookSpecificOutput": {
       "hookEventName": "PreToolUse",
       "permissionDecision": "allow",
       "permissionDecisionReason": "Familiar notified"
     }
   }
   ```

4. 修改配置后重启 Qoder；官方当前不支持 Hooks 热加载。

### 10.5 修复后复检

复检日期：2026-08-03

复检目标仍为 `/Users/sam.gl/.qoder/settings.json`，并使用当前安装的 `/Applications/Familiar.app/Contents/Resources/bin/familiar-cli` 对五个事件进行模拟调用。

本次已修复的运行协议问题：

- Qoder `PreToolUse` 现在返回官方结构 `hookSpecificOutput.permissionDecision: "allow"`，并包含 `hookEventName` 和 `permissionDecisionReason`。
- Qoder `Stop` 现在返回 `{}`，不再输出当前版本未定义的顶层 `decision` 字段。
- `UserPromptSubmit`、`PreToolUse`、`PostToolUse`、`PostToolUseFailure`、`Stop` 五个事件模拟调用均返回退出码 `0`。

本次仍未解决的配置问题：

- `hooks.SessionStart` 仍存在于 [settings.json](/Users/sam.gl/.qoder/settings.json:73)，不在官方当前五事件支持清单内。
- `timeout: 15` 仍存在于已有 `qoder-cli-hook.sh` 配置组中；官方说明当前版本不支持自定义超时，因此该值不能视为生效。

原因是 `QoderHook` 的重新注入采用合并策略，只追加缺失的 Familiar Hook，不会删除用户已有的事件或字段。因此本次重新注入已经修复了 CLI 返回协议，但不会自动清理上述历史配置。当前结论为：Familiar 五个 Hook 和运行时返回协议已符合官方文档，配置文件整体仍需手动移除 `SessionStart`，并处理自定义 `timeout` 后，才能称为完全收敛。
