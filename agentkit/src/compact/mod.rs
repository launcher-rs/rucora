//! 上下文压缩模块
//!
//! 提供上下文压缩功能，防止上下文超出模型限制。
//!
//! # 功能
//!
//! - **自动压缩** - 接近上下文限制时自动触发
//! - **响应式压缩** - API 拒绝时触发
//! - **手动压缩** - 用户主动触发
//! - **会话记忆压缩** - 压缩会话记忆
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use agentkit::compact::{CompactConfig, CompactStrategy, ContextManager};
//! use agentkit::provider::OpenAiProvider;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // 创建压缩配置
//! let config = CompactConfig::new()
//!     .with_auto_compact(true)
//!     .with_strategy(CompactStrategy::Auto);
//!
//! // 创建上下文管理器
//! let mut manager = ContextManager::new(config);
//!
//! // 检查是否需要压缩
//! if manager.should_compact("gpt-4o-mini") {
//!     // 执行压缩
//!     let provider = OpenAiProvider::from_env()?;
//!     manager.compact(&provider).await?;
//! }
//! # Ok(())
//! # }
//! ```

pub mod config;
pub mod grouping;
pub mod prompt;
pub mod token_counter;

pub use config::{CompactConfig, CompactStrategy};
pub use grouping::{group_messages_by_api_round, groups_to_text, select_groups_to_compact};
pub use prompt::{
    BASE_COMPACT_PROMPT, PARTIAL_COMPACT_PROMPT, generate_compact_prompt,
    generate_partial_compact_prompt,
};
pub use token_counter::{ContextWindowManager, TokenCounter};

use agentkit_core::provider::LlmProvider;
use agentkit_core::provider::types::{ChatMessage, Role};
use std::result::Result;

/// 上下文管理器
///
/// 管理对话消息和 token 使用量，支持自动压缩。
pub struct ContextManager {
    /// 对话消息
    messages: Vec<ChatMessage>,
    /// 当前 token 计数
    token_count: u32,
    /// 压缩配置
    config: CompactConfig,
    /// Token 计数器
    token_counter: TokenCounter,
    /// 压缩边界索引
    compact_boundary: Option<usize>,
}

impl ContextManager {
    /// 创建新的上下文管理器
    pub fn new(config: CompactConfig) -> Self {
        Self {
            messages: Vec::new(),
            token_count: 0,
            config,
            token_counter: TokenCounter::new(),
            compact_boundary: None,
        }
    }

    /// 添加消息
    pub fn add_message(&mut self, message: ChatMessage) {
        let tokens = self.estimate_message_tokens(&message);
        self.token_count = self.token_count.saturating_add(tokens);
        self.messages.push(message);
    }

    /// 获取消息列表
    pub fn messages(&self) -> &[ChatMessage] {
        &self.messages
    }

    /// 获取当前 token 计数
    pub fn token_count(&self) -> u32 {
        self.token_count
    }

    /// 估算消息的 token 数量
    fn estimate_message_tokens(&self, message: &ChatMessage) -> u32 {
        let role_str = match message.role {
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::System => "system",
            Role::Tool => "tool",
        };

        self.token_counter
            .estimate_message(&message.content, role_str)
    }

    /// 检查是否需要压缩
    pub fn should_compact(&self, model: &str) -> bool {
        let context_window = get_context_window_for_model(model);
        self.config.should_compact(self.token_count, context_window)
    }

    /// 执行压缩
    pub async fn compact(
        &mut self,
        provider: &dyn LlmProvider,
        _model: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // 1. 分组消息
        let groups = self.group_messages_by_api_round();

        // 2. 选择要压缩的组（保留最近的）
        let groups_to_compact = self.select_groups_to_compact(&groups);

        if groups_to_compact.is_empty() {
            return Ok(String::new());
        }

        // 3. 生成压缩摘要
        let summary: String = self
            .generate_compact_summary(provider, &groups_to_compact)
            .await?;

        // 4. 创建边界消息
        let boundary_message = self.create_compact_boundary(summary.clone());

        // 5. 替换已压缩的消息
        self.replace_compacted_messages(boundary_message, groups_to_compact.len());

        // 6. 更新 token 计数
        self.recalculate_token_count();

        Ok(summary)
    }

