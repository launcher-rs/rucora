# AgentKit 上下文压缩系统集成 TODO

## 背景

根据 Claude Code 源码分析，上下文压缩是生产级 Agent 系统的核心功能。当对话历史接近模型上下文窗口限制时，需要自动压缩旧消息，保留关键信息，确保对话可以继续进行。

## 集成目标

在 AgentKit 中实现上下文压缩系统，支持：
1. **自动压缩** - 接近上下文限制时自动触发
2. **响应式压缩** - API 拒绝时触发
3. **手动压缩** - 用户主动触发
4. **会话记忆压缩** - 压缩会话记忆

## 需要继承/实现的方法

### 1. ConversationManager (对话管理器)

**文件**: `agentkit/src/conversation.rs` 或 `agentkit-core/src/conversation/mod.rs`

#### 需要添加的方法：

```rust
impl ConversationManager {
    /// 检查是否需要压缩
    /// 原因：在每次添加消息后检查 token 使用量，决定是否触发压缩
    pub fn should_compact(&self, model: &str, config: &CompactConfig) -> bool;
    
    /// 执行压缩
    /// 原因：核心压缩逻辑，调用 LLM 生成摘要并替换旧消息
    pub async fn compact(&mut self, provider: &dyn LlmProvider, config: &CompactConfig) -> Result<()>;
    
    /// 按 API 轮次分组消息
    /// 原因：Claude Code 使用 API 轮次而非用户轮次进行分组，更细粒度
    fn group_messages_by_api_round(&self) -> Vec<Vec<AgentMessage>>;
    
    /// 生成压缩摘要
    /// 原因：使用压缩提示词模板调用 LLM 生成结构化摘要
    async fn generate_compact_summary(&self, provider: &dyn LlmProvider, messages: &[Vec<AgentMessage>]) -> Result<String>;
    
    /// 创建压缩边界消息
    /// 原因：保存压缩摘要作为系统消息，标记压缩边界
    fn create_compact_boundary(&self, summary: String) -> AgentMessage;
    
    /// 替换已压缩的消息
    /// 原因：用边界消息替换旧消息，保留压缩后的上下文
    fn replace_compacted_messages(&mut self, boundary_message: AgentMessage);
    
    /// 后压缩清理
    /// 原因：恢复最近访问的关键文件，确保上下文连续性
    async fn run_post_compact_cleanup(&mut self) -> Result<()>;
    
    /// 获取当前 token 计数
    /// 原因：监控 token 使用量，触发压缩阈值检查
    pub fn token_count(&self) -> u32;
}
```

### 2. Agent / ToolAgent (Agent 执行层)

**文件**: `agentkit/src/agent/tool.rs` 或 `agentkit/src/agent/execution.rs`

#### 需要添加的方法：

```rust
impl<P> ToolAgent<P> {
    /// 在 run 循环中检查压缩
    /// 原因：每次 API 调用前后检查上下文状态，必要时触发压缩
    async fn check_and_compact_context(&self, context: &mut AgentContext) -> Result<()>;
}

impl DefaultExecution {
    /// 在执行循环中集成压缩检查
    /// 原因：统一执行层需要管理上下文压缩
    async fn run_loop_with_compact(&self, agent: &dyn Agent, input: AgentInput) -> Result<AgentOutput>;
}
```

### 3. Provider (LLM Provider)

**文件**: `agentkit/src/provider/mod.rs` 或 `agentkit-core/src/provider/trait.rs`

#### 需要添加的方法：

```rust
impl LlmProvider {
    /// 获取模型的上下文窗口大小
    /// 原因：不同模型有不同的上下文限制，需要动态获取
    fn get_context_window(&self, model: &str) -> u32;
    
    /// 估算消息的 token 数量
    /// 原因：快速估算 token 使用量，避免每次都调用完整的 tokenization
    fn estimate_tokens(&self, messages: &[ChatMessage]) -> u32;
}
```

### 4. Config (配置系统)

**文件**: `agentkit/src/config.rs` 或新建 `agentkit/src/compact/config.rs`

#### 需要添加的结构：

```rust
/// 压缩配置
#[derive(Debug, Clone)]
pub struct CompactConfig {
    /// 是否启用自动压缩
    pub auto_compact_enabled: bool,
    /// 自动压缩缓冲区 tokens
    pub auto_compact_buffer_tokens: u32,
    /// 警告缓冲区 tokens
    pub warning_buffer_tokens: u32,
    /// 错误缓冲区 tokens
    pub error_buffer_tokens: u32,
    /// 手动压缩缓冲区 tokens
    pub manual_compact_buffer_tokens: u32,
    /// 压缩策略
    pub strategy: CompactStrategy,
    /// 微压缩间隔（消息数量）
    pub micro_compact_interval: u32,
    /// 后压缩恢复文件数量
    pub post_compact_max_files_to_restore: usize,
    /// 后压缩 token 预算
    pub post_compact_token_budget: u32,
}

/// 压缩策略
#[derive(Debug, Clone)]
pub enum CompactStrategy {
    Auto,           // 自动压缩（接近限制时）
    Reactive,       // 响应式压缩（API 拒绝时）
    Manual,         // 手动压缩（用户触发）
    SessionMemory,  // 会话记忆压缩
}
```

### 5. Token Counter (Token 计数器)

**文件**: `agentkit/src/cost/mod.rs` 或新建 `agentkit/src/token_counter.rs`

#### 需要添加的方法：

