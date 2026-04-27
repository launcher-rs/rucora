//! 分层上下文压缩引擎
//!
//! 参考 Hermes Agent 的上下文压缩设计，实现智能的分层压缩算法：
//! 1. 修剪旧工具结果（廉价预压缩）
//! 2. 保护头部消息（系统提示 + 首次交互）
//! 3. 按 Token 预算保护尾部消息（最近 ~20K tokens）
//! 4. 用结构化 LLM 提示摘要中间回合
//! 5. 后续压缩时迭代更新先前摘要
//!
//! # 设计目标
//!
//! - **分层保护**: 头部/尾部分离保护，只压缩中间部分
//! - **结构化摘要**: 使用结构化模板提取关键信息
//! - **迭代更新**: 后续压缩时更新先前摘要，保持信息新鲜度
//! - **成本控制**: 避免过度压缩导致信息丢失

use agentkit_core::provider::LlmProvider;
use agentkit_core::provider::types::{ChatMessage, Role};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// 压缩策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionStrategy {
    /// 激进压缩（尽可能压缩，适合长对话）
    Aggressive,
    /// 平衡压缩（保留更多上下文，适合中等对话）
    Balanced,
    /// 保守压缩（只压缩必要部分，适合短对话）
    Conservative,
}

impl Default for CompressionStrategy {
    fn default() -> Self {
        Self::Balanced
    }
}

/// 压缩配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// 压缩策略
    pub strategy: CompressionStrategy,
    /// 保护头部消息数量（这些消息不会被压缩）
    pub protect_head_count: usize,
    /// 保护尾部消息 Token 数（最近的消息保留这么多 token）
    pub protect_tail_tokens: usize,
    /// 触发压缩的上下文使用率阈值（0.0-1.0）
    pub compression_threshold: f64,
    /// 压缩后的目标上下文使用率
    pub target_usage_ratio: f64,
    /// 最大压缩迭代次数
    pub max_iterations: usize,
    /// 摘要失败冷却期（秒），防止频繁重试
    pub summary_cooldown_seconds: u64,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            strategy: CompressionStrategy::Balanced,
            protect_head_count: 3,
            protect_tail_tokens: 20_000,
            compression_threshold: 0.85,
            target_usage_ratio: 0.60,
            max_iterations: 3,
            summary_cooldown_seconds: 600,
        }
    }
}

impl CompressionConfig {
    /// 创建激进压缩配置
    pub fn aggressive() -> Self {
        Self {
            strategy: CompressionStrategy::Aggressive,
            protect_head_count: 2,
            protect_tail_tokens: 15_000,
            compression_threshold: 0.80,
            target_usage_ratio: 0.50,
            ..Default::default()
        }
    }

    /// 创建保守压缩配置
    pub fn conservative() -> Self {
        Self {
            strategy: CompressionStrategy::Conservative,
            protect_head_count: 5,
            protect_tail_tokens: 25_000,
            compression_threshold: 0.90,
            target_usage_ratio: 0.70,
            ..Default::default()
        }
    }
}

/// 结构化摘要模板
///
/// 参考 Hermes Agent 的摘要模板设计，提取对话中的关键信息。
const STRUCTURED_SUMMARY_TEMPLATE: &str = r#"请对以下对话进行结构化摘要，以便后续继续工作而不丢失关键上下文。

## Goal — 用户试图完成什么
[描述用户的主要目标和任务]

## Constraints & Preferences — 用户偏好、编码风格
[记录用户的特殊要求、偏好、编码风格等]

## Progress — Done / In Progress / Blocked
- **Done**: [已完成的工作]
- **In Progress**: [正在进行的工作]
- **Blocked**: [阻塞的问题]

## Key Decisions — 重要技术决策
[记录重要的技术决策及其原因]

## Resolved Questions — 已回答的问题
[已解决的问题，防止重新回答]

## Pending User Asks — 未回答的问题
[用户提出但尚未回答的问题]

## Relevant Files — 读取/修改/创建的文件
[相关文件列表]

## Remaining Work — 剩余工作
[还需要完成的工作]

## Critical Context — 不能丢失的具体值
[重要的代码片段、配置值、URL 等]

## Tools & Patterns — 使用过的工具及有效用法
[使用过的工具和有效的工作模式]

---

请基于以上模板对对话进行摘要，保持简洁但完整。"#;

/// 分层压缩引擎
///
/// 实现智能的上下文压缩，保护关键信息并压缩冗余内容。
pub struct LayeredCompressor {
    /// 压缩配置
    config: CompressionConfig,
    /// 上次摘要时间（防止频繁压缩）
    last_summary_timestamp: Option<u64>,
    /// 上次摘要内容（用于迭代更新）
    last_summary_content: Option<String>,
}

