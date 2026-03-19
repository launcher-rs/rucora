# agentkit TODO

> 作为基础库（core + runtime + 可选实现集合），这里记录“还缺什么能力 / 为什么要做 / 做到什么算完成”。

## 0. 总体目标（Why）

- **可组合**：core 只提供稳定抽象；runtime 提供可插拔编排；agentkit 提供常用实现与示例。
- **可扩展**：用户能在不 fork 的情况下接入自定义 Provider / Tool / Skill / Memory / Retrieval。
- **可运行**：给出“开箱即用”的默认组合（最小可用 + 生产可用配置）。
- **可观测**：生产环境可定位问题（日志/trace/metrics/调试工具）。
- **可控与安全**：工具调用、文件/网络访问要有策略与审计，避免“默认全开”。

---

## 1. Provider 能力补全

- [x] **统一 Streaming 接口（core）**
  - **目标**：让 runtime/agent loop 能以同一套方式消费流式 token、tool_calls、事件。
  - **落点**：`agentkit-core/src/provider/*`
  - **验收**：至少 1 个 provider 实现能流式输出；runtime 能边流式边执行 tool loop。

- [x] **Provider 的重试/退避/超时/取消（runtime）**
  - **目标**：生产可用的健壮性（网络抖动、速率限制、长响应）。
  - **落点**：`agentkit-runtime`（调用 provider 的统一封装层）
  - **验收**：可配置 max_retries、backoff、timeout；取消能中断流式。

- [ ] **Provider 连接池/多 Provider 路由（runtime/agentkit）**
  - **目标**：按模型能力、价格、限流动态选择 provider；支持 fallback。
  - **落点**：`agentkit/src/provider/*` + `agentkit-runtime`
  - **验收**：给出 RouterProvider 示例；具备健康检查与降级策略。

- [ ] **结构化输出（JSON / Schema）能力（core + provider 实现）**
  - **目标**：减少 prompt 解析成本；让工具输入/业务输出更稳定。
  - **落点**：`agentkit-core/provider/types.rs`（补充 response_format / schema）
  - **验收**：至少一个 provider 支持严格 JSON 输出并在测试中覆盖。

---

## 2. Tool Calling / Agent Loop 能力

- [ ] **可插拔 AgentLoop（runtime）**
  - **目标**：Simple / ToolCalling / ReAct / Plan-and-Execute 等 loop 可替换。
  - **落点**：`agentkit-runtime`（抽象 loop trait + 默认实现）
  - **验收**：新增 `AgentLoop` trait；现有 `ToolCallingAgent` 迁移为某个 loop 实现。

- [ ] **Tool 调用并发策略（runtime）**
  - **目标**：支持并行工具调用、最大并发限制、队列与超时。
  - **落点**：`agentkit-runtime`
  - **验收**：模型一次请求多个 tool_calls 时可并发执行；结果顺序可控。

- [x] **工具安全策略（policy）与审计（runtime + tools）**
  - **目标**：默认安全；支持 allowlist/denylist（路径、域名、命令、Git 操作等）。
  - **落点**：`agentkit-runtime`（policy hook）+ `agentkit/src/tools/*`
  - **验收**：拒绝策略有明确错误；记录审计事件（谁调用了什么，参数摘要）。

- [x] **Cmd/Shell 工具危险命令拦截 + 白名单/黑名单（tools + runtime）**
  - **目标**：避免 `cmd_exec`/`shell` 被提示注入利用；默认阻止破坏性命令（rm/del/format/registry 等）并允许显式放行。
  - **落点**：`agentkit/src/tools/cmd_exec.rs`、`agentkit/src/tools/shell.rs` + `agentkit-runtime` policy
  - **验收**：
    - 默认规则能拦截常见危险命令与高风险参数组合
    - 支持按“命令名/子命令/参数模式/工作目录”配置 allowlist/denylist
    - 被拦截时返回结构化错误（包含命中规则 id 与原因）并记录审计事件

- [x] **工具输出限制与截断标准化（core/runtime）**
  - **目标**：避免巨大输出拖垮上下文；统一截断字段与提示。
  - **落点**：`agentkit-core/tool/types.rs` 或 runtime wrapper
  - **验收**：所有内置 tools 都遵循统一的 `truncated` / `max_bytes` 协议。

---

## 3. Memory / Retrieval / RAG

- [x] **内置 Memory 实现（agentkit）**
  - **目标**：提供最小可用的短期/长期记忆实现（InMemory、File/SQLite）。
  - **落点**：`agentkit/src/*`（新增 memory 实现模块）
  - **验收**：在 examples 中演示：写入记忆、检索、在对话中引用。

- [ ] **向量库适配器（agentkit）**
  - **目标**：让 `VectorStore` 有落地实现（如 sqlite + cosine、或接入外部向量库）。
  - **落点**：`agentkit/src/retrieval/*`
  - **验收**：至少提供 1 个本地实现 + 1 个远程实现（可选）。

- [ ] **Embedding Provider 抽象与实现（core + agentkit）**
  - **目标**：RAG 需要稳定的 embedding 接口与缓存。
  - **落点**：`agentkit-core/src/embed/*` + `agentkit/src/embed/*`
  - **验收**：embedding 支持批量、缓存、维度校验。

