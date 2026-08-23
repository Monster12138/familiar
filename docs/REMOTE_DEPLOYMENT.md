# 远程模式部署指南

本文说明如何将 Familiar 服务端和 Agent 部署在一台服务器上，并让本地桌面端以
`remote` 模式订阅状态。默认情况下 Familiar 仍然是本地一体化模式；只有需要把
Agent 和桌面 UI 分离时才使用本方案。

## 1. 部署拓扑

```text
远程服务器
  Agent
    -> familiar-cli hook
    -> UDS / 127.0.0.1:19527
    -> familiar-cli serve
    -> StateMachine
    -> HTTP/WebSocket :19528
                         ^
                         | HTTPS/WSS（推荐）
                         |
本地机器               familiar 桌面端（只订阅状态）
```

服务端和 Agent 必须在同一台机器上。远程模式不是 Hook Relay：桌面端不能把本机
Agent 的 Hook 事件转发到远端，也不能通过桌面端远程修改服务器上的 Agent 配置。

端口职责如下：

| 端口 | 用途 | 是否暴露公网 |
| --- | --- | --- |
| `19527` | Hook ingest TCP（仅监听 `127.0.0.1`） | 否 |
| `19528` | HTTP/WebSocket API、State Stream 和 Hooks 状态接口 | 按需 |

Unix 系统优先使用 UDS；TCP 仅作为回退路径。生产部署建议把 Hook ingest 保持在
loopback 或 UDS，只开放 API 端口。

## 2. 安装服务端

建议创建专用的系统用户，并让该用户同时运行 Agent 和 Familiar 服务端；这样它可以
安全地访问自己的 Agent 配置和 Hook socket：

```bash
sudo useradd --system --create-home --home-dir /var/lib/familiar \
  --shell /usr/sbin/nologin familiar
```

如果 Agent 必须由其他用户运行，请不要把该用户的配置复制给 `familiar`，而应以实际
Agent 用户执行后续 Hook 安装，并按该用户调整配置文件和 socket 权限。

将 `familiar-cli` 安装到服务器，例如：

```bash
sudo install -m 0755 target/release/familiar-cli /usr/local/bin/familiar-cli
sudo install -d -m 0755 /etc/familiar /etc/familiar/tls
```

如果从源码构建：

```bash
cargo build --release -p familiar-cli
```

配置文件应从仓库的 [config/default.toml](../config/default.toml) 复制后修改，
不要只创建一个缺少其他必需配置段的 TOML 文件：

```bash
sudo install -m 0640 config/default.toml /etc/familiar/server.toml
sudo chown familiar:familiar /etc/familiar/server.toml
sudo install -d -o familiar -g familiar -m 0700 /var/lib/familiar/auth
```

下面只列出远程服务端需要调整的配置段。其他渲染、通知和清理配置可以按部署需要
保留默认值。

```toml
[hooks]
enabled = ["claude-code", "codex", "qoder"]
socket_path = "/run/familiar/familiar.sock"
tcp_port = 19527

[server]
bind = "0.0.0.0:19528"

[server.tls]
enabled = true
cert_path = "/etc/familiar/tls/server.crt"
key_path = "/etc/familiar/tls/server.key"

[server.auth]
enabled = true
token_file = "/var/lib/familiar/auth/token"
auto_generate = true

[server.state_stream]
max_updates_per_second = 10
max_task_summary_chars = 160
max_activity_summary_chars = 160
```

如果只在可信内网、SSH tunnel、Tailscale 或 WireGuard 内使用，可以将
`server.tls.enabled` 设为 `false`；此时客户端也必须将 `remote.tls` 设为 `false`。
跨公网部署建议始终开启 TLS。TLS 关闭时 token 和状态数据都会以明文传输。

### 2.1 TLS 证书

`cert_path` 和 `key_path` 必须指向 PEM 格式证书和私钥。TLS 开启但文件缺失或无效时，
`familiar-cli serve` 会启动失败，不会自动降级为明文 HTTP。

