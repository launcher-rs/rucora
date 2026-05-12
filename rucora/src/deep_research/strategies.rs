//! 研究策略实现

use async_trait::async_trait;
use rucora_core::provider::LlmProvider;
use rucora_core::research::{
    ResearchConfig, ResearchContext, ResearchQualityAssessor, StrategyResult,
};
use std::sync::Arc;

/// 标准多阶段研究策略
///
/// 典型的三阶段流程：
/// 1. 搜索收集 - 多轮搜索收集信息
/// 2. 深度精读 - 对重要 URL 进行深度阅读
/// 3. 综合报告 - 汇总所有信息生成报告
pub struct StandardStrategy {
    config: ResearchConfig,
}

impl StandardStrategy {
    pub fn new() -> Self {
        Self {
            config: ResearchConfig::default(),
        }
    }

    pub fn with_config(config: ResearchConfig) -> Self {
        Self { config }
    }

    pub fn with_topic(config: ResearchConfig, topic: &str) -> Self {
        let _ = topic;
        Self { config }
    }
}

impl Default for StandardStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl rucora_core::research::StrategyTrait for StandardStrategy {
    fn name(&self) -> &'static str {
        "standard"
    }

    fn description(&self) -> &'static str {
        "标准多阶段研究策略：搜索 → 精读 → 综合报告"
    }

    async fn search(
        &self,
        _provider: &Arc<dyn LlmProvider>,
        topic: &str,
        context: &mut ResearchContext,
    ) -> Result<StrategyResult, rucora_core::research::ResearchError> {
        use rucora_core::research::InfoPiece;

        let mut search_round: u32 = 0;
        let max_rounds = self.config.max_iterations;

        // 初始化评估器
        let assessor = ResearchQualityAssessor::with_default(topic);

        loop {
            search_round += 1;

            // 模拟搜索过程（实际实现需要调用搜索工具）
            // 这里生成一些示例数据用于演示评分逻辑
            let dummy_info = InfoPiece::new(
                format!("关于 {topic} 的研究信息 #{search_round}"),
                Some("https://example.com".to_string()),
                rucora_core::research::SourceType::Official,
            );
            context.add_info(dummy_info);

            // 评估当前研究质量
            let score = assessor.assess(
                &context.collected_info,
                &context.citations,
                search_round as usize,
            );

            // 生成改进建议
            let suggestion = assessor.suggest(&score);

            // 设置置信度
            let confidence = score.overall;

            // 检查是否应该继续
            if !assessor.should_continue(&score, search_round, max_rounds) {
                let mut result = StrategyResult::complete();
                result.confidence = confidence;
                result.search_count = search_round;
                result.new_info = context.collected_info.clone();
                return Ok(result);
            }

            // 如果达到最大轮次，停止
            if search_round >= max_rounds {
                let result = StrategyResult {
                    confidence,
                    search_count: search_round,
                    new_info: context.collected_info.clone(),
                    is_complete: true,
                    ..Default::default()
                };
                return Ok(result);
            }

            // 输出调试信息（实际应用中可以通过日志输出）
            tracing::debug!(
                "第 {} 轮研究: 评分={:.2} ({}), 建议: {}",
                search_round,
                score.overall,
                score.level(),
                suggestion.description
            );
        }
    }

    fn should_continue(&self, result: &StrategyResult) -> bool {
        !result.is_complete && result.confidence < 0.8
    }

    fn max_iterations(&self) -> Option<u32> {
        Some(self.config.max_iterations)
    }

    fn config(&self) -> &ResearchConfig {
        &self.config
    }
}

/// 快速研究策略
///
/// 适用于简单事实查询，30秒-3分钟内完成。
pub struct FastStrategy {
    config: ResearchConfig,
}

impl FastStrategy {
    pub fn new() -> Self {
        Self {
            config: ResearchConfig::fast(),
        }
    }
}

impl Default for FastStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl rucora_core::research::StrategyTrait for FastStrategy {
    fn name(&self) -> &'static str {
        "fast"
    }

    fn description(&self) -> &'static str {
        "快速研究策略：单次搜索 + 快速综合"
    }

    async fn search(
        &self,
        _provider: &Arc<dyn LlmProvider>,
        topic: &str,
        context: &mut ResearchContext,
    ) -> Result<StrategyResult, rucora_core::research::ResearchError> {
        use rucora_core::research::InfoPiece;

        let assessor = ResearchQualityAssessor::with_default(topic);
        let search_count: u32 = 1;

        // 快速策略：只进行一轮搜索和评估
        let dummy_info = InfoPiece::new(
            format!("关于 {topic} 的快速信息"),
            Some("https://example.com".to_string()),
            rucora_core::research::SourceType::Official,
        );
        context.add_info(dummy_info);

        let score = assessor.assess(
            &context.collected_info,
            &context.citations,
            search_count as usize,
        );

        let mut result = StrategyResult::complete();
        result.confidence = score.overall;
        result.search_count = search_count;
        result.new_info = context.collected_info.clone();

        Ok(result)
    }

    fn should_continue(&self, result: &StrategyResult) -> bool {
        !result.is_complete && result.search_count < self.config.max_iterations
    }

    fn max_iterations(&self) -> Option<u32> {
        Some(self.config.max_iterations)
    }

    fn config(&self) -> &ResearchConfig {
        &self.config
    }
}