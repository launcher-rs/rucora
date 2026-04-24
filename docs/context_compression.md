# 上下文压缩指南

AgentKit 提供了多层上下文压缩系统，用于防止 LLM 对话中的上下文窗口溢出。

## 概述

上下文压缩模块位于 `agentkit::compact`，包含两个层次的实现：

1. **简单/传统层** - `CompactConfig`, `ContextManager`, `TokenCounter`
2. **高级分层引擎** - `LayeredCompressor`（受 Hermes Agent 启发设计）

## 核心类型

### CompactConfig（简单配置）

```rust
pub struct CompactConfig {
    pub auto_compact_enabled: bool,          // 启用自动压缩
    pub auto_compact_buffer_tokens: u32,     // 保留缓冲 token 数（默认：13,000）
    pub warning_buffer_tokens: u32,          // 警告阈值缓冲（默认：20,000）
    pub error_buffer_tokens: u32,            // 错误阈值缓冲（默认：20,000）
    pub manual_compact_buffer_tokens: u32,   // 手动压缩缓冲（默认：3,000）
    pub strategy: CompactStrategy,           // 压缩策略
    pub micro_compact_interval: u32,         // 微压缩间隔（默认：10 条消息）
    pub post_compact_max_files_to_restore: usize, // 压缩后恢复文件数（默认：5）
    pub post_compact_token_budget: u32,      // 压缩后 token 预算（默认：50,000）
}
```

**CompactStrategy 枚举：**

```rust
pub enum CompactStrategy {
    Auto,          // 接近限制时自动触发
    Reactive,      // API 拒绝请求时触发
    Manual,        // 用户手动触发
    SessionMemory, // 会话记忆压缩
}
```

### CompressionConfig（高级分层配置）

```rust
pub struct CompressionConfig {
    pub strategy: CompressionStrategy,       // 激进/平衡/保守
    pub protect_head_count: usize,           // 保护的头部消息数（默认：3）
    pub protect_tail_tokens: usize,          // 保护的尾部 token 数（默认：20,000）
    pub compression_threshold: f64,          // 触发阈值比率（默认：0.85）
    pub target_usage_ratio: f64,             // 压缩后目标使用率（默认：0.60）
    pub max_iterations: usize,               // 最大压缩迭代次数（默认：3）
    pub summary_cooldown_seconds: u64,       // 压缩间隔冷却时间（默认：600）
}
```

**CompressionStrategy 枚举：**

```rust
pub enum CompressionStrategy {
    Aggressive,   // 最大压缩，适合长对话
    Balanced,     // 默认，平衡方法
    Conservative, // 最小压缩，适合短对话
}
```

**预构建配置：**

| 配置 | head 数量 | tail token | 阈值 | 目标使用率 | 适用场景 |
|------|----------|-----------|------|-----------|---------|
| `aggressive()` | 2 | 15,000 | 80% | 50% | 长对话 (>200 轮) |
| `default()` / `Balanced` | 3 | 20,000 | 85% | 60% | 中等对话 (50-200 轮) |
| `conservative()` | 5 | 25,000 | 90% | 70% | 短对话 (<50 轮) |

### LayeredCompressor

高级压缩引擎，实现 5 步算法：

```rust
pub struct LayeredCompressor {
    config: CompressionConfig,
    last_summary_timestamp: Option<u64>,   // 防止频繁压缩
    last_summary_content: Option<String>,   // 支持迭代更新摘要
}
```

### TokenCounter

```rust
pub struct TokenCounter {
    avg_chars_per_token: f32,  // ~4.0 用于英语，~1.5 用于中文
}
```

### ContextWindowManager

```rust
pub struct ContextWindowManager {
    context_window: u32,   // 模型的上下文窗口大小
    current_tokens: u32,   // 当前 token 使用
    counter: TokenCounter, // Token 计数器实例
}
```

### ContextManager（简单管理器）