- [ ] **RAG 管线（chunking / indexing / retrieval / cite）（agentkit）**
  - **目标**：提供“从文档到可检索知识”的标准流程与引用格式。
  - **落点**：`agentkit/src/retrieval/*` + `agentkit/src/tools/*`
  - **验收**：给出完整示例：导入文件 -> 建库 -> 问答 -> 返回引用片段。

---

## 4. Skills 生态（脚本化/打包/分发）

- [ ] **Skill 打包规范与版本管理（skills + runtime）**
  - **目标**：skills 不只是本地目录；需要可发布/可升级/可复现。
  - **落点**：`skills/` 约定 + `agentkit/src/skills/*` loader
  - **验收**：定义 skill manifest（name/version/deps/capabilities）；loader 校验并报错清晰。

- [ ] **Rhai skill 的宿主 API 标准库（agentkit）**
  - **目标**：脚本 skill 需要稳定的内置函数集（call_tool、http、fs、log、json 等）。
  - **落点**：`agentkit/src/skills/*`（注册器默认实现）
  - **验收**：不写自定义 registrar 也能跑通示例 skills。

- [ ] **技能测试框架（agentkit）**
  - **目标**：对 skill 输出做断言（输入 -> 输出），并支持 mock 工具。
  - **落点**：`agentkit/tests` + `agentkit/src/skills/*`
  - **验收**：至少 2 个 skill 有稳定单测；可在 CI 中运行。

---

## 5. Channels / 事件系统

- [ ] **统一事件模型（token / tool / skill / memory / debug）（core + runtime）**
  - **目标**：GUI/CLI 可订阅事件；支持流式可视化与审计。
  - **落点**：`agentkit-core/src/channel/*` + runtime 发射事件
  - **验收**：tool 执行、provider token、错误都能以事件形式输出。

- [ ] **回放（replay）与轨迹持久化（runtime）**
  - **目标**：调试/评估/对比运行结果；支持最小可复现。
  - **落点**：`agentkit-runtime`
  - **验收**：一次 run 能导出 trace JSON；可从 JSON 回放关键步骤。

---

## 6. 配置与可用性（DX）

- [ ] **统一配置系统（env + file + profile）（agentkit）**
  - **目标**：用户少写 glue code；不同环境可切换（dev/prod）。
  - **落点**：`agentkit/src/*`（config 模块）
  - **验收**：示例支持从 `AGENTKIT_*` 和 yaml/toml 读取 provider、tools、policies。

- [ ] **错误类型分层与可诊断性（core）**
  - **目标**：用户能区分 provider/tool/runtime 错误并做策略处理。
  - **落点**：`agentkit-core/src/error.rs`
  - **验收**：错误携带 kind、source、可选的 retriable 标记。

- [ ] **示例与 cookbook 补全（examples + docs）**
  - **目标**：让“怎么用”比“有哪些 trait”更直观。
  - **落点**：`agentkit/examples` + `docs/`
  - **验收**：至少覆盖：tool calling、skills loader、RAG、policy、streaming。

---

## 7. 测试、基准与兼容性

- [ ] **契约测试（core traits 的行为约束）（core + runtime）**
  - **目标**：第三方实现不会悄悄破坏 runtime 预期。
  - **落点**：`agentkit-core/tests` 或 `agentkit-runtime/tests`
  - **验收**：提供一组 trait contract tests（Tool/Provider/VectorStore）。

- [ ] **基准（bench）与性能护栏（workspace）**
  - **目标**：tool loop、JSON 序列化、检索等关键路径可量化。
  - **落点**：criterion benches
  - **验收**：至少 3 个基准；CI 可选跑。

---

## 8. 可选：CLI / Server / 集成

- [ ] **官方 CLI（可选 crate）**
  - **目标**：快速试用、运行 skills、执行 agent loop、导出 trace。
  - **落点**：新增 `agentkit-cli`（独立 crate）
  - **验收**：`agentkit run --skill-dir skills/ --provider ...` 可跑通。

- [ ] **HTTP Server 模式（可选 crate）**
  - **目标**：把 agent 暴露为服务（SSE/WS streaming），方便接 UI。
  - **落点**：新增 `agentkit-server`（axum 等）
  - **验收**：SSE 输出 token/tool events；支持取消请求。

- [x] **MCP（Model Context Protocol）客户端/服务端适配（可选 crate）**
  - **目标**：接入外部 MCP 工具与资源，把 agentkit 作为“工具宿主/工具消费者”。
  - **落点**：新增 `agentkit-mcp`（或 `agentkit-integrations`）
  - **验收**：
    - 可将 MCP tools 映射为 `agentkit_core::tool::Tool`
    - 支持 stdio 与 http(s) 传输（至少一种）
    - 具备权限策略对接（域名/路径/命令/资源访问）

- [x] **A2A（Agent-to-Agent）通信与协作（可选 crate / runtime 扩展）**
  - **目标**：多 agent 协作（delegation、handoff、共享记忆/工具）成为一等能力。
  - **落点**：`agentkit-core/src/channel/*` + `agentkit-runtime`（多 agent orchestrator）
  - **验收**：
    - 定义最小消息协议（task、result、event、cancel）
    - 支持本地 in-process 与可选远程传输（例如 http/websocket）
    - 提供一个 multi-agent 示例（leader/worker）并可导出统一 trace