证书的主机名必须覆盖客户端配置中的 `remote.endpoint`。如果使用自签名证书，需在
客户端所在系统信任该 CA；不要为了绕过证书校验而把服务暴露到公网。

### 2.2 认证 Token

服务端使用持久化 Token 文件作为 Bearer Token。首次启动时，如果 `token_file` 不存在且
`auto_generate = true`，`familiar-cli serve` 会生成随机 Token，创建权限为 `0600` 的文件，
之后每次启动都复用同一个 Token。原始 Token 不会写入日志或 TOML。

```bash
# 可选：提前显式初始化，避免服务首次启动时才创建
sudo -u familiar familiar-cli auth init --config /etc/familiar/server.toml

# 在需要配置客户端时读取；该命令输出敏感信息，请勿记录或共享
sudo -u familiar familiar-cli auth show --config /etc/familiar/server.toml
```

服务端只读取 `server.auth.token_file`，不再兼容旧的服务端 Token 环境变量配置。客户端使用
`remote.token_file` 读取本机独立凭据文件，不把 Token 写入配置文件。

## 3. 安装 Agent Hooks

Hook 必须以运行 Agent 的用户身份安装，不能用 root 为其他用户修改 `~/.claude`、
`~/.codex` 等配置。服务端配置文件使用绝对路径时，必须把同一个路径传给 CLI：

```bash
sudo -u familiar familiar-cli hooks preview \
  --agent codex \
  --config /etc/familiar/server.toml

sudo -u familiar familiar-cli hooks install \
  --all \
  --config /etc/familiar/server.toml

sudo -u familiar familiar-cli hooks status \
  --json \
  --config /etc/familiar/server.toml
```

注入的命令会自动携带 `--config /etc/familiar/server.toml`，因此 Agent 后续执行
`familiar-cli hook` 时会连接到同一份服务端配置中的 UDS/TCP ingest。注入过程会备份
原始 Agent 配置，并且重复执行是幂等的。

如果 Agent 使用的是另一位系统用户，请切换为该用户执行安装，并确保该用户可以：

- 读取 `/etc/familiar/server.toml`；
- 执行 `/usr/local/bin/familiar-cli`；
- 访问 `/run/familiar/familiar.sock`（如果使用 UDS）。

## 4. 使用 systemd 运行服务端

创建 `/etc/systemd/system/familiar.service`：

```ini
[Unit]
Description=Familiar remote state server
After=network.target

[Service]
Type=simple
User=familiar
Group=familiar
RuntimeDirectory=familiar
RuntimeDirectoryMode=0750
ExecStart=/usr/local/bin/familiar-cli serve --config /etc/familiar/server.toml
Restart=on-failure
RestartSec=3
NoNewPrivileges=true
PrivateTmp=true

[Install]
WantedBy=multi-user.target
```

然后启动并检查日志：

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now familiar.service
sudo systemctl status familiar.service
sudo journalctl -u familiar.service -f
```

`RuntimeDirectory=familiar` 会创建 `/run/familiar`，与上面的 UDS 路径匹配。若使用
自定义 socket 目录，需要提前创建目录并授予 Agent 运行用户访问权限。

## 5. 配置本地桌面端

在本地机器的 Familiar 配置文件中保留完整配置，只修改运行模式和远程连接段：

```toml
[runtime]
mode = "remote"