```rust
pub struct ContextManager {
    messages: Vec<ChatMessage>,
    token_count: u32,
    config: CompactConfig,
    token_counter: TokenCounter,
    compact_boundary: Option<usize>,
}
```

### ConversationManager（完整管理器）

结合消息管理和压缩的全面管理器：

```rust
pub struct ConversationManager {
    system_prompt: Option<String>,
    messages: Vec<ChatMessage>,
    max_messages: usize,
    max_tokens: usize,
    auto_compress: bool,
    compact_config: CompactConfig,
    token_counter: TokenCounter,
    token_count: u32,
    compact_boundary: Option<usize>,
}
```

## 主要方法

### CompactConfig 方法

| 方法 | 签名 | 用途 |
|------|------|------|
| `new()` | `fn new() -> Self` | 创建默认配置 |
| `with_auto_compact()` | `fn with_auto_compact(self, enabled: bool) -> Self` | 启用/禁用自动压缩 |
| `with_strategy()` | `fn with_strategy(self, strategy: CompactStrategy) -> Self` | 设置压缩策略 |
| `with_buffer_tokens()` | `fn with_buffer_tokens(self, tokens: u32) -> Self` | 设置缓冲 token |
| `should_compact()` | `fn should_compact(&self, current: u32, window: u32) -> bool` | 检查是否需要压缩 |
| `is_at_warning_level()` | `fn is_at_warning_level(&self, current: u32, window: u32) -> bool` | 检查警告阈值 |
| `is_at_error_level()` | `fn is_at_error_level(&self, current: u32, window: u32) -> bool` | 检查错误阈值 |
| `get_auto_compact_threshold()` | `fn get_auto_compact_threshold(&self, window: u32) -> u32` | 计算触发阈值 |

### LayeredCompressor 方法

| 方法 | 签名 | 用途 |
|------|------|------|
| `new()` | `fn new(config: CompressionConfig) -> Self` | 自定义配置创建 |
| `default_engine()` | `fn default_engine() -> Self` | 默认配置创建 |
| `should_compress()` | `fn should_compress(&self, current: usize, window: usize) -> bool` | 检查是否需要压缩 |
| `compress()` | `async fn compress(&mut self, provider: &dyn LlmProvider, messages: Vec<ChatMessage>, window: usize) -> Result<Vec<ChatMessage>, Box<dyn Error + Send + Sync>>` | 执行分层压缩 |
| `last_summary()` | `fn last_summary(&self) -> Option<&String>` | 获取上次摘要内容 |

### TokenCounter 方法

| 方法 | 签名 | 用途 |
|------|------|------|
| `new()` | `fn new() -> Self` | 创建（默认 4 字符/token） |
| `with_avg_chars_per_token()` | `fn with_avg_chars_per_token(self, avg: f32) -> Self` | 设置字符/token 比率 |
| `estimate()` | `fn estimate(&self, text: &str) -> u32` | 估算文本 token |
| `estimate_message()` | `fn estimate_message(&self, content: &str, role: &str) -> u32` | 估算消息 token（含角色开销） |
| `count_messages()` | `fn count_messages(&self, messages: &[(String, String)]) -> u32` | 计算消息列表 token |
| `estimate_file()` | `fn estimate_file(&self, content: &str, file_type: &str) -> u32` | 估算文件 token（含类型乘数） |

### ConversationManager 方法

