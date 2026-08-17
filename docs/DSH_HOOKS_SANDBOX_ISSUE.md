# DSH Issue: dsh-hooks-claude-code hooks fail with STATUS_DLL_INIT_FAILED (0xC0000142) on Windows when the deployment sandbox default is not danger-full-access

> 提交给 DeepSeek Harness 的 issue 草稿 / Draft issue for DeepSeek Harness
> 仓库: https://github.com/deepseek-ai/deepseek-harness
> 相关包: `@deepseek-ai/dsh-hooks-claude-code`, `@deepseek-ai/dsh-pwsh-sandbox`, `@deepseek-ai/dsh-sandbox-windows-acl`

---

## 中文

### 标题

`dsh-hooks-claude-code` 在 Windows 上、部署沙箱默认非 `danger-full-access` 时，hook 命令以 `STATUS_DLL_INIT_FAILED (0xC0000142)` 失败

### 环境

- Windows 11 x64（含 GUI 宿主：DSH Desktop，Electron）
- `@deepseek-ai/dsh-hooks-claude-code@0.1.0-rc.6`
- `@deepseek-ai/dsh-hook-protocol@0.1.0-rc.6`
- `@deepseek-ai/dsh-pwsh-sandbox`（Windows 上作为 `ctx.shell` 组装）
- 本机未安装 PowerShell 7，`resolvePwshPath` 回退到 `C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe`（PS 5.1）

### 现象

hook 配置（Claude Code 方言）中任一 `type: "command"` hook 触发时：

- `hook/invoked` 与 `hook/result` 正常记录
- `hook/result` 的 `exitCode` 恒为 `3221225794`（`0xC0000142`，`STATUS_DLL_INIT_FAILED`），耗时约 85–90ms
- `stderrSummary` 为空；hook 命令体内的任何副作用（如 `[System.IO.File]::AppendAllText` 写诊断文件）**从未发生**——进程在命令开始执行前就已死于 DLL 初始化

### 触发条件

仅当**无会话的低层调用**（hooks 桥接）解析出的 `sandboxPolicy.mode` 不是 `danger-full-access` 时发生：

- `dsh-hooks-claude-code` 的 `runPoint` → `runHook`（`dsh-hook-protocol`）构造的 shell 请求**不携带 `sandboxPolicy`**
- `dsh-pwsh-sandbox.resolve()` 回退到 `this.ctx.sandboxPolicy.resolve()`（无 session 参数）→ 返回**部署默认** `defaultMode`
- 部署默认来自 `dsh-base` 的 `sandbox-policy` 条目：`mode: !!js process.env.DSH_PERMISSION_MODE ?? 'workspace-write'` → **`workspace-write`**
- `run()` 判定 `mode !== "danger-full-access"` → 走 `confine()` → Windows 上选中 `windows-acl` runner（受限令牌）
- **受限令牌下 spawn 的 `powershell.exe` 在 DLL 初始化阶段死亡，退出码 `0xC0000142`**

### 对比：为什么 DSH 模型工具不受影响

`dsh-tool-pwsh` 显式从**调用会话**解析权限并传给 shell：

```javascript
const resolveSandboxPolicy = (exec) =>
    sandboxPolicy?.resolve({ session: exec.agent.session })  // 会话级 danger-full-access
// ...
...policy !== void 0 ? { sandboxPolicy: policy } : {}
```

因此会话为 `danger-full-access` 时，工具执行不沙箱、完全正常。**hooks 桥接缺少同样的会话上下文传递**。

### 已排除的因素

以下均已在外部复现中验证**正常**（exit 0），说明问题不在 familiar-cli、命令格式、路径或用户权限：

| 复现场景 | 结果 |
|---|---|
| `windows-acl` runner（无 agent 模式，`--temp <tmpdir>`）直接运行 `powershell.exe -Command "familiar-cli hook ..."` | ✅ exit 0 |
| 同上 + `ENCODING_PREAMBLE` + `--write-sid/--temp-write-sid` seam 模式参数 | ✅ exit 0 |
| 从 DSH 主进程完整环境（PEB 读取 53 个变量，应用 `scrubbedParentEnv`）spawn runner | ✅ exit 0 |
| 无控制台宿主模拟（`windowsHide: true` / `CREATE_NO_WINDOW`） | ✅ exit 0 |
| 管理员（HIGH integrity）与标准用户（MEDIUM）上下文 | ✅ 均正常 |
| 直接运行 familiar-cli.exe（不经 shell，受限令牌下） | ✅ exit 0（familiar-cli 沙箱友好） |

