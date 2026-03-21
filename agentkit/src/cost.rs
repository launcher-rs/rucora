//! Token 计数和成本管理模块
//!
//! # 概述
//!
//! 本模块提供 Token 计数和 API 成本管理功能，支持：
//! - 精确的 Token 计数
//! - 成本估算和追踪
//! - 预算控制
//! - 使用统计
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use agentkit::cost::{TokenCounter, CostTracker};
//!
//! // 创建 Token 计数器
//! let counter = TokenCounter::new("gpt-4");
//! let tokens = counter.count_text("Hello, World!");
//!
//! // 创建成本追踪器
//! let mut tracker = CostTracker::new();
//! tracker.record_usage("gpt-4", 100, 200, 0.0001);
//!
//! // 获取当前成本
//! let cost = tracker.get_current_cost();
//!
//! // 检查预算
//! if tracker.check_budget(10.0) {
//!     println!("预算充足");
//! }
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Token 计数器
///
/// 支持不同模型的 Token 计数。
#[derive(Debug, Clone)]
pub struct TokenCounter {
    /// 模型名称
    model: String,
    /// 每 Token 字符数估算（英文 4，中文 1.5）
    chars_per_token: f64,
}

impl TokenCounter {
    /// 创建新的 Token 计数器
    pub fn new(model: &str) -> Self {
        let chars_per_token = Self::get_chars_per_token(model);
        Self {
            model: model.to_string(),
            chars_per_token,
        }
    }

    /// 获取模型的字符/Token 比率
    fn get_chars_per_token(model: &str) -> f64 {
        // 根据模型类型返回不同的估算值
        if model.contains("gpt-4") || model.contains("claude") {
            3.5 // 高级模型更精确
        } else if model.contains("zh") || model.contains("chinese") {
            1.5 // 中文
        } else {
            4.0 // 默认英文
        }
    }

    /// 计算文本的 Token 数（估算）
    pub fn count_text(&self, text: &str) -> usize {
        let chars = text.chars().count() as f64;
        (chars / self.chars_per_token).ceil() as usize
    }

    /// 计算消息列表的 Token 数
    pub fn count_messages(
        &self,
        messages: &[agentkit_core::provider::types::ChatMessage],
    ) -> usize {
        let mut total = 0;
        for msg in messages {
            // 每条消息的基础 Token 开销
            total += 4; // 消息格式开销

            // 角色 Token
            total += match msg.role {
                agentkit_core::provider::types::Role::System => 2,
                agentkit_core::provider::types::Role::User => 1,
                agentkit_core::provider::types::Role::Assistant => 1,
                agentkit_core::provider::types::Role::Tool => 3,
            };

            // 内容 Token
            total += self.count_text(&msg.content);

            // 名称 Token（如果有）
            if let Some(name) = &msg.name {
                total += self.count_text(name) + 1;
            }
        }
        total
    }

    /// 计算工具定义的 Token 数
    pub fn count_tools(&self, tools: &[agentkit_core::tool::types::ToolDefinition]) -> usize {
        let mut total = 0;
        for tool in tools {
            total += self.count_text(&tool.name);
            if let Some(desc) = &tool.description {
                total += self.count_text(desc);
            }
            // JSON schema 估算
            let schema_str = tool.input_schema.to_string();
            total += self.count_text(&schema_str);
        }
        total
    }

    /// 获取模型名称
    pub fn model(&self) -> &str {
        &self.model
    }
}

/// Token 使用记录
#[derive(Debug, Clone)]
pub struct TokenUsage {
    /// 输入 Token 数
    pub prompt_tokens: usize,
    /// 输出 Token 数
    pub completion_tokens: usize,
    /// 总 Token 数
    pub total_tokens: usize,
}

impl TokenUsage {
    /// 创建新的使用记录
    pub fn new(prompt_tokens: usize, completion_tokens: usize) -> Self {
        Self {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
        }
    }
}

/// 成本追踪器
///
/// 追踪 API 调用成本和 Token 使用量。
#[derive(Debug, Default)]
pub struct CostTracker {
    /// 使用记录
    records: Arc<RwLock<Vec<UsageRecord>>>,
    /// 预算限制（美元）
    budget_limit: Arc<RwLock<Option<f64>>>,
}

impl CostTracker {
    /// 创建新的成本追踪器
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置预算限制
    pub fn with_budget_limit(mut self, limit: f64) -> Self {
        self.budget_limit = Arc::new(RwLock::new(Some(limit)));
        self
    }

    /// 记录使用量
    ///
    /// # 参数
    ///
    /// - `model`: 模型名称
    /// - `prompt_tokens`: 输入 Token 数
    /// - `completion_tokens`: 输出 Token 数
    /// - `cost`: 总成本（美元）
    pub async fn record_usage(
        &self,
        model: &str,
        prompt_tokens: usize,
        completion_tokens: usize,
        cost: f64,
    ) {
        let mut records = self.records.write().await;
        records.push(UsageRecord {
            model: model.to_string(),
            usage: TokenUsage::new(prompt_tokens, completion_tokens),
            cost,
            timestamp: std::time::SystemTime::now(),
        });
    }

    /// 获取当前总成本
    pub async fn get_current_cost(&self) -> f64 {
        let records = self.records.read().await;
        records.iter().map(|r| r.cost).sum()
    }

    /// 获取总 Token 使用量
    pub async fn get_total_usage(&self) -> TokenUsage {
        let records = self.records.read().await;
        let mut total = TokenUsage::new(0, 0);
        for record in records.iter() {
            total.prompt_tokens += record.usage.prompt_tokens;
            total.completion_tokens += record.usage.completion_tokens;
            total.total_tokens += record.usage.total_tokens;
        }
        total
    }