impl LayeredCompressor {
    /// 创建新的压缩引擎
    pub fn new(config: CompressionConfig) -> Self {
        Self {
            config,
            last_summary_timestamp: None,
            last_summary_content: None,
        }
    }

    /// 创建默认引擎
    pub fn default_engine() -> Self {
        Self::new(CompressionConfig::default())
    }

    /// 判断是否需要压缩
    ///
    /// # 参数
    ///
    /// - `current_tokens`: 当前上下文 Token 数
    /// - `context_window`: 模型的上下文窗口大小
    ///
    /// # 返回
    ///
    /// 如果应该压缩则返回 true
    pub fn should_compress(&self, current_tokens: usize, context_window: usize) -> bool {
        if context_window == 0 {
            return false;
        }

        let usage_ratio = current_tokens as f64 / context_window as f64;

        // 检查是否超过压缩阈值
        if usage_ratio < self.config.compression_threshold {
            return false;
        }

        // 检查冷却期（防止频繁压缩）
        if let Some(last_ts) = self.last_summary_timestamp {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);

            if now - last_ts < self.config.summary_cooldown_seconds {
                debug!(
                    elapsed = now - last_ts,
                    cooldown = self.config.summary_cooldown_seconds,
                    "压缩冷却期，跳过压缩"
                );
                return false;
            }
        }

        true
    }

    /// 执行分层压缩
    ///
    /// # 参数
    ///
    /// - `provider`: LLM Provider（用于生成摘要）
    /// - `messages`: 当前对话消息列表
    /// - `context_window`: 模型的上下文窗口大小
    ///
    /// # 返回
    ///
    /// 压缩后的消息列表
    pub async fn compress(
        &mut self,
        provider: &dyn LlmProvider,
        messages: Vec<ChatMessage>,
        context_window: usize,
    ) -> Result<Vec<ChatMessage>, Box<dyn std::error::Error + Send + Sync>> {
        info!(
            original_count = messages.len(),
            context_window = context_window,
            "开始分层压缩"
        );

        // 记录原始消息数量用于计算压缩率
        let original_count = messages.len();

        // 步骤 1: 修剪旧工具结果（廉价预压缩）
        let messages = self.trim_old_tool_results(messages);

        // 步骤 2: 分离头部/中间/尾部
        let (head, middle, tail) = self.split_messages(messages);

        debug!(
            head_count = head.len(),
            middle_count = middle.len(),
            tail_count = tail.len(),
            "消息分层完成"
        );

        // 如果中间没有消息，无需压缩
        if middle.is_empty() {
            info!("无中间消息，跳过压缩");
            return Ok([head, middle, tail].concat());
        }

        // 步骤 3: 生成结构化摘要
        let summary = self.generate_structured_summary(provider, &middle).await?;

        // 更新摘要元数据
        self.last_summary_timestamp = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        );
        self.last_summary_content = Some(summary.clone());

        // 步骤 4: 创建摘要消息
        let summary_message = ChatMessage::system(format!(
            "<conversation-summary>\n{}\n</conversation-summary>\n\n\
             以上是之前对话的结构化摘要。请基于此摘要和后续对话继续工作。",
            summary
        ));

        // 步骤 5: 重组消息
        let compressed = [head, vec![summary_message], tail].concat();

        info!(
            compressed_count = compressed.len(),
            compression_ratio = format!(
                "{:.1}%",
                (1.0 - compressed.len() as f64 / original_count as f64) * 100.0
            ),
            "压缩完成"
        );

        Ok(compressed)
    }

    /// 修剪旧工具结果（廉价预压缩）
    ///
    /// 移除早期回合中的工具调用结果，保留最近的工具结果。
    fn trim_old_tool_results(&self, messages: Vec<ChatMessage>) -> Vec<ChatMessage> {
        let mut trimmed = Vec::new();
        let mut tool_result_count = 0;
        let max_tool_results = match self.config.strategy {
            CompressionStrategy::Aggressive => 2,
            CompressionStrategy::Balanced => 4,
            CompressionStrategy::Conservative => 6,
        };

        // 反向遍历，保留最近的工具结果
        let mut messages_reversed = messages;
        messages_reversed.reverse();

        for msg in messages_reversed {
            if msg.role == Role::Tool {
                if tool_result_count < max_tool_results {
                    trimmed.push(msg);
                    tool_result_count += 1;
                }
                // 否则跳过旧的工具结果
            } else {
                trimmed.push(msg);
            }
        }

        // 恢复原始顺序
        trimmed.reverse();
        trimmed
    }

    /// 分离消息为头部/中间/尾部
    fn split_messages(
        &self,
        messages: Vec<ChatMessage>,
    ) -> (Vec<ChatMessage>, Vec<ChatMessage>, Vec<ChatMessage>) {
        let head_count = self.config.protect_head_count.min(messages.len());
        let head: Vec<ChatMessage> = messages[..head_count].to_vec();

        // 估算尾部消息的 token 数，保护最近的 N 个 token
        let mut tail_count = 0;
        let mut tail_tokens = 0;
        let token_counter = TokenCounter::new();

        for msg in messages.iter().rev() {
            let role_str = match msg.role {
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::System => "system",
                Role::Tool => "tool",
            };
            let tokens = token_counter.estimate_message(&msg.content, role_str);
            if tail_tokens + tokens > self.config.protect_tail_tokens {
                break;
            }
            tail_tokens += tokens;
            tail_count += 1;
        }

        let tail_start = messages.len().saturating_sub(tail_count);
        // 确保 tail_start 不小于 head_count
        let tail_start = tail_start.max(head_count);
        let tail: Vec<ChatMessage> = messages[tail_start..].to_vec();
        let middle: Vec<ChatMessage> = messages[head_count..tail_start].to_vec();

        (head, middle, tail)
    }

    /// 生成结构化摘要
    async fn generate_structured_summary(
        &self,
        provider: &dyn LlmProvider,
        messages: &[ChatMessage],
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // 将消息转换为文本
        let context_text = messages
            .iter()
            .map(|m| format!("[{}]: {}", Self::role_name(&m.role), m.content))
            .collect::<Vec<_>>()
            .join("\n\n");

        let prompt = if let Some(previous_summary) = &self.last_summary_content {
            // 迭代更新先前摘要
            format!(
                "这是之前的对话摘要：\n{}\n\n---\n\n这是新的对话内容：\n{}\n\n\
                 请更新之前的摘要以反映新的进展，保持结构化格式。",
                previous_summary, context_text
            )
        } else {
            // 首次生成
            format!(
                "{}\n\n---\n\n对话内容：\n{}",
                STRUCTURED_SUMMARY_TEMPLATE, context_text
            )
        };

        let request = agentkit_core::provider::types::ChatRequest::from_user_text(prompt);

        let response = provider.chat(request).await?;
        Ok(response.message.content)
    }

    /// 获取角色名称
    fn role_name(role: &Role) -> &'static str {
        match role {
            Role::User => "用户",
            Role::Assistant => "助手",
            Role::System => "系统",
            Role::Tool => "工具",
        }
    }

    /// 获取上次摘要内容
    pub fn last_summary(&self) -> Option<&String> {
        self.last_summary_content.as_ref()
    }
}

