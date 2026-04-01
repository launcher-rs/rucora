# AgentKit 上下文压缩系统集成报告

## 完成日期
2026 年 4 月 1 日

## 概述

基于 Claude Code 源码分析，成功在 AgentKit 中实现了上下文压缩系统。该系统允许 Agent 在长对话中自动管理上下文，当接近模型上下文窗口限制时自动压缩旧消息，保留关键信息。

## 完成的模块

### 1. 配置模块 (`agentkit/src/compact/config.rs`)

**功能**:
- `CompactConfig` - 压缩配置结构
- `CompactStrategy` - 压缩策略枚举

**关键参数**:
```rust
pub struct CompactConfig {
    pub auto_compact_enabled: bool,              // 是否启用自动压缩
    pub auto_compact_buffer_tokens: u32,         // 自动压缩缓冲区 (默认 13K)
    pub warning_buffer_tokens: u32,              // 警告缓冲区 (默认 20K)
    pub error_buffer_tokens: u32,                // 错误缓冲区 (默认 20K)
    pub manual_compact_buffer_tokens: u32,       // 手动压缩缓冲区 (默认 3K)
    pub strategy: CompactStrategy,               // 压缩策略
    pub micro_compact_interval: u32,             // 微压缩间隔 (默认 10)
    pub post_compact_max_files_to_restore: usize,// 后压缩恢复文件数 (默认 5)
    pub post_compact_token_budget: u32,          // 后压缩 token 预算 (默认 50K)
}
```

**压缩策略**:
- `Auto` - 自动压缩（接近限制时触发）
- `Reactive` - 响应式压缩（API 拒绝时触发）
- `Manual` - 手动压缩（用户触发）
- `SessionMemory` - 会话记忆压缩

### 2. Token 计数器模块 (`agentkit/src/compact/token_counter.rs`)

**功能**:
- `TokenCounter` - Token 估算器
- `ContextWindowManager` - 上下文窗口管理器

**方法**:
```rust
impl TokenCounter {
    pub fn estimate(&self, text: &str) -> u32;
    pub fn estimate_message(&self, content: &str, role: &str) -> u32;
    pub fn count_messages(&self, messages: &[(String, String)]) -> u32;
    pub fn estimate_file(&self, content: &str, file_type: &str) -> u32;
}

impl ContextWindowManager {
    pub fn new(context_window: u32) -> Self;
    pub fn add_message(&mut self, content: &str, role: &str);
    pub fn is_near_limit(&self, buffer: u32) -> bool;
    pub fn usage_percent(&self) -> f32;
}
```

### 3. 消息分组模块 (`agentkit/src/compact/grouping.rs`)

**功能**:
- 按 API 轮次分组消息（而非用户轮次）
- 选择要压缩的组
- 消息组转文本

**关键函数**:
```rust
pub fn group_messages_by_api_round(messages: &[ChatMessage]) -> Vec<Vec<ChatMessage>>;
pub fn select_groups_to_compact(groups: &[Vec<ChatMessage>], preserve_count: usize) -> Vec<Vec<ChatMessage>>;
pub fn groups_to_text(groups: &[Vec<ChatMessage>]) -> String;
```

### 4. 压缩提示词模块 (`agentkit/src/compact/prompt.rs`)

**功能**:
- 基础压缩提示词模板（9 部分结构化模板）
- 部分压缩提示词（仅压缩最近消息）
- 压缩指令模板

**提示词结构**:
1. 主要请求和意图
2. 关键技术概念
3. 文件和代码段
4. 错误和修复
5. 问题解决
6. 所有用户消息
7. 待处理任务
8. 当前工作
9. 可选下一步

### 5. 上下文管理器模块 (`agentkit/src/compact/mod.rs`)

**功能**:
- `ContextManager` - 管理对话消息和 token 使用量

**核心方法**:
```rust
impl ContextManager {
    pub fn new(config: CompactConfig) -> Self;
    pub fn add_message(&mut self, message: ChatMessage);
    pub fn should_compact(&self, model: &str) -> bool;
    pub async fn compact(&mut self, provider: &dyn LlmProvider, model: &str) -> Result<String, ...>;
    pub fn token_count(&self) -> u32;
    pub fn messages(&self) -> &[ChatMessage];
}
```

### 6. 示例代码 (`agentkit/examples/21_context_compact.rs`)

**演示内容**:
1. 创建压缩配置
2. 创建上下文管理器
3. 添加消息并监控 token 使用
4. 检查是否需要压缩
5. 消息分组
6. 压缩提示词模板
7. Token 计数器
8. 上下文窗口管理

## 使用示例

### 基本使用

