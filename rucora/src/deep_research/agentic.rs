//! Agentic 自主研究策略
//!
//! LLM 自主决策的研究流程，类似 LangGraph 风格的自主 Agent。

use async_trait::async_trait;
use rucora_core::provider::LlmProvider;
use rucora_core::research::{ResearchConfig, ResearchContext, StrategyResult};
use std::sync::Arc;

/// Agentic 自主研究策略
///
/// 核心特点：
/// - LLM 自主决定搜索策略
/// - 自适应切换搜索引擎
/// - 基于发现动态调整研究路径
/// - 迭代式发现和验证
pub struct AgenticStrategy {
    config: ResearchConfig,
    confidence_threshold: f32,
}

impl AgenticStrategy {
    pub fn new() -> Self {
        Self {
            config: ResearchConfig::agentic(),
            confidence_threshold: 0.8,
        }
    }

    pub fn with_config(config: ResearchConfig) -> Self {
        Self {
            config: config.clone(),
            confidence_threshold: 0.8,
        }
    }

    pub fn with_confidence_threshold(mut self, threshold: f32) -> Self {
        self.confidence_threshold = threshold;
        self
    }
}

impl Default for AgenticStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl rucora_core::research::StrategyTrait for AgenticStrategy {
    fn name(&self) -> &'static str {
        "agentic"
    }

    fn description(&self) -> &'static str {
        "Agentic 自主研究策略：LLM 自主决策，动态调整研究路径"
    }

    async fn search(
        &self,
        provider: &Arc<dyn LlmProvider>,
        topic: &str,
        context: &mut ResearchContext,
    ) -> Result<StrategyResult, rucora_core::research::ResearchError> {
        let max_iterations = self.config.max_iterations as usize;
        let mut iteration = 0;
        let mut current_result = StrategyResult::default();

        // Agentic 循环：让 LLM 自主决定下一步
        while iteration < max_iterations && self.should_continue(&current_result) {
            iteration += 1;
            context.set_phase(rucora_core::research::ResearchPhase::Search);

            // 1. 让 LLM 分析当前状态，决定下一步
            let decision = self
                .decide_next_action(provider, topic, context)
                .await?;

            // 2. 更新搜索计数
            current_result.search_count = iteration as u32;

            // 3. 检查是否应该综合
            let is_complete = matches!(
                decision.action,
                AgentAction::Synthesize | AgentAction::Done
            );
            if is_complete {
                current_result.is_complete = true;
                break;
            }

            // 4. 执行决定的动作（这里只是模拟，实际需要工具调用）
            // 在实际实现中，这里会调用搜索工具或阅读工具
            current_result.confidence = decision.confidence;
        }

        Ok(current_result)
    }

    fn should_continue(&self, result: &StrategyResult) -> bool {
        !result.is_complete && result.confidence < self.confidence_threshold
    }

    fn max_iterations(&self) -> Option<u32> {
        Some(self.config.max_iterations)
    }

    fn config(&self) -> &ResearchConfig {
        &self.config
    }
}

impl AgenticStrategy {
    /// 让 LLM 决定下一步行动
    async fn decide_next_action(
        &self,
        _provider: &Arc<dyn LlmProvider>,
        topic: &str,
        context: &ResearchContext,
    ) -> Result<AgentDecision, rucora_core::research::ResearchError> {
        // 构建上下文信息
        let _context_info = format!(
            "研究主题: {}\n\
             已搜索 {} 次\n\
             已收集 {} 条信息\n\
             已访问 {} 个 URL\n\
             当前置信度: {:.2}",
            topic,
            context.search_history.len(),
            context.collected_info.len(),
            context.visited_urls.len(),
            0.5 // 初始置信度
        );

        // 这里需要使用 LLM 生成下一步决策
        // 实际实现中会发送一个提示给 LLM，让它返回 JSON 格式的决策

        // 临时返回继续搜索的决策
        Ok(AgentDecision {
            action: AgentAction::Search("继续搜索".to_string()),
            reasoning: "需要更多搜索来提高置信度".to_string(),
            confidence: 0.6,
        })
    }
}

/// Agent 决策
#[derive(Debug, Clone)]
pub struct AgentDecision {
    pub action: AgentAction,
    pub reasoning: String,
    pub confidence: f32,
}

/// Agent 可执行的动作
#[derive(Debug, Clone, PartialEq)]
pub enum AgentAction {
    /// 搜索
    Search(String),
    /// 读取 URL
    Read(String),
    /// 综合信息
    Synthesize,
    /// 完成
    Done,
}