//! Token 计数器模块
//!
//! 提供 token 计数和估算功能。

use serde::{Deserialize, Serialize};

/// Token 计数器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenCounter {
    /// 每个 token 的平均字符数（英文约 4，中文约 1.5）
    avg_chars_per_token: f32,
}

impl Default for TokenCounter {
    fn default() -> Self {
        Self {
            avg_chars_per_token: 4.0, // 保守估计
        }
    }
}

impl TokenCounter {
    /// 创建新的 Token 计数器
    pub fn new() -> Self {
        Self::default()
    }
    
    /// 设置平均每个 token 的字符数
    pub fn with_avg_chars_per_token(mut self, avg: f32) -> Self {
        self.avg_chars_per_token = avg;
        self
    }
    
    /// 快速估算文本的 token 数量
    /// 
    /// # 参数
    /// * `text` - 要估算的文本
    /// 
    /// # 返回值
    /// 估算的 token 数量
    pub fn estimate(&self, text: &str) -> u32 {
        let char_count = text.chars().count() as f32;
        (char_count / self.avg_chars_per_token) as u32
    }
    
    /// 估算消息的 token 数量
    /// 
    /// # 参数
    /// * `content` - 消息内容
    /// * `role` - 消息角色 (system/user/assistant)
    /// 
    /// # 返回值
    /// 估算的 token 数量（包含角色开销）
    pub fn estimate_message(&self, content: &str, role: &str) -> u32 {
        let base_tokens = self.estimate(content);
        
        // 角色开销估算
        let role_overhead = match role {
            "system" => 4,
            "user" => 3,
            "assistant" => 4,
            _ => 3,
        };
        
        base_tokens + role_overhead
    }
    
    /// 计算消息列表的 token 数量（精确计算需要调用 LLM 的 tokenizer）
    /// 
    /// # 参数
    /// * `messages` - 消息列表
    /// 
    /// # 返回值
    /// 总 token 数量
    pub fn count_messages(&self, messages: &[(String, String)]) -> u32 {
        messages
            .iter()
            .map(|(role, content)| self.estimate_message(content, role))
            .sum()
    }
    
    /// 从文件内容估算 token 数量
    pub fn estimate_file(&self, content: &str, file_type: &str) -> u32 {
        // 代码文件通常 token 密度更高
        let multiplier = match file_type {
            "rust" | "python" | "javascript" | "typescript" => 1.2,
            "markdown" | "text" => 1.0,
            "json" | "yaml" => 1.5,
            _ => 1.0,
        };
        
        (self.estimate(content) as f32 * multiplier) as u32
    }
}

/// 上下文窗口管理
pub struct ContextWindowManager {
    /// 模型上下文窗口大小
    context_window: u32,
    /// 当前 token 使用量
    current_tokens: u32,
    /// Token 计数器
    counter: TokenCounter,
}

impl ContextWindowManager {
    /// 创建新的上下文窗口管理器
    pub fn new(context_window: u32) -> Self {
        Self {
            context_window,
            current_tokens: 0,
            counter: TokenCounter::new(),
        }
    }
    
    /// 获取上下文窗口大小
    pub fn context_window(&self) -> u32 {
        self.context_window
    }
    
    /// 获取当前 token 使用量
    pub fn current_tokens(&self) -> u32 {
        self.current_tokens
    }
    
    /// 获取剩余 token 数量
    pub fn remaining_tokens(&self) -> u32 {
        self.context_window.saturating_sub(self.current_tokens)
    }
    
    /// 获取使用百分比
    pub fn usage_percent(&self) -> f32 {
        (self.current_tokens as f32 / self.context_window as f32) * 100.0
    }
    
    /// 添加消息的 token 计数
    pub fn add_message(&mut self, content: &str, role: &str) {
        let tokens = self.counter.estimate_message(content, role);
        self.current_tokens = self.current_tokens.saturating_add(tokens);
    }
    
    /// 移除消息的 token 计数
    pub fn remove_message(&mut self, content: &str, role: &str) {
        let tokens = self.counter.estimate_message(content, role);
        self.current_tokens = self.current_tokens.saturating_sub(tokens);
    }
    
    /// 检查是否接近限制
    pub fn is_near_limit(&self, buffer: u32) -> bool {
        self.remaining_tokens() <= buffer
    }
    
    /// 重置计数器
    pub fn reset(&mut self) {
        self.current_tokens = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_estimate() {
        let counter = TokenCounter::new();
        
        // 英文估算
        let english = "Hello, world! This is a test message.";
        let estimated = counter.estimate(english);
        assert!(estimated > 0);
        
        // 中文估算
        let chinese = "你好，世界！这是一个测试消息。";
        let estimated_cn = counter.estimate(chinese);
        assert!(estimated_cn > 0);
    }
    
    #[test]
    fn test_context_window_manager() {
        let mut manager = ContextWindowManager::new(200_000);
        
        assert_eq!(manager.context_window(), 200_000);
        assert_eq!(manager.current_tokens(), 0);
        assert_eq!(manager.remaining_tokens(), 200_000);
        
        manager.add_message("Hello, world!", "user");
        assert!(manager.current_tokens() > 0);
        
        manager.reset();
        assert_eq!(manager.current_tokens(), 0);
    }
    
    #[test]
    fn test_is_near_limit() {
        let mut manager = ContextWindowManager::new(200_000);
        manager.current_tokens = 190_000;
        
        assert!(manager.is_near_limit(20_000));
        assert!(!manager.is_near_limit(5_000));
    }
}
