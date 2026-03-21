//! Conversation（对话历史）管理模块
//!
//! # 概述
//!
//! 本模块提供对话历史管理功能，支持：
//! - 自动添加消息到历史
//! - 窗口限制（保留最近 N 条消息）
//! - Token 限制（保留最近 N 个 token）
//! - 消息压缩（使用摘要）
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
//! // 清空历史
//! manager.clear();
//! ```

use agentkit_core::provider::types::{ChatMessage, Role};
use serde::{Deserialize, Serialize};

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

    /// 添加系统提示词（如果尚未设置）
    pub fn ensure_system_prompt(&mut self, prompt: impl Into<String>) {
        if self.system_prompt.is_none() {
            self.system_prompt = Some(prompt.into());
        }
    }

    /// 添加消息
    pub fn add_message(&mut self, message: ChatMessage) {
        // 如果是第一条消息且没有系统提示词，先添加系统提示词
        if self.messages.is_empty() {
            if let Some(prompt) = &self.system_prompt {
                self.messages.push(ChatMessage {
                    role: Role::System,
                    content: prompt.clone(),
                    name: None,
                });
            }
        }

        self.messages.push(message);
        self.enforce_limits();
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
            let has_system = self.messages.first()
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
        let has_system = self.messages.first()
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
    messages.iter()
        .map(|m| estimate_tokens(&m.content))
        .sum()
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
        let mut manager = ConversationManager::new()
            .with_system_prompt("你是助手");
        
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
        let mut manager = ConversationManager::new()
            .with_system_prompt("系统");
        
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