/// Token 计数器（简化版）
struct TokenCounter {
    avg_chars_per_token: f64,
}

impl TokenCounter {
    fn new() -> Self {
        Self {
            avg_chars_per_token: 4.0, // 英文平均 4 字符/token
        }
    }

    fn estimate_message(&self, content: &str, _role: &str) -> usize {
        let char_count = content.chars().count() as f64;
        let base_tokens = (char_count / self.avg_chars_per_token) as usize;
        // 角色开销约 4 tokens
        base_tokens + 4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_compress() {
        let engine = LayeredCompressor::default_engine();

        // 未达到阈值
        assert!(!engine.should_compress(10_000, 128_000));

        // 超过阈值
        assert!(engine.should_compress(110_000, 128_000));
    }

    #[test]
    fn test_trim_tool_results() {
        let engine = LayeredCompressor::new(CompressionConfig::aggressive());
        let messages = vec![
            ChatMessage::user("Hello"),
            ChatMessage::assistant("Hi"),
            ChatMessage::tool("tool1".to_string(), "result1".to_string()),
            ChatMessage::assistant("Done"),
            ChatMessage::tool("tool2".to_string(), "result2".to_string()),
            ChatMessage::assistant("Done2"),
            ChatMessage::tool("tool3".to_string(), "result3".to_string()),
            ChatMessage::tool("tool4".to_string(), "result4".to_string()),
            ChatMessage::tool("tool5".to_string(), "result5".to_string()),
        ];

        let trimmed = engine.trim_old_tool_results(messages);
        // Aggressive 策略最多保留 2 个工具结果
        let tool_count = trimmed.iter().filter(|m| m.role == Role::Tool).count();
        assert!(tool_count <= 2);
    }

    #[test]
    fn test_split_messages() {
        let engine = LayeredCompressor::default_engine();
        let messages: Vec<ChatMessage> = (0..20)
            .map(|i| {
                if i % 2 == 0 {
                    ChatMessage::user(format!("User message {}", i))
                } else {
                    ChatMessage::assistant(format!("Assistant message {}", i))
                }
            })
            .collect();

        let (head, middle, tail) = engine.split_messages(messages);

        // 头部保护 3 条消息
        assert_eq!(head.len(), 3);
        // 尾部至少 有几条消息
        assert!(!tail.is_empty());
        // 中间消息应该比原始少
        assert!(middle.len() < 17);
    }
}
