# AGENTS.md

本文件为在此仓库中工作的 AI 编程代理提供指导。

## 构建/检查/测试命令

### 构建
```bash
# 检查所有包是否可编译（快速）
cargo check --all-targets

# 构建所有包
cargo build

# 启用所有特性构建
cargo build --all-features
```

### 测试
```bash
# 运行工作区中的所有测试
cargo test --workspace

# 运行特定包的测试
cargo test -p rucora-core
cargo test -p rucora

# 按名称模式运行单个测试
cargo test test_name_pattern

# 在特定包中运行单个测试
cargo test -p rucora-core test_name_pattern

# 运行测试并显示标准输出/标准错误
cargo test -- --nocapture

# 运行特定测试文件
cargo test --test contract_tool
cargo test --test contract_provider

# 运行契约测试（核心 trait 的行为验证）
cargo test -p rucora-core --test contract_provider
cargo test -p rucora-core --test contract_tool
cargo test -p rucora-core --test contract_vector_store

# 运行属性测试
cargo test -p rucora-core --test property_tests

# 运行集成测试
cargo test -p rucora-core --test integration_test
```

### 代码检查
```bash
# 使用工作区 lint 配置运行 clippy
cargo clippy --all-targets

# 自动修复 clippy 警告（如可能）
cargo clippy --fix --all-targets

# 启用所有特性运行 clippy
cargo clippy --all-targets --all-features
```

### 运行示例
```bash
# 运行特定示例
cargo run --example 01_hello_world
cargo run --example 03_chat_with_tools
cargo run --example 07_rag
cargo run --example 15_react_agent

# 启用所有特性运行示例
cargo run --example 12_mcp --all-features
```

## 代码风格指南

### 导入
- 分组导入：标准库、外部 crate、crate 内部模块（用空行分隔）
- 内部 crate 导入使用 `crate::`
- 每组内按字母顺序排序导入
- 异步 trait 方法使用 `use async_trait::async_trait;`

### 格式化
- 遵循标准 `rustfmt` 约定
- 4 空格缩进（不使用制表符）
- 最大行长度：100 字符
- 多行结构末尾使用逗号

### 命名约定
- **类型/Trait**：`PascalCase`（如 `LlmProvider`、`ChatRequest`、`ToolError`）
- **函数/方法**：`snake_case`（如 `stream_chat`、`from_env`）
- **常量**：`SCREAMING_SNAKE_CASE`
- **模块**：`snake_case`（文件/目录名）
- **缩写**：遵循 Rust 约定（如 `LLM`、`API`、`HTTP`、`URL`、`JSON`）

### 类型
- 跨异步边界共享所有权使用 `Arc<T>`
- trait 使用 `Send + Sync` bound 确保异步兼容
- 函数参数尽可能使用 `&str` 而非 `String`
- 异步流使用 `BoxStream<'static, T>`
- 工具输入/输出中的动态 JSON 使用 `serde_json::Value`

### 错误处理
- 使用 `rucora_core::error` 中的统一错误类型：
  - `ProviderError` - LLM 提供者错误（Network、Api、Authentication、RateLimit、Timeout、Model、Message）
  - `ToolError` - 工具执行错误
  - `MemoryError` - 记忆操作错误
  - `SkillError` - 技能执行错误
  - `ChannelError` - 通信渠道错误
  - `AgentError` - Agent 执行错误
  - `DiagnosticError` - 结构化诊断信息
- 错误描述应包含上下文信息
- 使用 `ErrorCategory` 枚举进行错误分类（Network、Api、Authentication、Authorization、RateLimit、Timeout、Model、Tool、Policy、Configuration、Other）
- 为可重试错误实现 `is_retriable()` 逻辑
- 使用 `ErrorClassifier` trait 进行高级错误分类和故障切换决策

### 文档
- **所有代码注释必须使用中文**（项目约定）
- 公共 trait 和类型需要使用 `///` 文档注释
- 模块级文档使用 `//!`
- 可能失败的函数文档需包含 `# Errors` 部分
- 使用文档链接：[`module::Type`][]

### 异步模式
- 异步 trait 方法使用 `#[async_trait]` 宏
- 所有 I/O 操作必须是异步的
- 使用 `tokio` 运行时执行异步操作
- 测试函数使用 `#[tokio::test]` 标记

### 构建器模式
- 复杂配置使用构建器模式：
  ```rust
  let agent = ToolAgent::builder()
      .provider(provider)
      .model("gpt-4o-mini")
      .system_prompt("你是有用的助手")
      .tool(ShellTool)
      .try_build()?;
  ```

### Trait 实现
- 适当时为 `Box<T>` 和 `Arc<T>` 实现 trait（参见 provider/trait.rs）
- trait 对象兼容性使用 `?Sized` bound

## 项目架构