```rust
pub struct TokenCounter {
    // ...
}

impl TokenCounter {
    /// 计算消息列表的 token 数量
    /// 原因：精确计算 token 使用量，触发压缩阈值
    pub fn count_messages(&self, messages: &[ChatMessage]) -> u32;
    
    /// 计算单个消息的 token 数量
    /// 原因：增量计算 token 使用量
    pub fn count_message(&self, message: &ChatMessage) -> u32;
    
    /// 快速估算 token 数量
    /// 原因：性能优化，避免每次都进行完整的 tokenization
    pub fn estimate(&self, text: &str) -> u32;
}
```

### 6. Builder 模式扩展

**文件**: `agentkit/src/agent/tool.rs` (ToolAgentBuilder)

#### 需要添加的方法：

```rust
impl<P> ToolAgentBuilder<P> {
    /// 设置压缩配置
    /// 原因：允许用户在创建 Agent 时配置压缩行为
    pub fn with_compact_config(mut self, config: CompactConfig) -> Self;
    
    /// 启用自动压缩
    /// 原因：便捷的 API 启用自动压缩
    pub fn with_auto_compact(mut self, enabled: bool) -> Self;
}
```

## 实现优先级

### Phase 1 (核心功能 - 1-2 周)

| 任务 | 文件 | 工作量 | 优先级 |
|------|------|--------|--------|
| 1.1 创建 CompactConfig 和 CompactStrategy | `agentkit/src/compact/config.rs` | 低 | ⭐⭐⭐ |
| 1.2 实现 TokenCounter | `agentkit/src/token_counter.rs` | 中 | ⭐⭐⭐ |
| 1.3 扩展 ConversationManager 添加压缩方法 | `agentkit/src/conversation.rs` | 高 | ⭐⭐⭐ |
| 1.4 实现消息分组逻辑 | `agentkit/src/conversation.rs` | 中 | ⭐⭐⭐ |
| 1.5 实现压缩提示词模板 | `agentkit/src/compact/prompt.rs` | 低 | ⭐⭐⭐ |

### Phase 2 (集成 - 1-2 周)

| 任务 | 文件 | 工作量 | 优先级 |
|------|------|--------|--------|
| 2.1 在 Agent 执行循环中集成压缩检查 | `agentkit/src/agent/execution.rs` | 高 | ⭐⭐⭐ |
| 2.2 实现后压缩清理逻辑 | `agentkit/src/compact/cleanup.rs` | 中 | ⭐⭐ |
| 2.3 添加 Builder 方法支持压缩配置 | `agentkit/src/agent/tool.rs` | 低 | ⭐⭐ |
| 2.4 编写单元测试 | `agentkit/tests/compact.rs` | 高 | ⭐⭐ |

### Phase 3 (优化 - 1 周)

| 任务 | 文件 | 工作量 | 优先级 |
|------|------|--------|--------|
| 3.1 实现会话记忆压缩 | `agentkit/src/compact/session_memory.rs` | 高 | ⭐ |
| 3.2 添加压缩遥测 | `agentkit/src/compact/telemetry.rs` | 中 | ⭐ |
| 3.3 性能优化（缓存 token 计数） | `agentkit/src/token_counter.rs` | 中 | ⭐ |
| 3.4 编写集成测试和示例 | `agentkit/examples/XX_context_compact.rs` | 中 | ⭐ |

## 依赖关系

```
CompactConfig (配置)
      ↓
TokenCounter (Token 计数)
      ↓
ConversationManager (对话管理)
      ↓
DefaultExecution (执行层)
      ↓
ToolAgent (Agent)
```

## 测试计划

### 单元测试

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_should_compact_threshold();
    
    #[test]
    fn test_group_messages_by_api_round();
    
    #[tokio::test]
    async fn test_generate_compact_summary();
    
    #[tokio::test]
    async fn test_compact_and_restore();
}
```

### 集成测试

```rust
#[cfg(test)]
mod integration_tests {
    #[tokio::test]
    async fn test_auto_compact_in_agent_loop();
    
    #[tokio::test]
    async fn test_reactive_compact_on_api_error();
    
    #[tokio::test]
    async fn test_post_compact_file_restoration();
}
```

## 示例代码

创建示例：`agentkit/examples/21_context_compact.rs`

```rust
//! 上下文压缩示例

use agentkit::agent::ToolAgent;
use agentkit::compact::{CompactConfig, CompactStrategy};
use agentkit::provider::OpenAiProvider;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let provider = OpenAiProvider::from_env()?;
    
    // 配置自动压缩
    let compact_config = CompactConfig {
        auto_compact_enabled: true,
        auto_compact_buffer_tokens: 13_000,
        warning_buffer_tokens: 20_000,
        strategy: CompactStrategy::Auto,
        ..Default::default()
    };
    
    let agent = ToolAgent::builder()
        .provider(provider)
        .with_compact_config(compact_config)
        .build();
    
    // 进行长对话，自动触发压缩
    // ...
    
    Ok(())
}
```

## 验收标准

- [ ] 所有 Phase 1 任务完成并通过测试
- [ ] 所有 Phase 2 任务完成并通过集成测试
- [ ] 所有 Phase 3 任务完成（可选）
- [ ] 示例代码可以正常运行
- [ ] 文档完整（包括 API 文档和使用指南）
- [ ] 性能测试通过（压缩不显著影响性能）

## 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 压缩丢失关键信息 | 高 | 实现后压缩清理，恢复关键文件 |
| 压缩性能开销 | 中 | 缓存 token 计数，异步压缩 |
| 压缩提示词不够准确 | 中 | 使用结构化模板，包含 9 个必要部分 |
| 与现有代码冲突 | 中 | 逐步集成，保持向后兼容 |

---

*创建日期：2026 年 4 月 1 日*  
*基于 Claude Code v2.1.88 源码分析*