| 方法 | 签名 | 用途 |
|------|------|------|
| `new()` | `fn new() -> Self` | 创建管理器 |
| `with_system_prompt()` | `fn with_system_prompt(self, prompt: impl Into<String>) -> Self` | 设置系统提示 |
| `with_max_messages()` | `fn with_max_messages(self, max: usize) -> Self` | 设置最大消息数 |
| `with_max_tokens()` | `fn with_max_tokens(self, max: usize) -> Self` | 设置最大 token 数 |
| `with_compact_config()` | `fn with_compact_config(self, config: CompactConfig) -> Self` | 设置压缩配置 |
| `add_user_message()` | `fn add_user_message(&mut self, content: impl Into<String>)` | 添加用户消息 |
| `add_assistant_message()` | `fn add_assistant_message(&mut self, content: impl Into<String>)` | 添加助手消息 |
| `add_tool_result()` | `fn add_tool_result(&mut self, tool_call_id, content) -> Self` | 添加工具结果 |
| `get_messages()` | `fn get_messages(&self) -> &[ChatMessage]` | 获取所有消息 |
| `get_recent_messages()` | `fn get_recent_messages(&self, limit: usize) -> &[ChatMessage]` | 获取最近 N 条消息 |
| `token_count()` | `fn token_count(&self) -> u32` | 获取当前 token 数 |
| `should_compact()` | `fn should_compact(&self, model: &str) -> bool` | 检查是否需要压缩 |
| `compact()` | `async fn compact(&mut self, provider, model: &str) -> Result<String, Box<dyn Error + Send + Sync>>` | 执行压缩 |
| `compress()` | `fn compress(&mut self, summary: impl Into<String>)` | 应用预生成的摘要 |
| `clear()` | `fn clear(&mut self)` | 清除历史（保留系统提示） |
| `to_json()` / `from_json()` | 序列化方法 | 持久化/加载对话 |

## LayeredCompressor 工作原理（5 步算法）

### 步骤 1：清理旧工具结果（廉价预压缩）

删除早期对话轮次的旧工具调用结果：

| 策略 | 保留最新工具结果数 |
|------|-------------------|
| Aggressive | 2 |
| Balanced | 4 |
| Conservative | 6 |

### 步骤 2：将消息拆分为头部/中间/尾部

- **头部**：前 N 条消息（系统提示 + 初始交互），从不压缩
- **中间**：头部和尾部之间的所有内容，由 LLM 生成摘要
- **尾部**：最近的消息，保留到 `protect_tail_tokens`，原文保留

### 步骤 3：生成结构化摘要

使用 10 部分结构化模板摘要中间消息：

1. **Goal** - 用户试图完成的目标
2. **Constraints & Preferences** - 用户偏好和编码风格
3. **Progress** - 已完成 / 进行中 / 受阻
4. **Key Decisions** - 重要的技术决策和理由
5. **Resolved Questions** - 已回答的问题
6. **Pending User Asks** - 用户未回答的问题
7. **Relevant Files** - 读取/修改/创建的文件
8. **Remaining Work** - 仍需完成的工作
9. **Critical Context** - 不能丢失的具体值
10. **Tools & Patterns** - 使用的工具和有效模式

### 步骤 4：创建摘要消息

将摘要包装在 XML 标签中：

```
<conversation-summary>
[结构化摘要内容]
</conversation-summary>

This is a structured summary of the previous conversation. Please continue working based on this summary and subsequent messages.
```

### 步骤 5：重组消息

最终输出：`[头部消息, 摘要消息, 尾部消息]`

**迭代更新**：在后续压缩时，如果 `last_summary_content` 存在，LLM 会同时获得上次摘要和新的对话内容，并被要求更新摘要而不是创建新的。

**冷却机制**：每次压缩后记录时间戳。在 `summary_cooldown_seconds`（默认 600 秒 / 10 分钟）内的后续 `should_compress()` 调用返回 false，防止快速重压缩。

## 压缩触发机制

有三种触发机制：

### 1. 自动触发（`CompactStrategy::Auto`）

当 `current_tokens >= context_window - buffer_tokens` 时触发。

使用默认缓冲 13,000 token 的 200K Claude 模型：在 ~187K token（93.5% 使用）时触发。

### 2. 响应触发（`CompactStrategy::Reactive`）

当 LLM API 返回上下文溢出错误时触发。

错误分类器检测模式如 "context length exceeded"、"token limit exceeded" 等。

### 3. 手动触发（`CompactStrategy::Manual`）

用户显式调用 `compact()` 或 `compress()`。

对于 `LayeredCompressor`，触发基于使用率比率：
`current_tokens / context_window >= compression_threshold`（默认 0.85）。

## 使用示例

