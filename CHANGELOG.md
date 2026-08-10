# 变更日志

本项目遵循 [语义化版本](https://semver.org/lang/zh-CN/) 规范。

---

## [0.6.0] - 2026-08-10

**新增（New Features）**
- ASR（自动语音识别）支持：`rucora-core` 新增 `asr` 模块与 `AsrProvider` trait（`AsrRequest` / `AsrResult` / `AsrSegment`）
- `rucora-providers` 新增 `OpenAiAsrProvider`：通过 multipart 上传音频到 `/v1/audio/transcriptions`，解析 `verbose_json` 分段（含时间戳与说话人），支持 `from_env()` 与超时/自定义客户端配置
- `rucora::prelude` 导出 `AsrProvider`、`AsrRequest`、`AsrResult`、`AsrSegment` 与 `OpenAiAsrProvider`
- `AsrRequest` 全面对齐 OpenAI Audio API 参数：`response_format`（json/text/srt/vtt/verbose_json）、`word_timestamps`、`audio_address`（URL 模式）、`prompt`、`temperature`，并提供 `AsrRequest::builder()` 便捷构建
- 新增示例 `30_asr`：展示本地语音识别服务的音频转写（支持说话人分离、词级时间戳、URL 音频）

**优化**
- 可选参数默认 `None` 时不发送，由服务端默认行为决定，保证 OpenAI API 兼容且可自定义
- 修复 `enable_speaker_diarization` 默认值问题：qwen3-asr 默认开启，现通过 `Option<bool>` 显式控制，`Some(false)` 能真正关闭
- ASR 文件上传改为流式传输（`Part::stream` + `tokio-util`），大音频文件不再整体读入内存
- 转写响应改用类型化 `serde` 反序列化（替代手工 `Value` 解析），字段缺失安全默认

---

## [0.5.1] - 2026-08-04

**新增（New Features）**
- `Agent::run_batch_stream`：批量执行并逐条产出 `(原始索引, Result)` 的流，顺序永不乱，无需自行反查回填
- `AgentBatchExt::run_batch_with_callback`：批量执行带回调（索引 + `BatchProgress` 进度快照 + 结果），置于扩展 trait 以保持 `&dyn Agent` 的 dyn 兼容
- `BatchProgress { total, completed, succeeded, failed }` 进度结构体，附 `percent()` 便捷方法
- `Agent::run_batch` 重构为委托给 `run_batch_stream` 实现，行为不变（向后兼容）

**修复**
- 批量翻译场景下重复文本错位问题（结果自带索引后无需按文本反查）

---

## [0.5.0] - 2026-08-03

**破坏性变更（Breaking Changes）**
- Agent 构建器 `provider` 改为必填参数：`XxxAgent::builder(provider)` / `XxxAgentBuilder::new(provider)`
- `build()` 不再返回 `Result`，删除 `try_build()`、`.provider()` setter 与 `Default for XxxAgentBuilder`
- `agent!` 宏返回 Agent（不再需要 `?`），`Extractor` 内部构建同步更新
- 更新全部 24 个示例/测试与文档注释

**代码审查修复（docs/CODE_REVIEW_OVERALL.md）**
- P0：流式工具调用解析与历史消息回归（H1/H3/H4/H5/H7/H8）
- P1：统一 8 个 Provider 错误映射（H6）、工具 SSRF 防护与响应体流式读取（H10）、重写 `#[rucora_guard]` 宏为同步双参数（H9）、技能权限检查与缓存回填（H11）、流式执行引擎透传中间件链与增强配置（H2）

**MCP**
- 升级 `rmcp` 依赖 1.8 → 3.0，适配破坏性变更

**修复**
- `ToolRegistry` doctest 命名空间分隔符（`::` → `__`）
- `DefaultExecution::run` doctest 中 `AgentInput::new` 缺少 `?`

## [0.4.0] - 2026-07-21

**Provider 增强**
- 所有 8 个 Provider 新增 `with_request_timeout` / `with_connect_timeout` / `with_client`
- 修复 `elapsed: 0ns` bug（`map_reqwest_error` 改用实际耗时）

**基础设施**
- 结构化路由：`RouteSelection` + `RuntimeKey`（`FromStr` 解析 + 序列化）
- `InterruptSignal` 可打断信号（`AtomicBool` + `Notify`）
- `ToolErrorKind` + `ToolError::kind()` 轻量级错误分类

## [0.3.0] - 2026-07-03

**破坏性变更（Breaking Changes）**
- Deprecate agent builder `build()`，改用 `try_build()` 处理配置错误
- `AgentInput` 构造函数改为返回 `Result`
- 统一 `thiserror` 至 v2；从库依赖中移除 `anyhow`

**修复**
- 修复 `SummaryAgent::run_stream` 返回空流问题
- `ReActAgent` 中 `unreachable!()` 改为日志回退
- 移除重复的 memory 后端（`memory.rs`）
- 修复 15 个 `rucora-mcp` doctest 及其它历史 doctest
- 所有示例改用 `try_build().unwrap()`

---

## [0.2.0] - 2026-06-25

**SummaryAgent 摘要生成 Agent**
- 新增 `SummaryAgent`，支持 4 种内置摘要模式（Concise、BulletPoints、Detailed、TlDr）及自定义模式
- 3 级可配置模板系统：`prompt_template`/`chunk_template`/`combine_template`
- 基于 `text-splitter` 的长文本 map-reduce 分块策略
- 新增示例 `22_summary_agent` 演示 4 种场景用法

**移除废弃 API**
- 移除 `DefaultAgent` 类型别名（改用 `ToolAgent`）
- 移除 `RuntimeObserver` trait（改用 `ChannelObserver`）
- 移除废弃的工具模块重导出（`git`、`echo`、`file_legacy`、`system_legacy`、`web_legacy`）

**Doctest 修复（82 个全部通过）**
- 修复所有 doctest 中的错误导入路径（`rucora_runtime` → `rucora`，`rucora::` → `rucora::agent::`）
- 修复 Agent doctest 中缺少 `Agent` trait 导入的问题
- 修复 `&str` 到 `AgentInput` 的转换（添加 `.into()`）
- 修复工具实例化（`ShellTool` → `ShellTool::new()`）
- 修复 `ReActAgent.tools()` 调用方式（改为链式 `.tool()` 调用）
- 修复 `Extractor` doctest 中的导入和字段访问
- 修复 `prompt.rs` 中的断言值（ASCII 逗号 → 实际输出）
- 将依赖可选 feature 的 doctest（RAG/embed/retrieval）标记为 `ignore`
- 将不完整示例的 doctest 标记为 `ignore`

**代码质量修复**
- 修复 `middleware.rs` 中 `cached_value` 未使用变量警告
- 修复 `skill/mod.rs` 中 `Skill` trait 未正确重新导出的问题
- 修复 `skill_trait.rs` 和 `types.rs` 中的 UTF-8 BOM 字符
- 修复 23 处 doc 链接警告（无效 intra-doc 链接、未闭合 HTML 标签）
- 修复 12 处格式参数警告（`uninlined_format_args`）在 `rucora-deep-research` 示例中
- 修复 `collapsible_if`、`map_unwrap_or`、`redundant_closure` 等 clippy 警告

**Agent 构建器**
- 为所有 Agent builder 方法添加 `#[must_use]` 属性

**工具注册表**
- 添加 `ToolRegistry::definitions()` 缓存机制
- 在所有变更方法中添加缓存失效逻辑

**MCP 工具适配**
- `McpToolAdapter` 重命名为 `McpTool`（与代码一致），更新所有文档引用
- 修复 `rucora-mcp` doc 中对真实传输类型的引用

**工具调用缓存**
- 修复 `ToolResultCache` 的 LRU 淘汰策略（添加 `last_accessed` 字段）

**凭据清洗**
- 增强 `scrub_credentials` 函数，支持裸密钥模式检测（sk-, ghp_, xoxb-, AIza, JWT 等）

**错误处理**
- 添加 `ProviderError` 的 `From` trait 实现
- 改进 `execution.rs` 中的错误处理，保留结构化 `ProviderError`

**中间件**
- 实现 `RateLimitMiddleware` 的实际限流逻辑（滑动窗口算法）
- 实现 `CacheMiddleware` 的实际缓存基础设施

**性能优化**
- 优化 `remove_orphaned_tool_messages` 从 O(n²) 到 O(n)

**契约测试**
- 新增 `contract_memory.rs` 测试文件
- 新增 `contract_skill.rs` 测试文件

**Lint 配置**
- 在 `Cargo.toml` 中禁用 `dead_code` lint（开发阶段保留未使用代码）
- 更新 `AGENTS.md` 中的 lint 配置文档

### 新增功能

**宏系统 (Macros)**
- 新增 `rucora-macros` crate，提供过程宏与声明式宏支持
  - `#[rucora_tool]`: 从异步函数自动生成 `Tool` trait 实现（含参数结构体、JSON Schema）
  - `agent!`: 声明式构建各类 Agent（ToolAgent, SimpleAgent, ChatAgent, ReActAgent, ReflectAgent）
  - `messages!`: 快速构建 `Vec<ChatMessage>`
  - `chat_request!`: 快速构建 `ChatRequest`
  - `tool_params!`: 快速构建工具参数 JSON Schema
- 新增示例 `26_macros.rs` 演示所有宏系统用法

**类型增强**
- `rucora-core::tool::types`:
  - 新增 `ToolRiskLevel` 枚举（`Safe`, `Caution`, `Dangerous`），用于标记工具风险等级
  - `ToolResult` 增强：支持结构化数据 (`data`) 和二进制数据 (`bytes`)
  - 新增 `ToolResult::success()`, `failure()`, `with_data()`, `with_bytes()` 辅助构造函数
  - `ToolResult` 序列化默认 `success` 为 `true`
- `rucora-core::tool::trait`:
  - `Tool` trait 新增 `risk_level()` 方法，默认返回 `ToolRiskLevel::Safe`

**测试覆盖**
- 新增 `rucora-core` 单元测试（风险等级、ToolResult 增强）
- 新增 `rucora` 单元测试（宏系统验证）
- 新增 `rucora/tests/macro_integration.rs` 集成测试（过程宏展开、参数验证、错误处理）

**文档**
- 新增 `docs/macros.md` 宏系统完整文档

**Deep Research 核心模块 (Breaking Change)**
- 新增 `rucora-core::research` 模块，提供深度研究核心抽象
  - `ResearchContext`: 研究上下文，贯穿整个研究流程
  - `ResearchPhase`: 研究阶段（初始化、搜索、精读、综合、完成）
  - `ResearchStrategy`: 研究策略枚举（快速、标准、Agentic、研究库、学术）
  - `ResearchReport`: 研究报告结构
  - `ResearchConfig`: 研究配置

**Deep Research Trait**
- `DeepResearchEngine` trait: 深度研究引擎接口
- `StrategyTrait` trait: 研究策略接口
- `ResearchLibrary` trait: 研究库存储接口
- `CitationHandler` trait: 引用处理接口

**rucora 深度研究实现**
- 新增 `rucora::deep_research` 模块
  - `DefaultResearchEngine`: 默认研究引擎实现
  - `StandardStrategy`: 标准多阶段策略
  - `FastStrategy`: 快速研究策略
  - `AgenticStrategy`: Agentic 自主研究策略
  - `InMemoryResearchLibrary`: 内存研究库
  - `FileResearchLibrary`: 文件系统研究库

**评分系统**
- `ResearchQualityScore`: 研究质量评分（信息质量、完整性、置信度、综合评分）
- `ResearchSuggestion`: 研究改进建议类型
- `ScoringConfig`: 评分配置（阈值设置）
- `ResearchQualityAssessor`: 质量评估器，支持自动判断和提供改进建议

**新增示例**
- `quick_research`: 快速研究示例（30秒-3分钟）
- `iterative_research`: 迭代深化研究示例
- `rucora-deep-research-agentic`: Agentic 自主研究示例
- `rucora-deep-research-library`: 研究库示例
- `rucora-deep-research-academic`: 学术研究示例
- `research_quality_assessment`: 研究质量评分示例

**ShutdownToken 增强**
- 新增 `subscribe()` 方法，支持订阅关闭信号广播

**RetryPolicy 增强**
- 新增 `should_retry_with_error()` 方法，允许根据错误信息决定是否重试
- `TransientFilter` 现在支持基于错误信息的条件重试过滤

### 依赖更新
- 新增 `uuid` crate 用于生成唯一 ID
- 新增 `chrono` crate 用于时间处理
- 新增 `text-splitter` 0.29 替代自定义分块逻辑（用于 SummaryAgent）
- 升级 `html2text` 适配 `from_read` 返回 `Result` 的 API 变更
- 升级 `ra2a` 从 0.9 到 0.10 以匹配 `rucora-a2a`

### 文档新增
- `docs/deep_research_v2_plan.md`: Deep Research 0.2 实施计划
- `docs/deep_research_v2_implementation.md`: 实现思路详解
- `docs/deep_research_v2_quickstart.md`: 快速开始指南

---

## [0.1.5] - 2026-05-11

### 新增功能

**ShutdownToken 增强**
- 新增 `subscribe()` 方法，支持订阅关闭信号广播

**RetryPolicy 增强**
- 新增 `should_retry_with_error()` 方法，允许根据错误信息决定是否重试
- `TransientFilter` 现在支持基于错误信息的条件重试过滤

### 新增示例

**Deep Research 示例**
- `quick_research`: 快速研究示例（30秒-3分钟获取带引用的答案）
- `iterative_research`: 迭代深化研究示例（多轮迭代逐步深化研究）

**文档新增**
- `docs/deep_research_v2_plan.md`: Deep Research 0.2 实施计划
- `docs/deep_research_v2_implementation.md`: 实现思路详解
- `docs/deep_research_v2_quickstart.md`: 快速开始指南

---

## [0.1.4] - 2026-05-11

### 新增功能

**graceful_shutdown 模块**
- 新增 `graceful_shutdown` 模块，提供优雅关闭机制
- `ShutdownHandle`: 关闭句柄，用于触发和控制关闭流程
- `ShutdownToken`: 关闭令牌，用于检查是否已收到关闭信号
- `GracefulShutdown` trait: 统一关闭接口

**retry 模块**
- 新增 `retry` 模块，提供重试策略抽象
- `RetryPolicy` trait: 通用重试逻辑抽象
- `ExponentialBackoff`: 指数退避策略（支持可选抖动）
- `FixedDelay`: 固定间隔策略
- `NoRetry`: 不重试策略
- `RetryPolicyExt` trait: 重试策略扩展方法

### 改进

**并发性能提升**
- `InMemoryVectorStore` 使用 `DashMap` 替代 `Arc<RwLock<HashMap>>`，提升并发读写性能

**类型一致性改进**
- `ErrorDiagnostic::kind` 字段类型从 `&'static str` 改为 `String`
- `ToolCategory::name()` 返回类型从 `String` 改为 `&'static str`，减少不必要的内存分配

**输入验证增强**
- `LlmParams`: `temperature` 和 `top_p` 添加范围检查
- `VectorRecord`: 向量非空检查
- `VectorQuery`: `top_k` 范围检查 (1-1000)
- `AgentInput`: 文本非空检查
- `MetricAggregator::percentile`: 参数范围检查 (0-100)

### Bug 修复

**ToolFilterConfig 行为修复**
- 修复 `is_always_visible()` 和 `is_dynamic()` 方法在禁用过滤器时错误返回可见性状态的问题
- 修复 `new()` 默认配置的正确行为

**文档修复**
- 修复 doctest 示例代码标记

---

## [0.1.3] - 2026-05-09

### 安全修复

**收紧工具执行边界**

- `ShellTool` 不再将参数拼接成 shell 字符串执行，改为直接通过 `Command::new(...).args(...)` 传参，降低参数注入风险
- `CmdExecTool` 修复命令白名单前缀误判，并同步使用更安全的命令执行路径
- 文件工具的 `allowed_dirs` 校验改为基于 canonical path，避免写入路径通过父目录关系绕过限制
- Web/HTTP 工具新增共享 URL 安全校验，拒绝 localhost、私网、链路本地地址，并在 DNS 解析后再次检查目标地址
- `WebFetchTool` 增加响应体大小限制，并禁用自动重定向，避免重定向绕过访问控制

### 行为修复

**修复可选 feature 与异步执行问题**

- 修复 `rucora --no-default-features` 下 `prelude` 无条件导出 provider 类型导致的编译失败
- 移除 Agent 执行路径中异步上下文里的同步 `block_on`，改为真正异步获取对话历史
- OpenAI 流式响应补充 tool call 增量累积解析，使 streaming 模式可以收到完整工具调用

### Provider 改进

**改进错误分类与重试语义**

- OpenAI/Ollama provider 将网络、超时、HTTP 状态码错误映射为结构化 `ProviderError`
- `ResilientProvider` 优先使用结构化错误的 `is_retriable()` 结果，再回退到字符串分类
- 修复 Ollama tool call arguments 解析中的不必要 `unwrap`

### 文档与测试

**修复 provider 文档示例**

- 修正 `rucora-providers` doctest 中错误引用聚合 crate 路径的问题
- 更新 provider 示例中的旧构造函数调用，保证 doctest 可编译

---

## [0.1.2] - 2026-04-29

### API 改进

**简化 Extractor 泛型用法**

- `Extractor<P, T>` 调整为对外只暴露目标类型 `T`
- 创建方式从 `Extractor::<_, T>::builder(...)` 简化为 `Extractor::<T>::builder(...)`
- 为 `Box<T>` / `Arc<T>` 补充 `LlmProvider` 转发实现，支持内部类型擦除

### 行为修复

**修复对话历史与 Agent 构建行为不一致的问题**

- 统一 `ChatAgent`、`ToolAgent`、`ReActAgent`、`ReflectAgent` 的 `with_conversation(true)` 语义，避免调用顺序影响最终行为
- 修复启用 `ConversationManager` 时 system prompt 可能重复注入的问题
- 为各类 Agent builder 增加 `try_build()`，提供显式错误返回入口

**补全 ConversationManager token 限制**

- 实现 `with_max_tokens(...)` 的实际裁剪逻辑
- 修复 `clear()`、`from_json()`、系统提示词注入后的 token 计数
- 新增对应回归测试，确保 token 限制真实生效

**统一工具构造错误边界**

- `SerpapiTool::with_keys(...)`
- `TavilyTool::with_keys(...)`
- `rucora_tools::web::search` 中同类 `with_keys(...)`

以上接口改为返回 `Result`，空 key 不再直接 panic

**修复 Extractor 重试错误语义**

- `ExtractionError::MaxRetriesExceeded` 现在会在配置重试且所有尝试失败时真实返回

### 文档更新

**同步文档到当前 API 行为**

- 重写自动对话历史文档，纠正过时的 builder 示例
- 补充发布与版本管理文档，说明 workspace 统一版本下的 crates.io 发布策略
- 修正文档中漏写 `model(...)` 或仍使用旧接口的示例

---

## [0.1.1] - 2026-04-29

### Extractor 修复

**修复 Extractor 无法正确提取结构化数据的问题**

- 修复 Ollama provider 解析 tool_calls 时 arguments 格式兼容问题 — Ollama 返回的 arguments 为 JSON 对象而非 JSON 字符串，现同时兼容两种格式
- 适配 schemars 1.2 API 变更 — `schema_for!` 返回的 Schema 直接实现 `Serialize`，移除对已废弃的 `.schema` 字段的访问

**修改文件**:
- `rucora-providers/src/ollama.rs`
- `rucora/src/agent/extractor.rs`

---

## [0.1.0] - 2026-04-28

### 概述

rucora 首个公开版本。一个高性能、类型安全的 LLM Agent 框架，支持多 Provider、多工具、多协议。

### 核心特性

- **多 Provider 支持** — OpenAI、Anthropic、Gemini、Ollama、DeepSeek、Moonshot、OpenRouter、Azure OpenAI
- **5 种 Agent 类型** — SimpleAgent、ChatAgent、ToolAgent、ReActAgent、ReflectAgent
- **统一 LLM 参数配置** — `LlmParams` 类型支持 temperature、top_p、max_tokens 等参数
- **Extractor 结构化数据提取** — 基于 tool calling 的 JSON 数据提取
- **20+ 内置工具** — 文件操作、Shell、HTTP、Web 爬取、搜索、数学计算等
- **技能系统** — YAML 定义的可复用技能模板
- **MCP / A2A 协议支持** — Model Context Protocol 和 Agent-to-Agent 协议
- **高级内存系统** — 命名空间隔离、重要性评分、GDPR 合规
- **上下文压缩** — 分层压缩引擎，支持 Aggressive/Balanced/Conservative 策略
- **循环检测** — 防止 Agent 陷入无限循环
- **错误分类器** — 14 种精细错误原因分类，自动判断重试/回退/压缩策略
- **Prompt 注入防护** — 8 种威胁类型检测

### 架构模块

| 模块 | 职责 |
|------|------|
| `rucora-core` | 核心抽象层（traits/types） |
| `rucora` | 主库（实现聚合） |
| `rucora-providers` | LLM Provider 实现 |
| `rucora-tools` | 工具实现 |
| `rucora-mcp` | MCP 协议支持 |
| `rucora-a2a` | A2A 协议支持 |
| `rucora-skills` | 技能系统 |
| `rucora-embed` | Embedding 支持 |
| `rucora-retrieval` | 向量存储 |

---

## 贡献

欢迎贡献代码、文档或反馈问题！

1. Fork 项目
2. 创建特性分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 开启 Pull Request

## 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。
