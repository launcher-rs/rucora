# ConversationManager vs ContextManager 分析

## 功能对比

| 功能 | ConversationManager | ContextManager (compact) | 说明 |
|------|---------------------|-------------------------|------|
| **消息管理** | ✅ | ✅ | 两者都管理 ChatMessage |
| **添加消息** | ✅ | ✅ | add_message() |
| **获取消息** | ✅ | ✅ | messages() / get_messages() |
| **系统提示词** | ✅ | ❌ | ConversationManager 支持 |
| **最大消息数** | ✅ | ❌ | ConversationManager 支持 |
| **最大 token 数** | ⚠️ (TODO) | ✅ | ContextManager 实现 |
| **Token 计数** | ❌ | ✅ | ContextManager 有 TokenCounter |
| **自动压缩** | ⚠️ (标记) | ✅ | ContextManager 完整实现 |
| **压缩触发** | ❌ | ✅ | should_compact() |
| **压缩执行** | ⚠️ (手动) | ✅ | compact() 调用 LLM |
| **消息分组** | ❌ | ✅ | group_messages_by_api_round() |
| **压缩提示词** | ❌ | ✅ | 9 部分结构化模板 |
| **后压缩清理** | ❌ | ✅ | 恢复关键文件 |

## 问题

### 1. 功能重叠
- 两者都管理 `Vec<ChatMessage>`
- 两者都有 `add_message()` 方法
- 两者都有压缩相关功能（一个标记，一个实现）

### 2. 使用场景混淆
- `ConversationManager` 用于 Agent 的 `with_conversation(true)`
- `ContextManager` 用于独立的上下文压缩

### 3. Token 计数缺失
- `ConversationManager` 有 `max_tokens` 字段但未实现
- `ContextManager` 有完整的 TokenCounter

## 集成方案

### 方案 A: 合并为一个模块（推荐）

**目标**: 统一为 `ConversationManager`，集成压缩功能

```rust
// agentkit/src/conversation.rs
use crate::compact::{CompactConfig, CompactStrategy, TokenCounter, group_messages_by_api_round};

pub struct ConversationManager {
    // 现有字段
    system_prompt: Option<String>,
    messages: Vec<ChatMessage>,
    max_messages: usize,
    max_tokens: usize,
    
    // 新增字段
    token_counter: TokenCounter,
    compact_config: CompactConfig,
    compact_boundary: Option<usize>,
}

impl ConversationManager {
    // 现有方法保持不变
    pub fn add_message(&mut self, message: ChatMessage);
    pub fn get_messages(&self) -> &[ChatMessage];
    
    // 新增压缩方法
    pub fn should_compact(&self, model: &str) -> bool;
    pub async fn compact(&mut self, provider: &dyn LlmProvider, model: &str) -> Result<String>;
    pub fn token_count(&self) -> u32;
}
```

**优点**:
- 统一 API，用户不需要学习两个管理器
- 向后兼容，现有代码不受影响
- 功能完整，既有消息管理又有压缩

**缺点**:
- 需要重构 `compact/mod.rs`
- 需要更新 `ContextManager` 的测试

### 方案 B: 明确分工

**目标**: 保持两个模块，但明确分工

```rust
// ConversationManager: 简单的消息管理
pub struct ConversationManager {
    messages: Vec<ChatMessage>,
    max_messages: usize,
    // 仅用于 Agent 的 with_conversation
}

// ContextManager: 完整的上下文压缩
pub struct ContextManager {
    messages: Vec<ChatMessage>,
    token_count: u32,
    compact_config: CompactConfig,
    // 用于独立的上下文压缩场景
}
```

**使用场景**:
- `ConversationManager`: Agent 对话历史（简单场景）
- `ContextManager`: 长对话压缩（复杂场景）

**优点**:
- 保持现有代码结构
- 清晰的职责分离

**缺点**:
- 用户需要学习两个 API
- 功能重叠仍然存在

### 方案 C: ContextManager 作为 ConversationManager 的扩展

**目标**: `ContextManager` 包装 `ConversationManager`

```rust
pub struct ContextManager {
    inner: ConversationManager,
    token_counter: TokenCounter,
    compact_config: CompactConfig,
}

impl ContextManager {
    pub fn new(conversation: ConversationManager, config: CompactConfig) -> Self;
    
    // 委托给 inner
    pub fn add_message(&mut self, message: ChatMessage) {
        self.inner.add_message(message);
        // 同时更新 token 计数
    }
    
    // 压缩功能
    pub async fn compact(&mut self, provider: &dyn LlmProvider, model: &str) -> Result<String>;
}
```

**优点**:
- 复用现有 `ConversationManager` 代码
- 渐进式升级路径

**缺点**:
- 包装层增加复杂度
- 性能开销

## 推荐方案

**推荐方案 A**: 合并为统一的 `ConversationManager`

**理由**:
1. **用户体验**: 单一 API，易于理解和使用
2. **向后兼容**: 现有方法保持不变
3. **功能完整**: 集成所有压缩功能
4. **代码复用**: 可以复用 `compact` 模块的工具函数

**实施步骤**:

1. Phase 1: 在 `ConversationManager` 中添加压缩方法
2. Phase 2: 更新 Agent 使用新的压缩功能
3. Phase 3: 标记 `ContextManager` 为 deprecated（可选）
4. Phase 4: 更新文档和示例

## 当前状态

- `ConversationManager`: 位于 `agentkit/src/conversation.rs`
- `ContextManager`: 位于 `agentkit/src/compact/mod.rs`
- 两者独立存在，功能有重叠

## 下一步

根据团队讨论决定采用哪个方案，然后实施重构。

---

*分析日期：2026 年 4 月 1 日*