### 示例 1：基本 CompactConfig 使用

```rust
use agentkit::compact::{CompactConfig, CompactStrategy};
use agentkit::conversation::ConversationManager;

// 创建压缩配置
let config = CompactConfig::new()
    .with_auto_compact(true)
    .with_strategy(CompactStrategy::Auto)
    .with_buffer_tokens(50_000);

// 创建带压缩的对话管理器
let mut manager = ConversationManager::new()
    .with_system_prompt("You are a friendly assistant")
    .with_max_messages(100)
    .with_compact_config(config);

// 添加消息（自动跟踪 token 数）
manager.add_user_message("Hello, I want to learn Rust");
manager.add_assistant_message("Hello! Rust is a systems programming language...");

// 检查是否需要压缩
if manager.should_compact("gpt-4o") {
    println!("需要压缩！Token 数：{}", manager.token_count());
}
```

### 示例 2：执行压缩

```rust
use agentkit::provider::OpenAiProvider;

let provider = OpenAiProvider::from_env()?;
let model_name = "gpt-4o-mini";

if manager.should_compact(&model_name) {
    match manager.compact(&provider, &model_name).await {
        Ok(summary) => {
            println!("压缩成功！");
            println!("摘要长度：{} 字符", summary.len());
            println!("压缩后 Token 数：{}", manager.token_count());
        }
        Err(e) => {
            println!("压缩失败：{}", e);
        }
    }
}
```

### 示例 3：LayeredCompressor 使用

```rust
use agentkit::{CompressionConfig, LayeredCompressor, CompressionStrategy};

// 使用特定策略创建压缩器
let config = CompressionConfig::aggressive();
let mut compressor = LayeredCompressor::new(config);

// 或使用默认
let engine = LayeredCompressor::default_engine();

// 检查是否需要压缩
let should = engine.should_compress(110_000, 128_000); // true (85.9% > 85%)

// 执行压缩（需要 LLM 提供者）
// let compressed = compressor.compress(&provider, messages, 128_000).await?;
```

### 示例 4：TokenCounter 使用

```rust
use agentkit::compact::TokenCounter;

let counter = TokenCounter::new();

// 简单文本估算
let tokens = counter.estimate("Hello, world! This is a test.");

// 消息估算（包含角色开销）
let msg_tokens = counter.estimate_message("fn main() {}", "assistant");
// 返回：base_tokens + 4（助手角色开销）

// 对于中文，调整比率
let cn_counter = TokenCounter::new().with_avg_chars_per_token(1.5);

// 文件估算（代码文件获得 1.2 倍乘数）
let file_tokens = counter.estimate_file("fn main() { println!(\"hi\"); }", "rust");
```

### 示例 5：ContextWindowManager 使用

```rust
use agentkit::compact::ContextWindowManager;

let mut window = ContextWindowManager::new(200_000); // Claude 200K

window.add_message("Hello, world!", "user");
println!("使用率：{:.1}%", window.usage_percent());

if window.is_near_limit(20_000) {
    println!("接近限制！剩余：{}", window.remaining_tokens());
}

window.reset();
```

### 示例 6：消息分组用于压缩

```rust
use agentkit::compact::{group_messages_by_api_round, select_groups_to_compact, groups_to_text};

let messages = vec![
    ChatMessage::user("Query 1"),
    ChatMessage::assistant("Response 1"),
    ChatMessage::user("Query 2"),
    ChatMessage::assistant("Response 2"),
    ChatMessage::user("Query 3"),
    ChatMessage::assistant("Response 3"),
    ChatMessage::user("Query 4"),
    ChatMessage::assistant("Response 4"),
];

// 按 API 轮次分组
let groups = group_messages_by_api_round(&messages);
// 结果：4 组 [user, assistant] 对

// 选择要压缩的组（保留最后 2 组）
let to_compact = select_groups_to_compact(&groups, 2);
// 结果：前 2 组被选中用于压缩

// 转换为文本用于 LLM 摘要
let text = groups_to_text(&to_compact);
// 结果：格式化的文本，带有 "=== Round 1 ===" 标题
```

