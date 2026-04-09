//! Conversation（对话历史）管理模块
//!
//! # 概述
//!
//! 本模块提供对话历史管理功能，支持：
//! - 自动添加消息到历史
//! - 窗口限制（保留最近 N 条消息）
//! - Token 限制（保留最近 N 个 token）
//! - 消息压缩（使用 LLM 生成摘要）
//! - 持久化存储
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use agentkit::conversation::ConversationManager;
//! use agentkit_core::provider::types::{ChatMessage, Role};
//!
//! let mut manager = ConversationManager::new()
//!     .with_max_messages(20)
//!     .with_system_prompt("你是一个有用的助手");
//!
//! // 添加用户消息
//! manager.add_user_message("你好".to_string());
//!
//! // 添加助手回复
//! manager.add_assistant_message("你好！有什么可以帮助你的？".to_string());
//!
//! // 获取历史消息（用于 API 调用）
//! let messages = manager.get_messages();
//!
//! // 检查是否需要压缩
//! if manager.should_compact("gpt-4o") {
//!     // 执行压缩
//!     // manager.compact(&provider, "gpt-4o").await?;
//! }
//!
//! // 清空历史
//! manager.clear();
//! ```

use agentkit_core::provider::LlmProvider;
use agentkit_core::provider::types::{ChatMessage, Role};
use serde::{Deserialize, Serialize};

// 导入压缩模块
use crate::compact::generate_compact_prompt;
use crate::compact::{CompactConfig, TokenCounter};
use crate::compact::{group_messages_by_api_round, groups_to_text, select_groups_to_compact};

/// 对话历史管理器
///
/// 负责管理对话消息的添加、检索和压缩。
///
/// # 示例
///
/// ```rust
/// use agentkit::conversation::ConversationManager;
///
/// let manager = ConversationManager::new()
///     .with_max_messages(20);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationManager {
    /// 系统提示词
    system_prompt: Option<String>,
    /// 消息历史
    messages: Vec<ChatMessage>,
    /// 最大消息数（0 表示无限制）
    max_messages: usize,
    /// 最大 token 数（0 表示无限制）
    max_tokens: usize,
    /// 是否自动压缩
    auto_compress: bool,

    // 压缩相关字段
    /// 压缩配置
    compact_config: CompactConfig,
    /// Token 计数器
    token_counter: TokenCounter,
    /// 当前 token 计数
    token_count: u32,
    /// 压缩边界索引
    compact_boundary: Option<usize>,
}

impl Default for ConversationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ConversationManager {
    /// 创建新的对话管理器
    pub fn new() -> Self {
        Self {
            system_prompt: None,
            messages: Vec::new(),
            max_messages: 0,
            max_tokens: 0,
            auto_compress: false,
            compact_config: CompactConfig::default(),
            token_counter: TokenCounter::new(),
            token_count: 0,
            compact_boundary: None,
        }
    }

    /// 设置系统提示词
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// 设置最大消息数
    pub fn with_max_messages(mut self, max: usize) -> Self {
        self.max_messages = max;
        self
    }

    /// 设置最大 token 数
    pub fn with_max_tokens(mut self, max: usize) -> Self {
        self.max_tokens = max;
        self
    }

    /// 启用自动压缩
    pub fn with_auto_compress(mut self, enable: bool) -> Self {
        self.auto_compress = enable;
        self
    }

    /// 设置压缩配置
    pub fn with_compact_config(mut self, config: CompactConfig) -> Self {
        self.compact_config = config;
        self
    }

    /// 启用自动压缩（便捷方法）
    pub fn with_auto_compact(mut self, enabled: bool) -> Self {
        self.compact_config.auto_compact_enabled = enabled;
        self
    }

    /// 设置压缩缓冲区 tokens
    pub fn with_compact_buffer_tokens(mut self, tokens: u32) -> Self {
        self.compact_config.auto_compact_buffer_tokens = tokens;
        self
    }

    /// 添加系统提示词（如果尚未设置）
    pub fn ensure_system_prompt(&mut self, prompt: impl Into<String>) {
        if self.system_prompt.is_none() {
            self.system_prompt = Some(prompt.into());
        }
    }