唯一无法从外部复现的是 **DSH Desktop（Electron GUI）进程内部**的 spawn 链——提示差异与 Electron 内置 Node / 运行时环境相关，但核心判定成立：**问题出在 `windows-acl` 受限令牌 + `powershell.exe` 的组合**，且 hooks 桥接把 shell 请求置于该组合下。

### 期望行为 / 建议修复

1. **（首选）hooks 桥接携带会话权限**：`dsh-hooks-claude-code`（及 `dsh-hooks-codex`）在 `runPoint`/`runHook` 时，应像 `dsh-tool-pwsh` 一样解析并传入 `exec.agent.session` 的 `sandboxPolicy`（`{ session: exec.agent.session }`），使 hook 命令沿用当前会话的权限模式（用户已是 `danger-full-access` 时不应被降级沙箱）。
2. **（补充）`windows-acl` runner 兼容性**：即使保留 `workspace-write` 沙箱，也应排查为何受限令牌 + 无控制台 GUI 宿主下 `powershell.exe` 的 DLL 初始化会以 `0xC0000142` 死亡（README 已记录 `CREATE_NO_WINDOW`/`CREATE_NEW_CONSOLE` 下的同类边界；保活组（登录 SID + Everyone）在部分宿主场景下可能仍不足）。

### 影响

在部署沙箱默认非 `danger-full-access` 的环境里，**所有依赖 `powershell` 的 command hook 完全不可用**（每次触发即崩溃）。用户被迫把部署默认改为 `danger-full-access` 才能使用 hooks，削弱了默认保守的沙箱设计。这同时影响任何想在 Windows 上以默认 `workspace-write` 沙箱运行 hooks 的用户。

### 复现

1. Windows 11 + DSH Desktop，安装 `dsh-hooks-claude-code@0.1.0-rc.6`，`configPath` 指向含 `type: "command"` hook 的 JSON（如 `familiar-cli hook ...`）。
2. 保持 `sandbox-policy` 部署默认为 `workspace-write`（不设 `DSH_PERMISSION_MODE`）。
3. 在 DSH 会话中触发任一工具调用。
4. 观察 `hook/result`：`exitCode = 3221225794`。
5. 将部署默认改为 `danger-full-access`（或设置 `DSH_PERMISSION_MODE=danger-full-access`）后，hook 恢复 `exitCode = 0`。

---

## English

### Title

`dsh-hooks-claude-code` hooks fail with `STATUS_DLL_INIT_FAILED (0xC0000142)` on Windows when the deployment sandbox default is not `danger-full-access`

### Environment

- Windows 11 x64, GUI host: DSH Desktop (Electron)
- `@deepseek-ai/dsh-hooks-claude-code@0.1.0-rc.6`
- `@deepseek-ai/dsh-hook-protocol@0.1.0-rc.6`
- `@deepseek-ai/dsh-pwsh-sandbox` (mounted as `ctx.shell` on Windows)
- PowerShell 7 is not installed; `resolvePwshPath` falls back to `C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe` (PS 5.1)

### Symptom

With any `type: "command"` hook (Claude Code dialect) configured:

- `hook/invoked` and `hook/result` are recorded normally.
- `hook/result.exitCode` is always `3221225794` (`0xC0000142`, `STATUS_DLL_INIT_FAILED`), taking ~85–90 ms.
- `stderrSummary` is empty; **no side effect inside the hook command ever runs** (e.g. `[System.IO.File]::AppendAllText` never writes a file) — the process dies during DLL initialization before the command starts.

### Trigger condition

Only when the `sandboxPolicy.mode` resolved for the **session-less low-level caller** (the hooks bridge) is not `danger-full-access`:

- `dsh-hooks-claude-code`'s `runPoint` → `runHook` (`dsh-hook-protocol`) builds a shell request **without `sandboxPolicy`**.
- `dsh-pwsh-sandbox.resolve()` falls back to `this.ctx.sandboxPolicy.resolve()` (no session) → returns the **deployment default** `defaultMode`.
- The deployment default comes from `dsh-base`'s `sandbox-policy` entry: `mode: !!js process.env.DSH_PERMISSION_MODE ?? 'workspace-write'` → **`workspace-write`**.
- `run()` sees `mode !== "danger-full-access"` → `confine()` → the `windows-acl` runner (restricted token) on Windows.
- **`powershell.exe` spawned under the restricted token dies during DLL initialization with `0xC0000142`.**

### Contrast: why DSH model tools are unaffected

`dsh-tool-pwsh` explicitly resolves the policy from the **calling session** and passes it to the shell:

```javascript
const resolveSandboxPolicy = (exec) =>
    sandboxPolicy?.resolve({ session: exec.agent.session })  // session-level danger-full-access
// ...
...policy !== void 0 ? { sandboxPolicy: policy } : {}
```

So with a `danger-full-access` session, tool execution is unconfined and works fine. **The hooks bridge lacks the same session-context propagation.**

### Factors ruled out

All of the following reproduce **successfully** (exit 0) externally, showing the problem is not in the hook target binary, command quoting, paths, or user privilege:

| Reproduction | Result |
|---|---|
| `windows-acl` runner (agentless, `--temp <tmpdir>`) running `powershell.exe -Command "familiar-cli hook ..."` directly | ✅ exit 0 |
| Same + `ENCODING_PREAMBLE` + seam-mode `--write-sid/--temp-write-sid` args | ✅ exit 0 |
| Spawning the runner with the DSH main process's full environment (53 vars read via PEB, `scrubbedParentEnv` applied) | ✅ exit 0 |
| No-console host simulation (`windowsHide: true` / `CREATE_NO_WINDOW`) | ✅ exit 0 |
| Elevated (HIGH integrity) vs standard-user (MEDIUM) contexts | ✅ both fine |
| Running the hook binary directly (no shell) under the restricted token | ✅ exit 0 (binary is sandbox-friendly) |

The only chain that cannot be reproduced externally is inside the **DSH Desktop (Electron GUI) process** — pointing to Electron's bundled Node / runtime environment — but the core judgment stands: the fault lies in the **`windows-acl` restricted token + `powershell.exe`** combination, and the hooks bridge places shell requests under that combination.

### Expected behavior / suggested fix

1. **(Preferred) hooks bridge carries session policy**: `dsh-hooks-claude-code` (and `dsh-hooks-codex`) should resolve and pass `exec.agent.session`'s `sandboxPolicy` (i.e. `{ session: exec.agent.session }`) in `runPoint`/`runHook`, just as `dsh-tool-pwsh` does, so hook commands follow the current session's permission mode (a user who chose `danger-full-access` should not be silently sandboxed down).
2. **(Additionally) `windows-acl` runner compatibility**: even when keeping the `workspace-write` sandbox, investigate why `powershell.exe` DLL initialization dies with `0xC0000142` under the restricted token in a no-console GUI host (the README already documents the same boundary for `CREATE_NO_WINDOW` / `CREATE_NEW_CONSOLE`; the keep-alive groups (logon SID + Everyone) may be insufficient in some host scenarios).

### Impact

In deployments whose sandbox default is not `danger-full-access`, **every `powershell`-based command hook is completely unusable** (crashes on each trigger). Users are forced to set the deployment default to `danger-full-access` to use hooks, weakening the conservative default sandbox design. This affects anyone wanting to run hooks on Windows under the default `workspace-write` sandbox.

### Reproduction

1. Windows 11 + DSH Desktop, install `dsh-hooks-claude-code@0.1.0-rc.6`, point `configPath` at JSON containing a `type: "command"` hook (e.g. `familiar-cli hook ...`).
2. Keep the `sandbox-policy` deployment default at `workspace-write` (no `DSH_PERMISSION_MODE`).
3. Trigger any tool call in a DSH session.
4. Observe `hook/result`: `exitCode = 3221225794`.
5. After setting the deployment default to `danger-full-access` (or `DSH_PERMISSION_MODE=danger-full-access`), hooks return `exitCode = 0`.