[remote]
endpoint = "familiar.example.com:19528"
path = "/api/v1/state-stream"
tls = true
token_file = "/Users/you/.config/familiar/remote-token"
connect_timeout_secs = 10
reconnect_initial_secs = 1
reconnect_max_secs = 30
```

推荐直接在桌面端设置面板填写 Token。客户端会将 Token 保存到独立凭据文件，配置文件只
保存 `remote.token_file` 路径，已有 Token 不会在界面中回显。手工配置时可使用上面的
`token_file` 路径。

```bash
cd app
npm run tauri dev
```

生产安装包直接启动桌面端即可。修改配置后需要重启桌面端，使运行模式和远程连接
参数重新加载。

`remote.path` 目前应保持为 `/api/v1/state-stream`。远程 Hooks 只读状态接口固定为
`/api/v1/hooks/status`，反向代理必须同时转发这两个路径。

远程模式下：

- 桌面端通过 WSS/WS 订阅最新完整状态快照；
- 中间快照允许丢失，客户端以最后收到的有效快照为准；
- 本地 CPU、内存、磁盘指标仍然描述桌面端所在机器；
- Hooks 面板只显示服务端注入状态，不显示服务器配置路径，也不提供远程写操作。

## 6. 验证部署

将持久化 Token 临时读入当前 shell，仅用于下面的验证请求：

```bash
export REMOTE_TOKEN="$(sudo -u familiar familiar-cli auth show --config /etc/familiar/server.toml)"
```

先验证服务端健康检查：

```bash
curl --fail --cacert /etc/familiar/tls/ca.crt \
  -H "Authorization: Bearer $REMOTE_TOKEN" \
  https://familiar.example.com:19528/health
```

再验证 Hooks 状态接口：

```bash
curl --fail --cacert /etc/familiar/tls/ca.crt \
  -H "Authorization: Bearer $REMOTE_TOKEN" \
  https://familiar.example.com:19528/api/v1/hooks/status
```

预期返回每个支持 Agent 的 `injected` 布尔值，例如：

```json
{
  "claude-code": { "injected": true },
  "codex": { "injected": true },
  "qoder": { "injected": false }
}
```

最后可以在服务端执行一次安全的合成 Hook 测试：

```bash
sudo -u familiar familiar-cli hooks test \
  --agent codex \
  --event UserPromptSubmit \
  --config /etc/familiar/server.toml
```

桌面端启动后，检查日志中是否出现 `connected to remote state stream`，并确认桌面宠物
可以看到服务端 Agent 状态变化。

## 7. 常见问题

### Hooks 状态接口返回 401

- 检查服务端 Token 文件是否存在且可由 `familiar` 用户读取：
  `sudo -u familiar familiar-cli auth show --config /etc/familiar/server.toml`；
- 检查客户端 `remote.token_file` 是否指向包含同一个 Token 的独立文件；
- 不要把 Token 写入配置文件、日志或提交到仓库。

### 服务端启动时报 UDS bind 失败

- 确认 `/run/familiar` 已存在且服务用户可写；
- 确认没有另一个 `familiar-cli serve` 占用同一个 socket；
- 检查旧 socket 文件是否属于已停止的进程，停止服务后再清理它。

### Hook 已安装但服务端没有事件

- 用 `familiar-cli hooks status --json --config ...` 确认安装用户和 Agent 用户一致；
- 检查注入命令是否包含正确的 `--config` 路径；
- 确认 Agent 用户能访问 UDS，或服务端配置了 loopback TCP `19527`；
- 查看 `journalctl -u familiar.service`，确认 Hook listener 已启动。

### 客户端一直重连

- `remote.endpoint` 只填写 `主机:端口`，不要重复写 `http://` 或 `https://`；
- `remote.tls` 必须与服务端 TLS 配置和反向代理实际协议一致；
- 防火墙只需要允许客户端访问 API 端口 `19528`，不应开放 Hook ingest 端口；
- 如果使用反向代理，确认 `/api/v1/state-stream` 支持 WebSocket Upgrade。

## 8. 安全边界

远程模式只同步经过长度限制的渲染状态摘要，不提供完整 prompt、transcript、文件内容、
Hook payload 或命令输出的远程查询接口。Hooks 注入、卸载和配置预览必须在运行 Agent
的服务器上通过 `familiar-cli hooks` 执行。

如果需要“Agent 在本地、状态服务在远端”，这属于尚未实现的远程 Hook Relay 场景，不能
直接套用本文档。
