//! 默认研究引擎实现

use async_trait::async_trait;
use rucora_core::research::{
    ResearchConfig, ResearchContext, ResearchPhase, ResearchProgress, ResearchReport,
    ResearchStrategy, StrategyTrait,
};
use rucora_core::provider::LlmProvider;
use std::sync::{Arc, RwLock};

/// 默认深度研究引擎
pub struct DefaultResearchEngine {
    config: ResearchConfig,
    strategy: Box<dyn StrategyTrait>,
    progress: Arc<RwLock<ResearchProgress>>,
}

impl DefaultResearchEngine {
    pub fn new(strategy: Box<dyn StrategyTrait>) -> Self {
        let config = strategy.config().clone();
        Self {
            config,
            strategy,
            progress: Arc::new(RwLock::new(ResearchProgress::new(
                ResearchPhase::Init,
                10,
            ))),
        }
    }

    pub fn with_config(config: ResearchConfig) -> Self {
        let max_iter = config.max_iterations;
        let strategy: Box<dyn StrategyTrait> = match config.strategy {
            ResearchStrategy::Fast => Box::new(super::strategies::FastStrategy::new()),
            _ => Box::new(super::strategies::StandardStrategy::with_config(config.clone())),
        };
        Self {
            config,
            strategy,
            progress: Arc::new(RwLock::new(ResearchProgress::new(
                ResearchPhase::Init,
                max_iter,
            ))),
        }
    }

    /// 使用指定的研究策略创建引擎
    pub fn with_strategy(config: ResearchConfig, strategy: Box<dyn StrategyTrait>) -> Self {
        let max_iter = config.max_iterations;
        Self {
            config,
            strategy,
            progress: Arc::new(RwLock::new(ResearchProgress::new(
                ResearchPhase::Init,
                max_iter,
            ))),
        }
    }
}

#[async_trait]
impl rucora_core::research::DeepResearchEngine for DefaultResearchEngine {
    async fn research(
        &self,
        provider: &Arc<dyn LlmProvider>,
        topic: &str,
    ) -> Result<ResearchReport, rucora_core::research::ResearchError> {
        // 更新进度
        {
            let mut p = self.progress.write().unwrap();
            p.phase = ResearchPhase::Search;
            p.description = format!("开始研究: {topic}");
        }

        let mut context = ResearchContext::new(topic);
        context.set_phase(ResearchPhase::Search);

        // 使用策略执行搜索
        let result = self.strategy.search(provider, topic, &mut context).await;

        // 创建报告
        let mut report = ResearchReport::new(topic.to_string(), self.config.strategy);

        match result {
            Ok(strategy_result) => {
                report.set_tokens(strategy_result.tokens_used);
                report.summary = format!(
                    "研究完成，收集到 {} 条信息",
                    strategy_result.new_info.len()
                );
            }
            Err(e) => {
                report.summary = format!("研究失败: {e}");
            }
        }

        // 更新进度为完成
        {
            let mut p = self.progress.write().unwrap();
            p.phase = ResearchPhase::Complete;
            p.description = "研究完成".to_string();
        }

        Ok(report)
    }

    fn progress(&self) -> ResearchProgress {
        self.progress.read().unwrap().clone()
    }
}