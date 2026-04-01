//! 上下文压缩配置模块
//!
//! 提供上下文压缩的配置和策略定义。

use serde::{Deserialize, Serialize};

/// 压缩策略
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum CompactStrategy {
    /// 自动压缩（接近限制时触发）
    #[default]
    Auto,
    /// 响应式压缩（API 拒绝时触发）
    Reactive,
    /// 手动压缩（用户触发）
    Manual,
    /// 会话记忆压缩
    SessionMemory,
}

/// 压缩配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactConfig {
    /// 是否启用自动压缩
    pub auto_compact_enabled: bool,
    /// 自动压缩缓冲区 tokens（保留这么多 tokens 的余量）
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

impl Default for CompactConfig {
    fn default() -> Self {
        Self {
            auto_compact_enabled: true,
            auto_compact_buffer_tokens: 13_000,      // Claude Code 默认值
            warning_buffer_tokens: 20_000,           // 警告阈值
            error_buffer_tokens: 20_000,             // 错误阈值
            manual_compact_buffer_tokens: 3_000,     // 手动压缩缓冲区
            strategy: CompactStrategy::Auto,
            micro_compact_interval: 10,              // 每 10 条消息微压缩
            post_compact_max_files_to_restore: 5,    // 恢复 5 个文件
            post_compact_token_budget: 50_000,       // 50K tokens 预算
        }
    }
}

impl CompactConfig {
    /// 创建新的压缩配置
    pub fn new() -> Self {
        Self::default()
    }
    
    /// 设置是否启用自动压缩
    pub fn with_auto_compact(mut self, enabled: bool) -> Self {
        self.auto_compact_enabled = enabled;
        self
    }
    
    /// 设置压缩策略
    pub fn with_strategy(mut self, strategy: CompactStrategy) -> Self {
        self.strategy = strategy;
        self
    }
    
    /// 设置自动压缩缓冲区
    pub fn with_buffer_tokens(mut self, tokens: u32) -> Self {
        self.auto_compact_buffer_tokens = tokens;
        self
    }
}

/// 压缩触发阈值计算
impl CompactConfig {
    /// 获取自动压缩触发阈值
    pub fn get_auto_compact_threshold(&self, context_window: u32) -> u32 {
        context_window.saturating_sub(self.auto_compact_buffer_tokens)
    }
    
    /// 获取警告阈值
    pub fn get_warning_threshold(&self, context_window: u32) -> u32 {
        context_window.saturating_sub(self.warning_buffer_tokens)
    }
    
    /// 获取错误阈值
    pub fn get_error_threshold(&self, context_window: u32) -> u32 {
        context_window.saturating_sub(self.error_buffer_tokens)
    }
    
    /// 获取手动压缩阈值
    pub fn get_manual_compact_threshold(&self, context_window: u32) -> u32 {
        context_window.saturating_sub(self.manual_compact_buffer_tokens)
    }
    
    /// 检查是否应该压缩
    pub fn should_compact(&self, current_tokens: u32, context_window: u32) -> bool {
        if !self.auto_compact_enabled {
            return false;
        }
        
        let threshold = self.get_auto_compact_threshold(context_window);
        current_tokens >= threshold
    }
    
    /// 检查是否达到警告级别
    pub fn is_at_warning_level(&self, current_tokens: u32, context_window: u32) -> bool {
        current_tokens >= self.get_warning_threshold(context_window)
    }
    
    /// 检查是否达到错误级别
    pub fn is_at_error_level(&self, current_tokens: u32, context_window: u32) -> bool {
        current_tokens >= self.get_error_threshold(context_window)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = CompactConfig::default();
        assert!(config.auto_compact_enabled);
        assert_eq!(config.auto_compact_buffer_tokens, 13_000);
        assert_eq!(config.warning_buffer_tokens, 20_000);
    }
    
    #[test]
    fn test_threshold_calculation() {
        let config = CompactConfig::default();
        let context_window = 200_000; // Claude 200K 上下文
        
        assert_eq!(config.get_auto_compact_threshold(context_window), 187_000);
        assert_eq!(config.get_warning_threshold(context_window), 180_000);
        assert_eq!(config.get_error_threshold(context_window), 180_000);
    }
    
    #[test]
    fn test_should_compact() {
        let config = CompactConfig::default();
        let context_window = 200_000;
        
        // 未达到阈值
        assert!(!config.should_compact(150_000, context_window));
        
        // 达到阈值
        assert!(config.should_compact(190_000, context_window));
        
        // 禁用自动压缩
        let config_disabled = CompactConfig::new().with_auto_compact(false);
        assert!(!config_disabled.should_compact(190_000, context_window));
    }
}