### 工作区结构
- **`rucora-core/`** - 核心抽象（trait、类型、错误、重试策略、优雅关闭）。不包含实现。
- **`rucora/`** - 聚合 crate，提供"开箱即用"的实现。
- **`rucora-providers/`** - LLM 提供者实现
- **`rucora-tools/`** - 工具实现（basic、file、math、media、memory、search、system、web）
- **`rucora-mcp/`** - MCP 协议集成
- **`rucora-a2a/`** - A2A 协议集成
- **`rucora-skills/`** - 技能系统（YAML 命令模板）
- **`rucora-embed/`** - 嵌入提供者
- **`rucora-retrieval/`** - 向量存储实现
- **`rucora-prompt/`** - 提示词管理
- **`examples/`** - 示例应用（a2a-client、a2a-server、rucora-skills-example、rucora-deep-research 等）
- **`rucora/examples/`** - 20+ 示例文件（01_hello_world.rs 至 25_streaming_agent.rs）

### Agent 类型

| Agent 类型 | 职责 | 适用场景 |
|------------|------|----------|
| `SimpleAgent` | 简单问答 | 翻译、总结、一次性任务 |
| `ChatAgent` | 纯对话 | 客服、心理咨询、闲聊 |
| `ToolAgent` | 工具调用 | 执行具体任务（默认选择） |
| `ReActAgent` | 推理 + 行动 | 多步推理任务 |
| `ReflectAgent` | 反思迭代 | 代码生成、写作 |

**注意**：`DefaultAgent` 已重命名为 `ToolAgent`（自 v0.2.0），旧名称仍可用但标记为 deprecated。

### 核心设计原则
1. **接口与实现分离**：`rucora-core` 仅定义 trait/类型
2. **决策与执行分离**：Agent 负责"做什么"（think），Execution 负责"怎么做"（run）
3. **特性门控模块**：使用 Cargo 特性（`providers`、`tools`、`skills`、`mcp`、`a2a`、`embed`、`retrieval`）
4. **统一事件模型**：所有通信使用 `ChannelEvent`（Message、TokenDelta、ToolCall、ToolResult、Skill、Memory、Debug、Error、Raw）

### 核心 Trait（rucora-core）

| Trait | 职责 |
|-------|------|
| `LlmProvider` | LLM 提供者抽象（chat、stream_chat） |
| `Tool` | 工具抽象（name、description、input_schema、call） |
| `VectorStore` | 向量存储抽象（upsert、search、delete） |
| `Memory` | 记忆抽象（add、query） |
| `Skill` | 技能抽象（name、description、input_schema、run_value） |
| `Channel` | 通信渠道抽象（send、stream） |
| `RetryPolicy` | 重试策略抽象（should_retry） |
| `GracefulShutdown` | 优雅关闭抽象（shutdown） |
| `ErrorClassifier` | 错误分类抽象（classify） |
| `InjectionGuard` | 提示注入检测抽象 |

### Lint 配置（Cargo.toml）
设置为 `deny` 的关键 lint（违反将导致构建失败）：
- `unnecessary_wraps` - 不要无必要地返回 Result
- `result_unit_err` - Result<(), String> 应使用更具体的错误类型
- `clone_on_copy` - 不要克隆 Copy 类型
- `needless_borrow` - 不必要的借用
- `redundant_clone` - 冗余的 clone 调用
- `single_match` - 单分支 match 使用 if let
- `unused_async` - 异步函数必须使用 async
- `manual_strip` - 手动实现 strip_prefix/suffix
- `manual_let_else` - 使用 let-else 模式
- `needless_collect` - 避免不必要的 collect()

已禁用的 lint：
- `dead_code` - 允许未使用的代码（开发阶段保留，避免频繁删除可能后续使用的代码）

### 契约测试
位于 `rucora-core/tests/`：
- `contract_provider.rs` - 验证 LlmProvider 实现
- `contract_tool.rs` - 验证 Tool 实现
- `contract_vector_store.rs` - 验证 VectorStore 实现

### 环境变量
- `OPENAI_API_KEY` - OpenAI API 密钥
- `ANTHROPIC_API_KEY` - Anthropic API 密钥
- `GOOGLE_API_KEY` - Google Gemini API 密钥
- `OPENAI_BASE_URL` - 自定义 API 基础 URL（用于 Ollama 等）
- `MODEL_NAME` - 示例使用的模型名称

### 快速开始模式
```rust
use rucora::provider::OpenAiProvider;
use rucora::agent::ToolAgent;
use rucora::prelude::Agent;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let provider = OpenAiProvider::from_env()?;
    let agent = ToolAgent::builder()
        .provider(provider)
        .model("gpt-4o-mini")
        .system_prompt("你是有用的助手")
        .try_build()?;
    let output = agent.run("你好".into()).await?;
    println!("{}", output.text().unwrap_or("无回复"));
    Ok(())
}
```

### 许可证
MIT 许可证
