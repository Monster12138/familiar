# Agent Hook 点 → 宠物状态 → 气泡状态

本文档记录 Familiar 当前实现中，各 Agent 原生 Hook 点经过事件归一化后，
如何影响 Agent 状态、桌面宠物动画和气泡展示。

> 基线：`origin/main`，提交 `0e00206`。
>
> 本文描述的是默认映射。用户可以通过
> `renderer.desktop-pet.event_status_map` 覆盖 `AgentEventType` 到
> `AgentStatus` 的默认映射；覆盖后，宠物心情和气泡图标会跟随最终状态变化。

## 状态定义

### Agent 状态与宠物动画

状态机先为每个 Agent 计算 `AgentStatus`，再根据所有活跃 Agent 聚合出
`FamiliarMood`。下表中的“宠物状态”同时列出这两个层次。

| AgentStatus | 聚合后的 FamiliarMood | 默认宠物动画 | 说明 |
| --- | --- | --- | --- |
| `Idle` | `Idle` | `idle` | 用户提交消息后新一轮请求开始时的默认状态；超过睡眠超时后全局可能变为 `Sleepy`。 |
| `Thinking` | `Thinking` | `thinking` | Agent 正在思考。 |
| `Working` | `Busy` | `working` | 正在执行工具、读写文件、搜索或浏览。 |
| `Pending` | `Watching` | `watching` | 等待用户输入或权限决定。 |
| `Completed` | `Celebrating` | `celebrating` | 完成态会短暂保留，随后清理该 Agent。 |
| `Failed` | `Alarmed` | `alarmed` | 错误优先级最高，会压过其他 Agent 的心情。 |
| `SessionStarted` / 无状态变化 | 重新聚合现有 Agent | 不单独改变 | `SessionStarted` 不创建可见 Agent；默认的 `SubagentStart`/`SubagentStop` 保持已有状态。 |

### 气泡状态

气泡由 `app/src/bubble.js` 和 `app/src/pet/BubbleOverlay.js` 渲染，最多显示最近活跃的
3 个 Agent。

| 最终 AgentStatus | 气泡是否展示 | 气泡图标 | 气泡内容 |
| --- | --- | --- | --- |
| `Idle` / `Pending` | 是 | 灰色省略号 | 用户指令；活动文本分别通常为 `Started session` 或 `Waiting for user input...`。 |
| `Thinking` / `Working` | 是 | 旋转指示器 | 用户指令和当前活动，例如 `Thinking...`、`Tool finished` 或具体工具活动。 |
| `Completed` | 是 | 绿色对勾 | 用户指令和完成摘要，通常为 `Task finished` 或 Agent 返回的摘要。 |
| `Failed` | 否 | 不适用 | `bubble.js` 当前只筛选 `Thinking`、`Working`、`Completed`、`Pending`、`Idle`；失败信息不进入气泡。 |
| 无状态变化 | 保持原气泡 | 保持原图标 | 事件本身不会更新状态或活动文本。 |

## Claude Code

配置文件：`~/.claude/settings.json`。工具型 Hook 使用 `matcher: "*"`。

| Hook 点 | 归一化事件 | 默认宠物状态 | 气泡状态 |
| --- | --- | --- | --- |
| `SessionStart` | `SessionStarted` | 保持当前宠物状态 | 保持当前气泡和图标；不创建可见 Agent。 |
| `UserPromptSubmit` | `AgentStarted` | `Idle` → `Idle / idle` | 展示省略号；显示用户指令，活动为 `Started session`。 |
| `PreToolUse` (`*`) | 按工具细分为 `RunningCommand`、`WritingFile`、`ReadingFile`、`SearchingCode`、`BrowsingWeb` 或 `Processing` | `Working` → `Busy / working` | 展示旋转指示器；显示具体工具活动。 |
| `PostToolUse` (`*`) | `Processing`（`Tool finished`） | `Working` → `Busy / working` | 展示旋转指示器；活动为 `Tool finished`。 |
| `PostToolUseFailure` (`*`) | `Processing`（`Tool failed`） | `Working` → `Busy / working` | 展示旋转指示器；活动为 `Tool failed`，不会进入 `Failed`。 |
| `PermissionRequest` (`*`) | `WaitingForInput` | `Pending` → `Watching / watching` | 展示省略号；活动为 `Waiting for user input...`。 |
| `SubagentStart` | `SubagentStarted` | 默认无状态变化 | 保持当前气泡和图标。 |
| `SubagentStop` | `SubagentStopped` | 默认无状态变化 | 保持当前气泡和图标。 |
| `Stop` | `TaskCompleted` | `Completed` → `Celebrating / celebrating` | 展示绿色对勾；活动为 `Task finished` 或完成摘要。 |
| `StopFailure` | `TaskFailed` | `Failed` → `Alarmed / alarmed` | 不展示气泡；失败信息仅保留在 Agent 状态/活动数据中。 |
| `SessionEnd` | `TaskCompleted` | `Completed` → `Celebrating / celebrating` | 展示绿色对勾；随后按完成态保留时间清理。 |