    /// 检查是否超出预算
    pub async fn check_budget(&self, budget: f64) -> bool {
        let current = self.get_current_cost().await;
        current <= budget
    }

    /// 获取使用统计
    pub async fn get_statistics(&self) -> UsageStatistics {
        let records = self.records.read().await;

        let mut stats = UsageStatistics::default();
        for record in records.iter() {
            stats.total_cost += record.cost;
            stats.total_prompt_tokens += record.usage.prompt_tokens;
            stats.total_completion_tokens += record.usage.completion_tokens;
            stats.total_requests += 1;

            stats
                .models
                .entry(record.model.clone())
                .or_insert_with(|| ModelStats::default())
                .requests += 1;
        }

        stats
    }

    /// 清空记录
    pub async fn clear(&self) {
        let mut records = self.records.write().await;
        records.clear();
    }
}

/// 使用记录
#[derive(Debug, Clone)]
pub struct UsageRecord {
    /// 模型名称
    pub model: String,
    /// Token 使用量
    pub usage: TokenUsage,
    /// 成本（美元）
    pub cost: f64,
    /// 时间戳
    pub timestamp: std::time::SystemTime,
}

/// 使用统计
#[derive(Debug, Default)]
pub struct UsageStatistics {
    /// 总成本
    pub total_cost: f64,
    /// 总输入 Token 数
    pub total_prompt_tokens: usize,
    /// 总输出 Token 数
    pub total_completion_tokens: usize,
    /// 总请求数
    pub total_requests: usize,
    /// 按模型分类的统计
    pub models: HashMap<String, ModelStats>,
}

/// 模型统计
#[derive(Debug, Default)]
pub struct ModelStats {
    /// 请求数
    pub requests: usize,
}

/// 预定义的模型价格（每 1000 Token）
pub mod pricing {
    /// OpenAI GPT-4 价格（每 1000 tokens）
    pub const GPT_4: f64 = 0.03;
    /// OpenAI GPT-4 Turbo 价格
    pub const GPT_4_TURBO: f64 = 0.01;
    /// OpenAI GPT-3.5 Turbo 价格
    pub const GPT_3_5_TURBO: f64 = 0.0005;
    /// Anthropic Claude 3 Opus 价格
    pub const CLAUDE_3_OPUS: f64 = 0.015;
    /// Anthropic Claude 3 Sonnet 价格
    pub const CLAUDE_3_SONNET: f64 = 0.003;
}

/// 根据模型和使用量计算成本
pub fn calculate_cost(model: &str, prompt_tokens: usize, completion_tokens: usize) -> f64 {
    let (prompt_price, completion_price) = get_model_prices(model);

    let prompt_cost = (prompt_tokens as f64 / 1000.0) * prompt_price;
    let completion_cost = (completion_tokens as f64 / 1000.0) * completion_price;

    prompt_cost + completion_cost
}

/// 获取模型价格
fn get_model_prices(model: &str) -> (f64, f64) {
    // 简化实现，实际应该使用完整的价格表
    if model.contains("gpt-4") && !model.contains("turbo") {
        (pricing::GPT_4, pricing::GPT_4 * 2.0)
    } else if model.contains("gpt-4") {
        (pricing::GPT_4_TURBO, pricing::GPT_4_TURBO * 2.0)
    } else if model.contains("gpt-3") {
        (pricing::GPT_3_5_TURBO, pricing::GPT_3_5_TURBO * 2.0)
    } else if model.contains("claude-3-opus") {
        (pricing::CLAUDE_3_OPUS, pricing::CLAUDE_3_OPUS * 2.0)
    } else if model.contains("claude") {
        (pricing::CLAUDE_3_SONNET, pricing::CLAUDE_3_SONNET * 2.0)
    } else {
        (0.001, 0.002) // 默认价格
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_counter_basic() {
        let counter = TokenCounter::new("gpt-4");

        // 英文测试
        let tokens = counter.count_text("Hello, World!");
        assert!(tokens > 0);

        // 中文测试
        let tokens = counter.count_text("你好世界");
        assert!(tokens > 0);
    }

    #[test]
    fn test_token_counter_models() {
        let gpt4 = TokenCounter::new("gpt-4");
        let gpt35 = TokenCounter::new("gpt-3.5-turbo");

        // 不同模型应该有不同的计数
        let text = "Hello";
        let tokens_gpt4 = gpt4.count_text(text);
        let tokens_gpt35 = gpt35.count_text(text);

        assert!(tokens_gpt4 > 0);
        assert!(tokens_gpt35 > 0);
    }

    #[tokio::test]
    async fn test_cost_tracker() {
        let tracker = CostTracker::new();

        // 记录使用
        tracker.record_usage("gpt-4", 100, 50, 0.0045).await;
        tracker
            .record_usage("gpt-3.5-turbo", 200, 100, 0.00015)
            .await;

        // 检查总成本
        let cost = tracker.get_current_cost().await;
        assert!((cost - 0.00465).abs() < 0.0001);

        // 检查使用量
        let usage = tracker.get_total_usage().await;
        assert_eq!(usage.prompt_tokens, 300);
        assert_eq!(usage.completion_tokens, 150);

        // 检查预算
        assert!(tracker.check_budget(1.0).await);
        assert!(!tracker.check_budget(0.001).await);
    }

    #[test]
    fn test_calculate_cost() {
        let cost = calculate_cost("gpt-4", 1000, 500);
        assert!(cost > 0.0);
    }
}
