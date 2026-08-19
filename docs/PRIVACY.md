# Privacy & Data Handling

Familiar is a local-first desktop application. This document describes what
the current Alpha implementation actually reads, keeps, logs, and modifies.

## No Remote Telemetry

Familiar does not send captured prompts, commands, file paths, transcripts, or
usage telemetry to a remote service. Agent hook events are delivered to the
desktop process over a Unix domain socket or TCP loopback connection.

Opening a project or system-settings link is an explicit user action and uses
the operating system's normal URL handler.

## Data Read and Processed in Memory

To render the current Agent state, Familiar may process the following local
data supplied by supported hook APIs:

- session or conversation identifiers;
- the latest user instruction or task description;
- command text and tool names;
- file paths, search queries, URLs, summaries, and error descriptions;
- Antigravity transcript files referenced by hook payloads, from which the
  latest relevant user instruction is extracted.

These values are translated into structured in-memory events and may be shown
in the desktop UI. The application does not copy or persist a full transcript.

## Persistence and Retention

The current Alpha desktop application keeps Agent activity state in process
memory only. Quitting the application clears that state. Although
`familiar-core` contains an experimental SQLite storage abstraction, it is not
wired into the desktop application and no Agent event-retention job currently
runs.

The `data_retention_days` setting is reserved for that future persistence
feature and has no effect in the current release. The versioned default is 90
days, but no event history is presently stored for it to delete.

The `[cleanup]` config section drives the built-in data-cleanup module
(Settings → 数据清理 / Data Cleanup). It can remove the timestamped `.bak.*`
backups Familiar creates before editing hook configs, and Familiar's own
operational logs in the temporary directory. Cleanup is manual-only and always
requires confirmation; nothing is deleted automatically.

User preferences are stored locally in:

```text
~/.config/familiar/config.toml
```

## Operational Logs

The desktop and server processes create operational logs in the operating
system temporary directory (`familiar_tauri.log` and `familiar_server.log`).
These logs record operational metadata such as event kind, session identifier,
and current mood. They do not record raw hook payloads, prompts, command text,
file paths, search queries, URLs, summaries, or error text.

Temporary logs follow the operating system's temporary-file lifecycle. They
can also be deleted manually while Familiar is not running.

## Hook Configuration Changes

Hook installation and removal are explicit actions in the Familiar settings
UI. Before changing an existing file, Familiar creates a timestamped backup
named `familiar-*.bak.<timestamp>`; the `familiar-` prefix marks it as
Familiar's own so the data-cleanup module never touches backups created by
other tools.
Depending on the selected integration, Familiar may update:

- Claude Code: `~/.claude/settings.json`
- Codex: `~/.codex/hooks.json`
- Antigravity: `~/.gemini/config/hooks.json`
- Qoder: `~/.qoder/settings.json`

Removing a hook through the settings UI removes Familiar-owned hook entries;
it does not delete unrelated Agent configuration or transcript history.

## Clearing Local Familiar Data

1. Quit Familiar to clear all in-memory Agent activity.
2. Use Settings → 数据清理 / Data Cleanup to remove hook-config backups and
   Familiar's operational logs in one step.
3. Remove `~/.config/familiar/config.toml` to reset saved preferences.
4. Use the settings UI to uninstall Agent hooks before removing the app.

## Security Reports

Please follow [`SECURITY.md`](../SECURITY.md) and do not include real prompts,
credentials, or proprietary source code in a public issue.
