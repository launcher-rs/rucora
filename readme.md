# agentkit

Agentkit 是一个 Rust 生态的模块化 Agent SDK：

- `agentkit-core`：只包含抽象（trait/类型/错误/事件），便于第三方实现与长期兼容。
- `agentkit`：常用实现的聚合入口（provider/tools/skills/retrieval/config 等）。
- `agentkit-runtime`：默认运行时（tool-calling loop、streaming、policy/audit、trace）。
- 可选扩展：`agentkit-cli`、`agentkit-server`、`agentkit-mcp`、`agentkit-a2a`。

## 文档

- `docs/design.md`：设计思想、模块职责、关键数据流
- `docs/cookbook_config.md`：统一配置系统（YAML/TOML + profile + env 覆盖）
- `docs/examples.md`：示例索引与推荐用法

## 快速开始

### 1) 运行 CLI（可选）

```bash
cargo run -p agentkit-cli -- run --skill-dir skills --prompt "用一句话介绍 Rust" --trace-path trace.jsonl
```

说明：

- provider 来自 `AgentkitConfig::load()`（可通过环境变量配置）
- skills 从 `skills/` 目录加载并作为 tools 暴露给模型
- `--trace-path` 会把 `ChannelEvent` 写入 JSONL 轨迹文件

### 2) 启动 HTTP Server（可选）

```bash
cargo run -p agentkit-server
```

SSE 接口：`POST /v1/chat/stream`（输出 `ChannelEvent` JSON）。

## Workspace Crates

- `agentkit-core/README.md`
- `agentkit/README.md`
- `agentkit-runtime/README.md`
- `agentkit-cli/README.md`
- `agentkit-server/README.md`
- `agentkit-mcp/README.md`
- `agentkit-a2a/README.md`