    /// 添加消息
    pub fn add_message(&mut self, message: ChatMessage) {
        // 估算 token 并更新计数
        let tokens = self.estimate_message_tokens(&message);
        self.token_count = self.token_count.saturating_add(tokens);

        // 如果是第一条消息且没有系统提示词，先添加系统提示词
        if self.messages.is_empty()
            && let Some(prompt) = &self.system_prompt {
                self.messages.push(ChatMessage {
                    role: Role::System,
                    content: prompt.clone(),
                    name: None,
                });
            }

        self.messages.push(message);
        self.enforce_limits();
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

    /// 添加用户消息
    pub fn add_user_message(&mut self, content: impl Into<String>) {
        self.add_message(ChatMessage {
            role: Role::User,
            content: content.into(),
            name: None,
        });
    }

    /// 添加助手消息
    pub fn add_assistant_message(&mut self, content: impl Into<String>) {
        self.add_message(ChatMessage {
            role: Role::Assistant,
            content: content.into(),
            name: None,
        });
    }

    /// 添加工具结果
    pub fn add_tool_result(&mut self, tool_call_id: impl Into<String>, content: impl Into<String>) {
        self.add_message(ChatMessage {
            role: Role::Tool,
            content: content.into(),
            name: Some(tool_call_id.into()),
        });
    }

    /// 获取所有消息
    pub fn get_messages(&self) -> &[ChatMessage] {
        &self.messages
    }

    /// 获取最近 N 条消息
    pub fn get_recent_messages(&self, limit: usize) -> &[ChatMessage] {
        if limit >= self.messages.len() {
            &self.messages
        } else {
            &self.messages[self.messages.len() - limit..]
        }
    }

    /// 获取消息数量
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// 清空历史
    pub fn clear(&mut self) {
        self.messages.clear();

        // 保留系统提示词
        if let Some(prompt) = &self.system_prompt {
            self.messages.push(ChatMessage {
                role: Role::System,
                content: prompt.clone(),
                name: None,
            });
        }
    }

    /// 执行限制检查
    fn enforce_limits(&mut self) {
        if self.max_messages > 0 && self.messages.len() > self.max_messages {
            // 保留系统提示词
            let has_system = self
                .messages
                .first()
                .map(|m| m.role == Role::System)
                .unwrap_or(false);

            let skip = if has_system { 1 } else { 0 };
            let _keep_count = self.max_messages - skip;

            if self.messages.len() > self.max_messages {
                let drain_count = self.messages.len() - self.max_messages;
                self.messages.drain(skip..skip + drain_count);
            }
        }

        // TODO: 实现 token 限制检查
        // 需要集成 token 计数器
    }

    /// 压缩历史（使用摘要）
    ///
    /// 将早期消息压缩为单个摘要消息。
    pub fn compress(&mut self, summary: impl Into<String>) {
        let has_system = self
            .messages
            .first()
            .map(|m| m.role == Role::System)
            .unwrap_or(false);

        let summary_message = ChatMessage {
            role: Role::System,
            content: format!("对话历史摘要：{}", summary.into()),
            name: None,
        };

        // 保留系统提示词和最近 2 条消息
        let mut new_messages = Vec::new();
        if has_system {
            new_messages.push(self.messages[0].clone());
        }
        new_messages.push(summary_message);

        // 保留最近 2 条消息
        if self.messages.len() > 2 {
            new_messages.extend_from_slice(&self.messages[self.messages.len() - 2..]);
        }

        self.messages = new_messages;
    }

    /// 导出为 JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.messages)
    }

    /// 从 JSON 导入
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let messages: Vec<ChatMessage> = serde_json::from_str(json)?;
        Ok(Self {
            messages,
            ..Default::default()
        })
    }

    // ==================== 压缩相关方法 ====================

    /// 获取当前 token 计数
    pub fn token_count(&self) -> u32 {
        self.token_count
    }

    /// 检查是否需要压缩
    pub fn should_compact(&self, model: &str) -> bool {
        let context_window = get_context_window_for_model(model);
        self.compact_config
            .should_compact(self.token_count, context_window)
    }

    /// 执行压缩
    pub async fn compact(
        &mut self,
        provider: &dyn LlmProvider,
        _model: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // 1. 分组消息
        let groups = group_messages_by_api_round(&self.messages);

        // 2. 选择要压缩的组（保留最近的 3 组）
        let groups_to_compact = select_groups_to_compact(&groups, 3);

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

        // 6. 重新计算 token 计数
        self.recalculate_token_count();

        Ok(summary)
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
            "{}\n\n{}",
            prompt, context_text
        ));

        let response = provider.chat(request).await?;
        Ok(response.message.content)
    }

    /// 创建压缩边界消息
    fn create_compact_boundary(&self, summary: String) -> ChatMessage {
        ChatMessage::system(format!(
            "<conversation_summary>\n{}\n</conversation_summary>\n\n\
             以上是之前对话的摘要。请基于此摘要继续对话。",
            summary
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

/// 简单的 token 计数器（估算）
///
/// 使用字符数估算 token 数（英文约 4 字符/token，中文约 1.5 字符/token）。
pub fn estimate_tokens(text: &str) -> usize {
    let chars = text.chars().count();
    // 简单估算：中英文混合平均 2.5 字符/token
    chars / 2 + 1
}

/// 计算消息列表的 token 数（估算）
pub fn estimate_messages_tokens(messages: &[ChatMessage]) -> usize {
    messages.iter().map(|m| estimate_tokens(&m.content)).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversation_manager_basic() {
        let mut manager = ConversationManager::new();

        manager.add_user_message("你好");
        manager.add_assistant_message("你好！有什么可以帮助你的？");

        assert_eq!(manager.len(), 2);
        assert!(!manager.is_empty());
    }

    #[test]
    fn test_conversation_manager_system_prompt() {
        let mut manager = ConversationManager::new().with_system_prompt("你是助手");

        manager.add_user_message("你好");

        assert_eq!(manager.len(), 2);
        assert_eq!(manager.messages[0].role, Role::System);
    }

    #[test]
    fn test_conversation_manager_max_messages() {
        let mut manager = ConversationManager::new()
            .with_system_prompt("系统")
            .with_max_messages(5);

        for i in 0..10 {
            manager.add_user_message(format!("消息 {}", i));
        }

        // 应该保留系统提示词 + 最近 4 条消息
        assert_eq!(manager.len(), 5);
        assert_eq!(manager.messages[0].role, Role::System);
    }

    #[test]
    fn test_conversation_manager_clear() {
        let mut manager = ConversationManager::new().with_system_prompt("系统");

        manager.add_user_message("你好");
        manager.clear();

        assert_eq!(manager.len(), 1);
        assert_eq!(manager.messages[0].content, "系统");
    }

    #[test]
    fn test_estimate_tokens() {
        // 英文测试
        assert!(estimate_tokens("Hello World") > 0);

        // 中文测试
        assert!(estimate_tokens("你好世界") > 0);
    }
}