    /// 按 API 轮次分组消息
    fn group_messages_by_api_round(&self) -> Vec<Vec<ChatMessage>> {
        group_messages_by_api_round(&self.messages)
    }

    /// 选择要压缩的组
    fn select_groups_to_compact(&self, groups: &[Vec<ChatMessage>]) -> Vec<Vec<ChatMessage>> {
        // 保留最后 3 组
        select_groups_to_compact(groups, 3)
    }

    /// 生成压缩摘要
    async fn generate_compact_summary(
        &self,
        provider: &dyn LlmProvider,
        messages: &[Vec<ChatMessage>],
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let prompt = generate_compact_prompt(None);
        let context_text = groups_to_text(messages);

        let request = agentkit_core::provider::types::ChatRequest::from_user_text(format!(
            "{prompt}\n\n{context_text}"
        ));

        let response = provider.chat(request).await?;
        Ok(response.message.content)
    }

    /// 创建压缩边界消息
    fn create_compact_boundary(&self, summary: String) -> ChatMessage {
        ChatMessage::system(format!(
            "<conversation_summary>\n{summary}\n</conversation_summary>\n\n\
             以上是之前对话的摘要。请基于此摘要继续对话。"
        ))
    }

    /// 替换已压缩的消息
    fn replace_compacted_messages(&mut self, boundary_message: ChatMessage, groups_count: usize) {
        // 计算要移除的消息数量
        let messages_to_remove = groups_count * 2; // 每组通常包含 user + assistant

        // 移除旧消息
        if messages_to_remove < self.messages.len() {
            self.messages.drain(0..messages_to_remove);
            self.messages.insert(0, boundary_message);
            self.compact_boundary = Some(0);
        }
    }

    /// 重新计算 token 计数
    fn recalculate_token_count(&mut self) {
        self.token_count = self
            .messages
            .iter()
            .map(|m| self.estimate_message_tokens(m))
            .sum();
    }
}

/// 获取模型的上下文窗口大小
fn get_context_window_for_model(model: &str) -> u32 {
    // 常见模型的上下文窗口
    match model {
        // Claude 模型
        m if m.contains("claude-3-5-sonnet") => 200_000,
        m if m.contains("claude-3-opus") => 200_000,
        m if m.contains("claude-3-sonnet") => 200_000,
        m if m.contains("claude-3-haiku") => 200_000,

        // GPT 模型
        m if m.contains("gpt-4o") => 128_000,
        m if m.contains("gpt-4-turbo") => 128_000,
        m if m.contains("gpt-4") => 8_192,
        m if m.contains("gpt-3.5-turbo") => 16_385,

        // 其他模型（保守估计）
        _ => 32_000,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_manager_creation() {
        let config = CompactConfig::default();
        let manager = ContextManager::new(config);

        assert_eq!(manager.token_count(), 0);
        assert_eq!(manager.messages().len(), 0);
    }

    #[test]
    fn test_add_message() {
        let mut manager = ContextManager::new(CompactConfig::default());

        manager.add_message(ChatMessage::user("你好"));
        assert_eq!(manager.messages().len(), 1);
        assert!(manager.token_count() > 0);
    }

    #[test]
    fn test_should_compact() {
        // 使用较小的 buffer 来触发压缩
        // 使用 gpt-4（8192 上下文窗口）
        // buffer 设置为 1000，所以阈值是 7192 tokens
        let config = CompactConfig::default().with_buffer_tokens(1000);
        let mut manager = ContextManager::new(config);

        // 添加消息直到超过阈值
        // 每条消息约 50 个字符，约 12-13 tokens + 角色开销 3-4 = 约 16 tokens
        // 需要约 7192 / 16 = 450 条消息
        for i in 0..500 {
            manager.add_message(ChatMessage::user(&format!(
                "这是第 {} 条测试消息，包含一些额外的内容来增加 token 数量",
                i
            )));
            manager.add_message(ChatMessage::assistant(&format!(
                "这是第 {} 条回复，同样包含一些额外的内容来增加 token 数量",
                i
            )));
        }

        assert!(manager.should_compact("gpt-4"));
    }
}
