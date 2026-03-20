# agentkit-server（HTTP Server，可选）

`agentkit-server` 把 agent 暴露为 HTTP 服务，方便接 UI 或其它系统。

## 特性

- **SSE streaming**：服务端持续输出 `ChannelEvent` JSON
- **可插拔**：provider/skills 仍来自 `agentkit::config` 与 skills loader

## 启动

```bash
cargo run -p agentkit-server
```

环境变量：

- `AGENTKIT_SERVER_ADDR`：监听地址（默认 `127.0.0.1:8080`）
- `AGENTKIT_SKILL_DIR`：skills 目录（默认 `skills`）

## API

### `GET /health`

返回：`{"ok": true}`

### `POST /v1/chat/stream`

请求体：

```json
{
  "messages": [{"role": "user", "content": "...", "name": null}],
  "metadata": null
}
```

响应：SSE，每条 `data:` 是一个 `ChannelEvent` 的 JSON 序列化。
