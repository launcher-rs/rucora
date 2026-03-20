# 示例（Examples）索引与推荐用法（中文）

本页列出仓库内的示例入口，并说明各示例想解决的问题。

## 配置 + Streaming + Trace

- 文件：`agentkit/examples/config_stream_trace_demo.rs`
- 适用：你想用一套统一配置启动 provider，并把 streaming 过程导出为可回放的 trace。
- 运行要点：
  - 通过 `AGENTKIT_CONFIG`/`AGENTKIT_PROFILE` 指定配置文件/配置 profile
  - 可通过 `AGENTKIT_TRACE_PATH` 指定 trace 输出路径

## Skills：从目录加载并作为 Tool 暴露

- 文件：`agentkit/examples/skill_read_local_file_demo.rs`
- 适用：你想从 `skills/` 目录加载技能（Rhai/Command），并让模型在 tool loop 中自动选择调用。

## HTTP 工具调用示例

- 文件：`agentkit/examples/http_summarize_demo.rs`
- 适用：你想演示工具调用（http_request）并让模型“先获取网页再总结”。

## CLI 快速试用

- crate：`agentkit-cli`
- 适用：你只想在命令行里快速跑一次：加载 config + skills，然后问一句问题。

示例：

```bash
cargo run -p agentkit-cli -- run --skill-dir skills --prompt "北京今天怎么样？" --trace-path trace.jsonl
```

## Server（SSE）服务化

- crate：`agentkit-server`
- 适用：你想把 agent 暴露成服务，前端/其它服务通过 SSE 接收事件流。

启动：

```bash
cargo run -p agentkit-server
```

请求：

```bash
curl -N http://127.0.0.1:8080/v1/chat/stream \
  -H "Content-Type: application/json" \
  -d '{"messages":[{"role":"user","content":"用一句话介绍 Rust","name":null}],"metadata":null}'
```