```rust
use agentkit::compact::{CompactConfig, CompactStrategy, ContextManager};
use agentkit::provider::OpenAiProvider;

// 创建压缩配置
let config = CompactConfig::new()
    .with_auto_compact(true)
    .with_strategy(CompactStrategy::Auto);

// 创建上下文管理器
let mut manager = ContextManager::new(config);

// 添加消息
manager.add_message(ChatMessage::user("你好"));
manager.add_message(ChatMessage::assistant("你好！有什么可以帮助你的吗？"));

// 检查是否需要压缩
if manager.should_compact("gpt-4o") {
    // 执行压缩
    let provider = OpenAiProvider::from_env()?;
    let summary = manager.compact(&provider, "gpt-4o").await?;
    println!("压缩摘要：{}", summary);
}
```

### Token 管理

```rust
use agentkit::compact::{TokenCounter, ContextWindowManager};

// Token 计数器
let counter = TokenCounter::new();
let tokens = counter.estimate("这是一段测试文本");

// 上下文窗口管理器
let mut window = ContextWindowManager::new(200_000); // Claude 200K
window.add_message("你好", "user");
window.add_message("你好！", "assistant");

println!("使用率：{:.2}%", window.usage_percent());
println!("是否接近限制：{}", window.is_near_limit(20_000));
```

## 压缩流程

```
1. 监控 token 使用量
   ↓
2. 达到阈值 → 触发压缩
   ↓
3. 按 API 轮次分组消息
   ↓
4. 选择要压缩的组（保留最近 3 轮）
   ↓
5. 调用 LLM 生成结构化摘要
   ↓
6. 创建压缩边界消息
   ↓
7. 替换已压缩的消息
   ↓
8. 重新计算 token 计数
   ↓
9. 继续对话
```

## 模型上下文窗口

系统内置了常见模型的上下文窗口：

| 模型 | 上下文窗口 |
|------|-----------|
| Claude 3.x | 200,000 tokens |
| GPT-4o | 128,000 tokens |
| GPT-4 Turbo | 128,000 tokens |
| GPT-4 | 8,192 tokens |
| GPT-3.5 Turbo | 16,385 tokens |
| 其他 | 32,000 tokens (默认) |

## 测试覆盖

### 单元测试

- `config.rs`: 配置创建、阈值计算、压缩触发测试
- `token_counter.rs`: Token 估算、上下文窗口管理测试
- `grouping.rs`: 消息分组、组选择测试
- `prompt.rs`: 提示词生成测试
- `mod.rs`: ContextManager 创建、消息添加、压缩触发测试

### 示例测试

运行示例：
```bash
cargo run --example 21_context_compact
```

## 性能优化

1. **快速估算**: 使用字符数估算 token，避免每次都调用完整 tokenizer
2. **增量计算**: 添加/移除消息时增量更新 token 计数
3. **按需压缩**: 仅在接近限制时触发压缩
4. **保留最近上下文**: 保留最近 3 轮对话，确保连续性

## 后续工作

### Phase 2 (集成到 Agent 执行循环)

- [ ] 在 `DefaultExecution::run_loop` 中集成压缩检查
- [ ] 实现后压缩清理逻辑（恢复关键文件）
- [ ] 添加 Builder 方法支持压缩配置

### Phase 3 (高级功能)

- [ ] 实现会话记忆压缩
- [ ] 添加压缩遥测
- [ ] 性能优化（缓存 token 计数）
- [ ] 编写集成测试

## API 兼容性

所有新增模块都使用独立的 `compact` 命名空间，不影响现有 API。

```rust
// 导入压缩模块
use agentkit::compact::{CompactConfig, ContextManager};

// 现有代码不受影响
use agentkit::agent::ToolAgent;
use agentkit::provider::OpenAiProvider;
```

## 文档

- TODO 文档：`docs/CONTEXT_COMPACT_TODO.md`
- 分析报告：`docs/claude_code_analysis_report.md` (已更新上下文压缩章节)
- 集成报告：`docs/CONTEXT_COMPACT_INTEGRATION.md` (本文档)

## 总结

成功实现了基于 Claude Code 源码分析的上下文压缩系统，包括：

✅ 配置系统（CompactConfig, CompactStrategy）
✅ Token 管理（TokenCounter, ContextWindowManager）
✅ 消息分组（按 API 轮次）
✅ 压缩提示词（9 部分结构化模板）
✅ 上下文管理器（ContextManager）
✅ 示例代码（21_context_compact.rs）
✅ 单元测试

该系统为 AgentKit 提供了生产级的上下文管理能力，使 Agent 能够处理超长对话而不丢失关键信息。

---

*集成完成日期：2026 年 4 月 1 日*  
*基于 Claude Code v2.1.88 源码分析*
