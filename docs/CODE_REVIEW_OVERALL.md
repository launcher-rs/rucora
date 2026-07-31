# 整体代码审查报告（rucora 工作区）

> 审查日期：2026-07-31
> 审查范围：rucora 工作区全部 10 个 crate + examples + 工程化配置（约 38k 行 Rust）
> 审查方法：按四维度并行深度审查（core 抽象层 / agent 实现 / providers+tools / 外围 crate+工程化），全部结论基于当前 git HEAD 逐行核验
> 严重程度：高（会产生 bug/panic/安全缺口）、中（功能缺失/设计缺陷）、低（文档/工程规范）

---

## 目录

1. [高风险（12 项）](#高风险)
2. [中风险（20 项）](#中风险)
3. [低风险（工程化）](#低风险工程化)
4. [修复优先级建议](#修复优先级建议)

---

## 高风险

### H1. 工具调用循环的历史消息回归（影响所有工具 Agent）

- **文件**：`rucora/src/agent/execution.rs:671-780`
- **问题**：`_run_loop` 的 `ChatMode::Single` 分支在调用 LLM 之后**从未把 `response.message`（含 tool_calls 的 assistant 消息）追加到 `messages`**（缺失处 769-777）。对比：`MapAll` 分支有 `messages.push(response.message)`（execution.rs:813），`Reduce` 分支有 push（execution.rs:837），唯独 Single 缺失。这是回归（此前版本有保存）。
- **后果链**：
  1. 第二轮循环开始时 `remove_orphaned_tool_messages`（execution.rs:60-108）把历史中所有 tool 结果消息判为"孤儿"删除；
  2. LLM 每轮看到的仍是 `[system, user]`，看不到上一轮 tool_calls 与结果 → 极大概率重复调用同一工具直至 `max_steps` / `LoopDetector::Break` 兜底；
  3. 启用 `conversation_manager` 时，工具调用路径下用户/assistant/工具结果全部不落盘，多轮历史永久丢失。
- **影响**：所有基于工具调用循环的 Agent（ToolAgent/ReActAgent/ReflectAgent）在 `run()` 非流式路径下的多步工具推理实际不可用。
- **建议**：Single 分支在 `_execute_tool_calls` 之前补 `messages.push(response.message.clone())`；并对工具调用路径的 conversation_manager 保存逻辑补写。

### H2. 流式路径旁路所有增强能力与中间件

- **文件**：`rucora/src/agent/stream_engine.rs:221-224`、`tool_execution.rs:181-189`、`execution.rs:1241-1254`
- **问题**：流式路径执行工具使用 `execute_tool_call_with_policy_and_observer`，内部传入**空的 `MiddlewareChain::new()`**（tool_execution.rs:187-189）；`StreamEngine` 结构体（stream_engine.rs:34-55）没有 `enhanced_config`/`enhanced_runtime`/`middleware_chain` 字段，`stream_engine()` 工厂（execution.rs:1241-1254）也未透传。
- **后果**：流式路径（`run_stream_simple`/`run_stream_text`）下工具执行静默丢失重试、超时、熔断、缓存能力；中间件钩子 `process_tool_call_before/after` 完全不生效；同一 Agent 的 `run()` 与 `run_stream()` 行为不一致。
- **建议**：给 `StreamEngine` 增加三个字段，`stream_engine()` 透传；stream_engine.rs:221 改用 `execute_tool_call_enhanced`。

### H3. Anthropic 流式工具调用参数永远为空

- **文件**：`rucora-providers/src/anthropic.rs:632-647`（content_block_delta 处理）
- **问题**：Anthropic Messages 流式协议中工具参数通过 `content_block_delta` 的 `delta.input_json_delta`（JSON 字符串增量）分片推送，`content_block_start` 只含 `id`/`name` 不含 `input`。代码只提取 `delta.text`，**完全不处理 `input_json_delta`**。结果流式工具调用被 yield 成 `input: {}`，参数全部丢失（非流式 `chat` 是正确的，270-299 行）。附带缺陷：未跟踪 `content_block.index`，多个并发工具调用参数会串扰。
- **建议**：按 `index` 维护 `BTreeMap<usize, ToolCallAccumulator>`，在 `content_block_delta` 追加 `input_json_delta`，`content_block_stop` 时组装完整 `ToolCall`。

### H4. Gemini 流式模式：工具调用完全缺失 + SSE 格式不匹配

- **文件**：`rucora-providers/src/gemini.rs:530-673`、`parse_tool_calls` 行 294-322
- **问题**：`stream_chat` 只对每个候选调用 `extract_text_content` 提取 `parts[].text`，完全不解析 `functionCall`（流式工具调用被静默丢弃，比 H3 更彻底）。且请求用 `:streamGenerateContent?alt=sse`（行 542）但解析假定"每行一个完整 JSON 对象"（行 640-647），与真实 SSE `data: ` 前缀格式不符；同时未处理 `finishReason` 与 `usageMetadata`。
- **建议**：补 `functionCall` 解析；按真实 SSE 格式（`data: ` 前缀剥离）解析；透出 finish_reason 与 usage。

### H5. 4 个 OpenAI 兼容 provider 流式工具调用未实现

- **文件**：`deepseek.rs:491-499`、`moonshot.rs:492-499`、`openrouter.rs:541-548`、`azure_openai.rs:533-540`
- **问题**：四个 provider 的 `stream_chat` 仅解析 `choices[0].delta.content` 文本增量，不处理 `delta.tool_calls`（对比 openai.rs:730-755 的 `BTreeMap<usize, (id, name, args)>` 增量拼装）。流式工具调用被丢弃，且与 openai.rs 行为不一致。
- **建议**：把 openai.rs:730-755 的增量拼装逻辑提取到 helpers 复用。

### H6. HTTP 错误映射分裂，6/8 provider 丢失错误分类与可重试性

- **文件**：仅 `openai.rs:86-116`、`ollama.rs:58-88` 正确实现 `map_reqwest_error`/`map_http_error`（401/403→Authentication、429→RateLimit、超时→Timeout、连接→Network、其余→Api）。
- 其余 6 个 provider 全部把错误压成 `ProviderError::Message`：`anthropic.rs:424,446-450`、`gemini.rs:457,479-483`、`deepseek.rs:337,360-362`、`moonshot.rs:338,361-363`、`openrouter.rs:387,410-412`、`azure_openai.rs:374,397-399`。
- **后果**：`ProviderError::is_retriable()` 对 `Message` 恒 false → `ResilientProvider` 对这 6 个 provider 的 5xx/429 **永不重试**；`retry_after` 丢失；401/403 不被分类为 Authentication，`ErrorClassifier`/故障切换失效。
- **建议**：把两函数提取到 `http_config.rs`/`helpers.rs` 公共函数，8 个 provider 统一调用；HTTP 错误优先读取响应体 `error`/`message` 字段。

### H7. SSE `[DONE]` 处理缺陷（死检查 + 流不终止 + CRLF 不兼容）

- **文件**：`openai.rs:711-712,799-802`、`anthropic.rs:583-585,664-666`、`openrouter.rs:534-536,560-562`、`deepseek.rs:484-486`（无外层）、`moonshot.rs:484-486`（无外层）、`azure_openai.rs:525-528`（无外层）
- **问题**：
  1. 事件文本在被检查前已被 `drain`/切片移除，外层 `if buf.contains("[DONE]")` 永远为 false，是死检查；
  2. 收到终止标记后外层 `bytes_stream.next()` 继续读，keep-alive 连接下残留事件仍被解析 yield（重复/垃圾 chunk）；
  3. deepseek/moonshot/azure 无外层兜底，流终止完全依赖连接关闭，持久连接下挂起；
  4. `buf.find("\n\n")` 不兼容 `\r\n\r\n`（HTTP/1.1 允许 CRLF）。
- **建议**：抽共享 SSE 解析器；`[DONE]` 命中设终止标志立即结束外层循环；用 `\r?\n` 感知分隔。

### H8. UTF-8 字节截断 panic（5 处）

- **文件**：
  - `rucora-mcp/src/tool.rs:233`（`&s[..800]`）、`:279`（`&s[..1200]`）——线上可触发，大输入/大结果日志构造
  - `examples/rucora-deep-research/src/research_agent.rs:243`（`&search_summary[..6000]`）、`:294`（`&search[..8000]`）、`:301`（`&deep_read[..8000]`）
  - `examples/rucora-deep-research/src/config.rs:273`（`&api_key[..4]`）
- **问题**：截断对象是 LLM 生成/网页抓取的任意文本，必含多字节字符。字节索引落在字符中间即 panic，可触发崩溃。
- **建议**：统一改用 `floor_char_boundary(n)` 或封装 `truncate_utf8(s, max)`。

### H9. `#[rucora_guard]` 宏与 trait 签名完全不匹配（用即编译失败）

- **文件**：`rucora-macros/src/lib.rs:305-329`、`rucora-core/src/injection_guard_trait.rs:90-98`
- **问题**：实际 trait 是**同步、两个参数** `fn scan(&self, content: &str, source: &str) -> ScanResult`；宏生成的是 `async fn scan(&self, content: &str) -> Result<ScanResult, AgentError>` + 多余的 `name()` 方法 → 三重编译错误（E0407/E0053）。且宏文档示例引用的 `ScanResult::Blocked`/`Threat::new` 在核心类型中不存在（实际是 `{ is_safe, threats, ... }` 结构体）。全仓库无任何测试/示例使用该宏，属无人验证的死代码。
- **建议**：按实际 trait 重写宏（同步 2 参数），修正文档示例，补编译测试。

### H10. 安全缺口（工具层）

- **S1 ShellTool 黑名单可绕过且误伤合法用法**：`rucora-tools/src/system/shell.rs:35-47,113-160`。`contains(forbidden)` 子串匹配：`rm -r -f` 绕过 `rm -rf` 检查；`curl` 误伤 `uncurl`；`contains("..")` 拦截 `git checkout ..`/`cd ..`；`DANGEROUS_OPERATORS` 含 `"\\"` 拦截所有 Windows 路径参数。而命令实际用 `Command::new(...).args(...)` 直连不经 shell，`|`/`&&`/`$(` 不会造成注入——检查既拦不了真实向量又拦大量正常用法。
- **S2 CmdExecTool 白名单可 SSRF/写文件**：`cmd_exec.rs:33-64,127-132`。白名单 `curl` 但不禁 `-o`/`-O`（可写任意路径）；无 URL 校验（`curl http://169.254.169.254/...` 打云元数据）。
- **S3 HttpRequestTool 响应大小限制失效**：`http.rs:232-243`、`fetch.rs:110-120`。先 `response.bytes().await` 整读再与 5MB 比较，超大响应先耗尽内存。
- **S4 BrowseTool 无 SSRF 校验**：`web/browse.rs:76-103`。直接 `client.get(url).send()`，不调 `validate_public_http_url`（同 crate http.rs:145-150、fetch.rs:90 都校验）。
- **S5 ShellTool 环境变量泄漏**：`shell.rs:331-333` 只清 3 个密钥，本仓库的 `OPENAI_API_KEY`、`ANTHROPIC_API_KEY`、`GOOGLE_API_KEY`、`SERPAPI_API_KEY` 等全部泄漏给子进程（对比 GitTool `env_clear()` + 白名单做对了，git.rs:316-322）。
- **S6 SSRF DNS 重绑定窗口与覆盖不全**：`web/security.rs:8-25,76-93`。先校验后 reqwest 重新解析（TOCTOU）；IPv6 漏 `is_site_local`；域名后缀黑名单漏 `.internal`/`.lan`/`.home`/`.corp`/`.arpa`/`nip.io` 通配服务。
- **S7 HttpRequestTool max_redirects 死字段**：`http.rs:51-52,190,192`。重定向策略硬编码 `Policy::none()`，`max_redirects` 字段/setter 是无效配置。

### H11. 损坏的宏 / 未实现的能力（skills）

- **PermissionsConfig 只解析不执行**：`rucora-skills/src/config.rs:98-119`。`network`/`filesystem`/`commands`/`allowed_domains`/`denied_domains` 五个字段无任何执行逻辑引用，`SkillExecutor` 直接启动任意脚本（loader.rs:363-620）。用户配置了权限却产生"已受保护"的错觉。
- **CachedSkillLoader 缓存未命中永远返回 None**：`rucora-skills/src/cache.rs:136-146`。miss 路径不调用底层 loader、不回填缓存，`CachedSkillLoader` 实际不可用。

### H12. 核心 trait 潜伏 bug（当前无消费方，一旦启用即触发）

- **interrupt.rs TOCTOU 丢失唤醒竞态**：`rucora-core/src/interrupt.rs:82-87`。`wait_for_interrupt` 先查 `interrupted()` flag 再 `notify.notified().await`；`interrupt()`（120-123）用 `notify_waiters()` 不保留 permit，中断发生在两者之间则等待者**永久挂起**。现有测试无法捕获。
- **ShutdownHandle::is_shutdown() 恒 false**：`rucora-core/src/graceful_shutdown.rs:48-54`。trait 默认实现恒 false，与 `ShutdownToken::is_shutdown()` 两种口径。
- **run_batch(max_concurrency=0) panic**：`rucora-core/src/agent/mod.rs:757`。`buffer_unordered(0)` 在 futures-util 中 `assert!(limit > 0)`。内部消费点都有 `.max(1)` 防护，公开 trait 方法无校验。
- **指数退避 jitter 非随机 + NaN 边界**：`rucora-core/src/retry.rs:157-163`。`Instant::now()` 后立即 `elapsed()`，`nanos % jitter_range` 几乎退化为 `delay - 0.1*delay`，抖动不随机，"雷鸣羊群"问题依旧；`delay==0` 时 `0.0 % 0.0 = NaN`。
- **RetryAction 枚举死代码**：`rucora-core/src/retry.rs:52-59`。全工作区无任何消费（仅文档引用）。

---

## 中风险

### M1. ReAct/Reflect 多轮决策退化为单次调用

- **文件**：`react.rs:85-97,148-206`、`reflect.rs:88-106`、`execution.rs:655`
- **问题**：① `_run_loop` 构造上下文硬编码 `tool_results: Vec::new()`（execution.rs:655），react.rs:90-92 的 observe 分支永远不可达；② 结合 H1（assistant 消息不落盘），ReAct 步骤 1 后 LLM 面对空洞历史；③ reflect 的 think 依赖 `step / 2` 奇偶切换阶段，但 execution.rs Single 分支对纯文本响应**立即 return**（execution.rs:739-766），而 reflect 提示词未要求调工具 → 整个"生成-反思-改进"退化为一次调用。
- **建议**：修复 H1 后重测；react 提示词显式要求工具调用；reflect 阶段切换改为响应驱动（检测到完成标记再 Return）。

### M2. SummaryAgent 流式/批处理不一致 + 死 max_steps

- **文件**：`summary.rs:262,268,272-334,354-363,706`
- **问题**：`run()` 走分块-合并（map-reduce）逻辑，`run_stream()` 注释自认"不包含分块-合并"直接委托 `run_stream_simple`，同一 Agent 两种调用产生不同质量输出；`run_stream_text`（370-377）实际调用的也是 `run()`。局部 `max_steps` 变量只被写进 `info!` 日志不参与控制流，执行器侧硬编码 `.with_max_steps(10)`，构建器无配置入口。
- **建议**：统一流式与批处理语义；`max_steps` 接入控制流或移除。

### M3. reflect.rs off-by-one（最终 Return 不可达）

- **文件**：`reflect.rs:89,94-96,448`、`execution.rs:642`
- **问题**：`max_steps = max_iterations * 2`，而 `_run_loop` 每轮开头先检查 `step >= max_steps` 再调 `think`。以 `max_iterations=3`（max_steps=6）推演：step 6 时先命中 MaxStepsExceeded，reflect.rs:94 的 `iteration >= max_iterations → Return` 永远没机会执行 → 完整迭代必以错误结束，`_build_final_result`（209-224）成死代码。
- **建议**：reflect.rs:448 改为 `max_iterations * 2 + 1`，或 execution.rs:642 改为 `step > max_steps`。

### M4. run_batch detached 任务泄漏

- **文件**：`rucora-core/src/agent/mod.rs:742-767`
- **问题**：每个输入 `tokio::task::spawn`（754）detached 任务。调用方 `select!`/`timeout`/drop 取消 `run_batch` 后，已 spawn 任务不取消、继续后台运行至完成——无 cancellation token、无 JoinHandle abort、无清理。批量任务可能继续消耗 API 配额、执行有副作用的工具。
- **建议**：持 JoinHandle 并在 drop 时 abort，或引入取消令牌。

### M5. ConcurrencyConfig 死代码

- **文件**：`tool_call_config.rs:370-407,614,639-642`、`execution.rs:935,1043`
- **问题**：`ConcurrencyConfig` 提供完整"按工具名设置独立并发数"API，但 `get_concurrency` 只在其单元测试中被调用（712-714）。执行侧并发只读 `self.max_tool_concurrency`，`.with_concurrency(...)` 配置无任何效果。与 AGENTS.md"dead_code 拒绝"方针相悖。
- **建议**：接入 execution.rs 并发路径，或移除公开 API。

### M6. enable_tool_logging 只写不读

- **文件**：`execution.rs:318,404,474-477`
- **问题**：字段被赋值、被 setter 写入，但任何地方都不读取；`with_tool_logging(false)` 无效；默认 true 且无 Agent 构建器暴露；`stream_engine()` 工厂也不传递。
- **建议**：在 tool_execution.rs 日志点（226-237、419-432）接入开关，或移除。

### M7. 并发执行路径竞态

- **文件**：`execution.rs:1007-1126`
- **问题**：① 并发任务各自对共享 `Arc<std::sync::Mutex<LoopDetector>>` 调 `record`，重复计数顺序不确定，串行的 Warning→Block→Break 语义在并发下不可预测；② 熔断"检查→执行→记录"非原子，并发任务可集体通过 `can_pass` 后集体失败；③ 缓存击穿：同输入并发 miss 冗余执行；缓存键依赖 JSON 序列化顺序不稳定；④ `std::sync::Mutex` 在 async 任务中阻塞锁。
- **建议**：并发路径收集结果后统一检测；熔断加原子化；缓存 per-key 锁。

### M8. 多条 system 消息只保留第一条

- **文件**：`anthropic.rs:197-202`、`gemini.rs:209-214`
- **问题**：从消息列表 `find` 第一条 `Role::System` 作为顶层 system 字段，其余被过滤丢弃。多段 system 指令静默丢失。
- **建议**：合并拼接或全部保留。

### M9. ResilientProvider.stream_chat 无重试 + 超时丢失分类

- **文件**：`resilient.rs:377-391,325-331`
- **问题**：`stream_chat` 直接透传不套 `RetryConfig`，与 `chat`（316-375）语义不一致，流式无重试；`chat` 超时包装成 `ProviderError::Message` 丢失 `Timeout` 分类，`should_retry` 识别不了。
- **建议**：流式套用重试配置；超时分支返回 `ProviderError::Timeout`。

### M10. feature 门控形同虚设

- **文件**：`rucora-retrieval/Cargo.toml:14-18` + `lib.rs:9-15`、`rucora-embed/Cargo.toml:14-18` + `lib.rs:8-12`
- **问题**：embed 声明 `openai`/`ollama`/`all` 特性但模块无条件编译；retrieval 的 `qdrant`、`chroma_persistent` 没有任何对应特性却永远被编译，`all` 也不含 qdrant。无法裁剪依赖。
- **建议**：每个模块加 `#[cfg(feature)]`，`all` 补齐全部特性。

### M11. ChromaPersistentStore 同步 I/O + 非原子落盘

- **文件**：`rucora-retrieval/src/chroma_persistent.rs:111-131,180-225,302-315`
- **问题**：async 方法内同步 `fs::*` 阻塞 tokio 线程；`save_to_disk` 先持读锁序列化、释放锁后才写文件，期间数据可变，落盘内容与内存态不一致；无临时文件+rename 原子写；每次 upsert 全量写盘 O(n)。
- **建议**：`tokio::fs` 或 `spawn_blocking`；临时文件+rename；脏标记/批量落盘。

### M12. A2A protocol/transport 空模块

- **文件**：`rucora-a2a/src/protocol.rs:1`、`transport.rs:1`、`lib.rs:13-21`
- **问题**：lib.rs 声明 `pub mod protocol; pub mod transport;` 并声称定义协议模型/传输层，实际为空。用户按文档引用拿到空模块。
- **建议**：补齐 `pub use ra2a::*` 转出，或删除空模块并修正文档。

### M13. `From<String> for AgentInput` 空文本 panic

- **文件**：`rucora-core/src/agent/mod.rs:321-325`
- **问题**：`From<String>` 对空文本 `.expect()` panic，而 `AgentInput::new` 对空文本返回 `Err`（222）。同一约束两种行为。
- **建议**：`From` 改为 `AgentInput::new(s).unwrap_or_default()` 或文档明确 panic。

### M14. route.rs "openai compatible" 死分支

- **文件**：`rucora-core/src/provider/route.rs:57-58`
- **问题**：先 `to_lowercase().replace(['-','_',' '],"")` 剥离空格，再比较 `s == "openai compatible"`——空格已被剥离，该分支永不可命中。
- **建议**：先比较再剥离，或比较处理后的 "openaicompatible"。

### M15. 两套同名 SkillDefinition / SkillContext

- **文件**：`rucora-core/src/skill/mod.rs:14-36,143-148` vs `skill/types.rs:149-169,210-216`
- **问题**：两套同名类型（扁平 vs 嵌套），types.rs 版本仅在文档注释中被引用，实践中为死类型。且 `Skill` trait（skill_trait.rs:165-302）根本不使用这两套类型——`run_value` 直接接受 `Value`。
- **建议**：删除 types.rs 死类型，或让 Skill trait 真正消费。

### M16. ToolError::Timeout retriable 不一致

- **文件**：`rucora-core/src/error.rs:464`
- **问题**：`ToolError::Timeout` → `retriable: false`（注释"工具超时通常不应该重试"），而 `SkillError::Timeout`、`AgentError::Timeout`、`ChannelError::Timeout`、`ProviderError::Timeout` 均为 true。超时语义上唯一例外。
- **建议**：确认设计意图，统一或文档说明。

### M17. run_with 的 `Self: Sized` 与文档矛盾

- **文件**：`rucora-core/src/agent/mod.rs:847`
- **问题**：`run_with` 有 `Self: Sized` 约束，但注释（824）声称"这是实现 dyn 兼容的关键方法"——dyn 调用不可用，矛盾。`AgentExecutor::run_stream`（866）仅接收 `input` 无 `&dyn Agent`，不支持 Agent 决策。

### M18. 工具层重复/职责不清

- **文件**：`cmd_exec.rs:43-64` vs `shell.rs:113-160`
- **问题**：两套"命令安全校验"互不相同且规则冲突（shell 禁 curl、cmd_exec 只允许 curl）；`D1-D7` providers 目录存在 5 份 `build_response_format`/`parse_tool_calls`/`build_tools` 逐字拷贝、6 处 SSE 循环复制、8 份构造样板；`ollama.rs:178-216` 手写与 `helpers.rs:38-76` 几乎相同的 build_messages；`openai.rs:355-399` 手写采样参数插入而 helpers.rs:95-136 已有 `apply_sampling_params`。

### M19. 测试缺口

- **文件**：`rucora-core/src/test_utils.rs:10-27`
- **问题**：`MockProvider` 固定返回文本、无工具调用、`stream_chat` 返回空流。各 Agent 单元测试全部只测构建器（builder 后即丢弃），没有测试调用 run()/工具循环/流式/循环检测。extractor.rs:676-764 已示范"按行为枚举的 MockProvider"范式但未推广到 execution/stream_engine。H1 这类回归至今未被发现。
- **建议**：扩展 MockProvider 支持脚本化 tool_calls/流式 chunk，补工具循环、并发执行、历史持久化、孤儿清理、context overflow、流式工具路径测试。

### M20. 构建器 API 不一致

- **文件**：execution.rs:474、simple.rs、chat.rs、tool.rs、react.rs、reflect.rs、summary.rs、extractor.rs
- **问题**：`with_` 前缀不统一（chat.rs:286 `max_history_messages` 无前缀，summary.rs:564-643 全无前缀）；`tool_registry`（tool.rs:307）独立于 `tool`/`tools`（289-304）；ReAct/Reflect 无 `max_tool_concurrency`；`temperature/top_p/...` 一套方法在 7 处重复复制易漂移；ReAct/Reflect 存储 `_provider`/`_system_prompt`/`_conversation_manager` 冗余字段（下划线掩盖死代码），ToolAgent 的 `_max_steps` 只存不用。

---

## 低风险（工程化）

### E1. 完全缺失 CI

- 仓库无 `.github/workflows`，`cargo check`、`cargo test --workspace`、`cargo clippy --all-targets`、`cargo deny check` 无人自动执行。deny.toml/clippy.toml 配置存在但无自动化运行。
- **建议**：新增 `.github/workflows/ci.yml`（check + test + clippy `-D warnings` + deny check + 缓存）。

### E2. 文档与代码脱节

- `readme.md:76,83`、`docs/README.md:71,78` 使用已删除的 `DefaultAgent`（CHANGELOG.md:16 记录 0.2.0 已移除）；
- `examples/README.md:11-29` 引用的 `basic-chat`/`tool-calling` 等二进制目标全部不存在；
- `CHANGELOG.md` 停留在 0.2.0（现版本 0.4.0），且 0.2.0 条目声称"禁用 dead_code"与当前 `Cargo.toml:54` deny 矛盾；
- `readme.md:117-136` 死链接（`docs/CHANGELOG.md`、`docs/INDEX.md`）、项目结构列出不存在的 `rucora-runtime`/`rucora-cli`，漏列 mcp/a2a/embed/retrieval/skills；
- `rucora-skills/README.md:34,100-103,117,69-88` 多处 API 与实现不符（错误导入、`.tools()/.build()` vs 实际 `.tool_registry()/.try_build()`、列出不存在的 file_skills 子模块）。

### E3. 缺少工具链/格式化配置

- 无 `.rustfmt.toml`（AGENTS.md 声称 4 空格/100 列但未固化）、无 `rust-toolchain.toml`、workspace 无 `rust-version`（MSRV）声明（edition 2024 依赖较新编译器）。

### E4. lint 配置问题

- `Cargo.toml:78` `uninlined_format_args` 已被较新 clippy 重命名为 `inline_format_args`，deny 不存在的 lint 会产生 unknown-lint 警告；
- `deny.toml:12` `db-path` 不是 `[advisories]` 合法键；`:20` `notice = "deny"` 遇任何公告即 fail，通常应 `warn`。

### E5. 未使用依赖

- `rucora-skills/Cargo.toml:20` 声明 `rucora-tools` 但源码无引用；`examples/a2a-client/Cargo.toml:12,20`、`examples/a2a-server/Cargo.toml:10,13-14`、`examples/quick_research/Cargo.toml:14,19` 存在未使用依赖。建议 `cargo machete` 清理。

### E6. .gitignore 覆盖不全

- 缺 `/reports`（deep-research 运行时生成）、`/chroma_db`（ChromaPersistentStore 落盘）、`.ruff_cache`（根目录已出现）、`*.rs.bk`。

### E7. 分支混乱

- `master`/`main`/`dev` 三个本地分支并存且均有远端对应，当前 checkout `master`，发布与协作易错位。建议统一单一主干。

### E8. 同步 I/O 阻塞 async

- `rucora-tools/src/search/content_search.rs:66-80`、`glob_search.rs:61-67`、`media/image_info.rs:212,227-230` 在 `#[async_trait]` 的 call 中同步 `std::fs` 阻塞运行时线程；`memory/mod.rs:16-18` 用 `std::sync::Mutex` 在 async 中 `lock().unwrap()`。
- 每个 Web 工具调用都新建 `reqwest::Client`（http.rs:193-198、fetch.rs:93-98、browse.rs:77-81、search.rs:159,342），连接池无法复用。

### E9. 其他小项

- `ollama.rs:95-101` `from_env` 返回 `Self` 而其他 provider 返回 `Result<Self, ProviderError>`，API 形状不统一；
- `openai.rs:599-607` 函数内局部 `preview` 闭包遮蔽 crate 级同名导入；`openai.rs` 是唯一未用 `apply_sampling_params` 的 provider；
- `http_config.rs:27-34` 与 `47-57` 构建逻辑重复，`build_client` 可复用 `build_client_with_timeout`；
- `gemini.rs:306-308` `ToolCall.id` 直接复用 name，同名工具多轮调用无法区分；
- `openai.rs:757-769` 流式把 finish_reason 附着在每个文本 delta 上，上层若以"非空"作终止条件会提前结束丢尾字；
- `loop_detector.rs:47` 枚举文档（"半阈值 Block"）与实现（满阈值 Block）矛盾，模块文档（:13）又一致，三处不一致；
- `rucora-macros` 发布元数据缺 readme/keywords/categories/documentation；
- `rucora-skills/Cargo.toml:14-16` 空 feature（default=[]、all=[]）无意义；
- `OpenAiEmbeddingProvider::from_env`（`rucora-embed/src/openai.rs:31-32`）强制要求 `EMBEDDING_MODEL` 与模块文档"默认 text-embedding-ada-002"不符；
- `examples/skills_usage.rs` 游离在根 examples/（无 Cargo.toml），AGENTS.md 的 `cargo run --example skills_usage` 必失败；
- `rucora-a2a/src/lib.rs:82-94` 响应解析多层 `.or_else` 链脆弱，无 schema 校验，应强类型反序列化；
- `rucora-skills/src/tool_adapter.rs:57-62` 非对象 input_schema 包装成非法 JSON Schema；
- `in_memory.rs:94-127` 搜索全量 O(n) 扫描且 `SearchResult.vector` 恒 None；
- `CachedEmbeddingProvider`（`rucora-embed/src/cache.rs:63-82`）缓存检查与回填两次加锁，并发重复计算（良性但浪费）；
- `content_search.rs`/`image_info.rs` 等对文件操作未做并发限制与大小保护。

---

## 修复优先级建议

| 优先级 | 项 | 一句话修复 |
|---|---|---|
| P0 | H1 | Single 分支补 `messages.push(response.message)` |
| P0 | H3/H4/H5 | 补 Anthropic/Gemini/4 个兼容 provider 的流式工具调用解析 |
| P0 | H7 | 共享 SSE 解析器，`[DONE]` 后立即终止 |
| P0 | H8 | UTF-8 截断改 `floor_char_boundary`（5 处） |
| P1 | H6 | 错误映射函数提取到 helpers，8 个 provider 统一 |
| P1 | H10-S2/S4/S5 | CmdExec 禁 `-o`、Browse 加 SSRF 校验、ShellTool 环境变量清理 |
| P1 | H9 | 重写 `#[rucora_guard]` 宏 |
| P1 | H2 | StreamEngine 接入增强配置与中间件 |
| P1 | H11 | PermissionsConfig 执行链路 / CachedSkillLoader miss 回填 |
| P2 | H12 | core 潜伏 bug：interrupt TOCTOU、run_batch=0 panic、ShutdownHandle is_shutdown |
| P2 | M1/M3/M2 | ReAct/Reflect 循环修复、reflect off-by-one、SummaryAgent 流式一致性 |
| P2 | M7/M4 | 并发路径竞态、run_batch 任务取消 |
| P3 | M18/M5/M6 | 重复代码抽取、ConcurrencyConfig 接入或移除、enable_tool_logging 生效 |
| P3 | M10/M11/M12 | feature 门控、ChromaPersistent 异步化、A2A 空模块 |
| P4 | E1-E9 | CI、文档同步、工具链配置、依赖清理、gitignore、分支收敛 |