## 与 Agent 集成

Agent 通过 `with_conversation(true)` 标志集成 `ConversationManager`：

```rust
let agent = ToolAgent::builder()
    .provider(provider)
    .model("gpt-4o-mini")
    .with_conversation(true)  // 启用内部 ConversationManager
    .system_prompt("You are a helpful assistant")
    .build();
```

`with_conversation(true)` 标志创建 `ConversationManager` 并包装在 `Arc<Mutex<>>` 中，以在线程安全的代理执行循环中共享。

**集成点：**
- `ChatAgent` - `agentkit/src/agent/chat.rs`
- `ReActAgent` - `agentkit/src/agent/react.rs`
- `ToolAgent` - `agentkit/src/agent/tool.rs`
- `ReflectAgent` - `agentkit/src/agent/reflect.rs`
- `AgentExecutor` - `agentkit/src/agent/execution.rs`

## 模型上下文窗口查找

```rust
fn get_context_window_for_model(model: &str) -> u32 {
    // Claude 模型：200,000
    // GPT-4o / GPT-4-Turbo：128,000
    // GPT-4：8,192
    // GPT-3.5-Turbo：16,385
    // 默认：32,000
}
```

## 策略选择指南

### 按对话长度

| 对话长度 | 推荐策略 | head 保护 | tail 保护 |
|----------|---------|----------|----------|
| < 50 轮 | Conservative | 5 条消息 | 25,000 token |
| 50-200 轮 | Balanced（默认） | 3 条消息 | 20,000 token |
| > 200 轮 | Aggressive | 2 条消息 | 15,000 token |

### 按模型设置缓冲

| 模型 | 上下文窗口 | 推荐缓冲 | 触发点 |
|------|-----------|---------|--------|
| GPT-4o / GPT-4-Turbo | 128K | 20,000 | 108K |
| Claude 3/3.5 | 200K | 30,000 | 170K |
| 本地模型 | 8K | 1,000 | 7K |
| 本地模型 | 32K | 2,000 | 30K |

## 最佳实践

1. **按对话长度选择策略** - 短对话用 Conservative，中等用 Balanced，长对话用 Aggressive。

2. **为每个模型设置适当的缓冲** - 不同模型有不同的上下文窗口，相应调整 `buffer_tokens`。

3. **生产环境使用分层压缩器** - `LayeredCompressor` 是更复杂的实现，具有头/尾保护、结构化摘要和迭代更新。`ContextManager` 是更轻量的替代方案。

4. **监控压缩效果** - 记录压缩前后的 token 数，跟踪压缩率，评估信息保留质量。

5. **在 Agent 中启用对话管理** - 使用 `.with_conversation(true)` 自动获得集成到代理执行循环的 `ConversationManager`。

6. **冷却机制防止抖动** - 默认 600 秒冷却时间确保压缩不会过于频繁，这可能破坏对话上下文。

7. **Token 估算是启发式的** - `TokenCounter` 使用基于字符的估算（英语 4 字符/token，中文 1.5 字符/token）。生产环境需要更精确的集成实际 LLM tokenizer。

8. **监控日志** - 启用 `tracing` 日志以查看压缩触发和效果：
   ```rust
   tracing::info!(tokens = token_count, "触发上下文压缩");
   ```

## 相关文件

- `agentkit/src/compact/config.rs` - CompactConfig 定义
- `agentkit/src/compact/engine.rs` - LayeredCompressor 实现
- `agentkit/src/compact/token_counter.rs` - TokenCounter 和 ContextWindowManager
- `agentkit/src/compact/mod.rs` - ContextManager 和模块导出
- `agentkit/src/compact/prompt.rs` - 压缩提示模板
- `agentkit/src/conversation.rs` - ConversationManager
- `agentkit/examples/21_unified_conversation.rs` - 对话管理示例
- `agentkit/examples/24_context_compression.rs` - 上下文压缩示例
