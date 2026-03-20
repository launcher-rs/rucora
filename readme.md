# agentkit

Agentkit 是一个 Rust 生态的模块化 Agent SDK：

- `agentkit-core`：只包含抽象（trait/类型/错误/事件），便于第三方实现与长期兼容。
- `agentkit`：常用实现的聚合入口（provider/tools/skills/retrieval/config 等）。
- `agentkit-runtime`：默认运行时（tool-calling loop、流式事件、policy、trace）。
- 可选扩展：`agentkit-cli`、`agentkit-server`、`agentkit-mcp`、`agentkit-a2a`。

## 为什么使用 Runtime 而不是 Agent

很多框架（例如 LangChain、rig-core）使用 “Agent” 来指代一个带有 prompt/tools/memory/loop 的高层对象。
在本项目中，我们选择把“执行流程/编排策略”的最小稳定接口定义为 `Runtime`：

- **避免概念重复**：`Agent::run` 与 `Runtime::run` 在签名上完全等价，保留两者会造成调用方困惑。
- **更适合可插拔编排**：不同 loop（tool-calling、planner/executor、router、budget 等）本质上是不同的运行时策略。
  用 `Runtime` 作为唯一执行抽象，便于组合、装饰与替换。
- **core 更轻、更稳定**：`agentkit-core` 只保留最小执行入口 `Runtime` 与统一事件模型 `ChannelEvent`，避免在 core 层绑定具体“智能体”语义。

如果你想在业务层使用 “Agent” 这个概念，推荐在上层（例如 `agentkit` crate）用 struct/builder 封装配置，
内部持有一个 `Runtime` 实现即可。

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