## Codex

配置文件：`~/.codex/hooks.json`。当前注入的工具型 Hook 未额外写入 matcher。

| Hook 点 | 归一化事件 | 默认宠物状态 | 气泡状态 |
| --- | --- | --- | --- |
| `SessionStart` | `SessionStarted` | 保持当前宠物状态 | 保持当前气泡和图标；不创建可见 Agent。 |
| `UserPromptSubmit` | `AgentStarted` | `Idle` → `Idle / idle` | 展示省略号；显示用户指令，活动为 `Started session`。 |
| `PreToolUse` | 按工具细分为 `RunningCommand`、`WritingFile`、`ReadingFile`、`SearchingCode`、`BrowsingWeb` 或 `Processing` | `Working` → `Busy / working` | 展示旋转指示器；显示具体工具活动。 |
| `PostToolUse` | `Processing`（`Tool finished`） | `Working` → `Busy / working` | 展示旋转指示器；活动为 `Tool finished`。 |
| `Stop` | `TaskCompleted` | `Completed` → `Celebrating / celebrating` | 展示绿色对勾；活动为 `Task finished` 或完成摘要。 |
| `SessionEnd` | `TaskCompleted` | `Completed` → `Celebrating / celebrating` | 展示绿色对勾；随后按完成态保留时间清理。 |

## Antigravity

配置文件：`~/.gemini/config/hooks.json`，注入到顶层 `familiar` 节点。
工具型 Hook 使用 `matcher: "*"`。

| Hook 点 | 归一化事件 | 默认宠物状态 | 气泡状态 |
| --- | --- | --- | --- |
| `PreInvocation` | `Thinking` | `Thinking` → `Thinking / thinking` | 展示旋转指示器；活动为 `Thinking...`。 |
| `PostInvocation` | `Thinking` | `Thinking` → `Thinking / thinking` | 展示旋转指示器；活动为 `Thinking...`。 |
| `PreToolUse` (`*`) | `RunningCommand`（包括非命令工具，活动文本为 `Using tool <name>`） | `Working` → `Busy / working` | 展示旋转指示器；显示工具活动。 |
| `PostToolUse` (`*`) | `Processing`（`Tool finished`） | `Working` → `Busy / working` | 展示旋转指示器；活动为 `Tool finished`。 |
| `Stop` | `TaskCompleted` | `Completed` → `Celebrating / celebrating` | 展示绿色对勾；活动为 `Task finished` 或完成摘要。 |

适配器还能够解析 `SessionStart`（`SessionStarted`）和 `SessionEnd`
（`TaskCompleted`），但它们不在当前 Antigravity 注入 payload 中，因此不属于当前
默认安装的 Hook 点。

## Qoder

配置文件：`~/.qoder/settings.json`。工具型 Hook 使用 `matcher: "*"`。

| Hook 点 | 归一化事件 | 默认宠物状态 | 气泡状态 |
| --- | --- | --- | --- |
| `SessionStart` | `SessionStarted` | 保持当前宠物状态 | 保持当前气泡和图标；不创建可见 Agent。 |
| `UserPromptSubmit` | `AgentStarted` | `Idle` → `Idle / idle` | 展示省略号；显示用户指令，活动为 `Started session`。 |
| `PreToolUse` (`*`) | 按工具细分为 `RunningCommand`、`WritingFile`、`ReadingFile`、`SearchingCode`、`BrowsingWeb` 或 `Processing` | `Working` → `Busy / working` | 展示旋转指示器；显示具体工具活动。 |
| `PermissionRequest` (`*`) | `WaitingForInput` | `Pending` → `Watching / watching` | 展示省略号；活动为 `Waiting for user input...`。 |
| `PostToolUse` (`*`) | `Processing`（`Tool finished`） | `Working` → `Busy / working` | 展示旋转指示器；活动为 `Tool finished`。 |
| `PostToolUseFailure` (`*`) | `Processing`（`Tool failed`） | `Working` → `Busy / working` | 展示旋转指示器；活动为 `Tool failed`，不会进入 `Failed`。 |
| `SubagentStart` | `SubagentStarted` | 默认无状态变化 | 保持当前气泡和图标。 |
| `SubagentStop` | `SubagentStopped` | 默认无状态变化 | 保持当前气泡和图标。 |
| `Stop` | `TaskCompleted` | `Completed` → `Celebrating / celebrating` | 展示绿色对勾；活动为 `Task finished` 或完成摘要。 |
| `SessionEnd` | `TaskCompleted` | `Completed` → `Celebrating / celebrating` | 展示绿色对勾；随后按完成态保留时间清理。 |
| `PreCompact` | `Processing`（活动为 `PreCompact`） | `Working` → `Busy / working` | 展示旋转指示器；活动为 `PreCompact`。 |
| `Notification` | 未识别事件的 fallback `Processing`（活动为 `Notification`） | `Working` → `Busy / working` | 展示旋转指示器；活动为 `Notification`。 |

## DeepSeek Harness

配置文件：`~/.dsh/familiar-hooks.json`，由 DeepSeek Harness bridge 读取。
工具型 Hook 使用 `matcher: "*"`。

| Hook 点 | 归一化事件 | 默认宠物状态 | 气泡状态 |
| --- | --- | --- | --- |
| `SessionStart` | `SessionStarted` | 保持当前宠物状态 | 保持当前气泡和图标；不创建可见 Agent。 |
| `UserPromptSubmit` | `AgentStarted` | `Idle` → `Idle / idle` | 展示省略号；显示用户指令，活动为 `Started session`。 |
| `PreToolUse` (`*`) | 按工具细分为 `RunningCommand`、`WritingFile`、`ReadingFile`、`SearchingCode`、`BrowsingWeb` 或 `Processing` | `Working` → `Busy / working` | 展示旋转指示器；显示具体工具活动。 |
| `PostToolUse` (`*`) | `Processing`（`Tool finished`） | `Working` → `Busy / working` | 展示旋转指示器；活动为 `Tool finished`。 |
| `Stop` | `TaskCompleted` | `Completed` → `Celebrating / celebrating` | 展示绿色对勾；活动为 `Task finished` 或完成摘要。 |
| `SubagentStart` | `SubagentStarted` | 默认无状态变化 | 保持当前气泡和图标。 |
| `SubagentStop` | `SubagentStopped` | 默认无状态变化 | 保持当前气泡和图标。 |

## 维护提示

- Hook 点以各 Agent 的 `get_injection_payload()` 为准；不要把适配器能够兼容的
  额外事件误认为已经注入的事件。
- `SessionStart` 只记录会话初始化，不创建或刷新可见 Agent；用户感知的生命周期从
  `UserPromptSubmit` 开始。
- 事件状态覆盖只改变最终 `AgentStatus`，聚合心情和气泡图标会按最终状态重新计算。
- `Failed` 当前会触发宠物告警动画，但不会出现在气泡列表中；如果需要在气泡中展示
  失败信息，需要同时调整 `app/src/bubble.js` 的状态过滤逻辑。
- `Completed` 不是永久 Agent：状态机按 `celebration_secs` 保留后移除，所以完成气泡也
  只会短暂显示。
