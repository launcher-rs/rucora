//! 深度研究集成测试
//!
//! 测试 `rucora::deep_research` 模块的协同工作，包括：
//! - DefaultResearchEngine 引擎基础功能
//! - StandardStrategy 标准策略
//! - FastStrategy 快速策略
//! - AgenticStrategy Agentic 策略
//! - InMemoryResearchLibrary 内存研究库
//! - ResearchQualityAssessor 与策略的集成

use std::sync::Arc;

use rucora_core::provider::LlmProvider;
use rucora_core::{
    DeepResearchEngine, ResearchContext, ResearchLibrary, ResearchReport,
    StrategyTrait,
};

// ===== 工具 Mock Provider =====

struct TestProvider;

#[async_trait::async_trait]
impl LlmProvider for TestProvider {
    async fn chat(
        &self,
        _request: rucora_core::provider::types::ChatRequest,
    ) -> Result<rucora_core::provider::types::ChatResponse, rucora_core::error::ProviderError>
    {
        Ok(rucora_core::provider::types::ChatResponse {
            message: rucora_core::provider::types::ChatMessage {
                role: rucora_core::provider::types::Role::Assistant,
                content: "test response".to_string(),
                name: None,
            },
            tool_calls: vec![],
            usage: None,
            finish_reason: None,
        })
    }
}

fn test_provider() -> Arc<dyn LlmProvider> {
    Arc::new(TestProvider) as Arc<dyn LlmProvider>
}

// ===== DefaultResearchEngine 测试 =====

#[test]
fn test_default_research_engine_creation() {
    let engine = rucora::deep_research::DefaultResearchEngine::with_config(
        rucora_core::research::ResearchConfig::default(),
    );
    let progress = engine.progress();
    assert_eq!(progress.phase, rucora_core::research::ResearchPhase::Init);
    assert_eq!(progress.current_step, 0);
}

#[test]
fn test_default_research_engine_new() {
    let strategy: Box<dyn StrategyTrait> =
        Box::new(rucora::deep_research::StandardStrategy::new());
    let engine = rucora::deep_research::DefaultResearchEngine::new(strategy);
    let progress = engine.progress();
    assert_eq!(progress.phase, rucora_core::research::ResearchPhase::Init);
}

#[test]
fn test_default_research_engine_with_strategy() {
    let config = rucora_core::research::ResearchConfig::fast();
    let strategy: Box<dyn StrategyTrait> =
        Box::new(rucora::deep_research::FastStrategy::new());
    let engine =
        rucora::deep_research::DefaultResearchEngine::with_strategy(config.clone(), strategy);
    let progress = engine.progress();
    assert_eq!(progress.total_steps, config.max_iterations);
}

// ===== StandardStrategy 测试 =====

#[test]
fn test_standard_strategy_name() {
    let strategy = rucora::deep_research::StandardStrategy::new();
    assert_eq!(strategy.name(), "standard");
}

#[test]
fn test_standard_strategy_description() {
    let strategy = rucora::deep_research::StandardStrategy::new();
    assert!(!strategy.description().is_empty());
}

#[test]
fn test_standard_strategy_max_iterations() {
    let strategy = rucora::deep_research::StandardStrategy::new();
    assert_eq!(strategy.max_iterations(), Some(10));
}

#[test]
fn test_standard_strategy_config() {
    let strategy = rucora::deep_research::StandardStrategy::new();
    assert_eq!(strategy.config().max_iterations, 10);
}

#[test]
fn test_standard_strategy_with_config() {
    let config = rucora_core::research::ResearchConfig::fast();
    let strategy = rucora::deep_research::StandardStrategy::with_config(config.clone());
    assert_eq!(strategy.config().max_iterations, 3);
}

#[tokio::test]
async fn test_standard_strategy_search() {
    let strategy = rucora::deep_research::StandardStrategy::new();
    let provider = test_provider();
    let mut context = ResearchContext::new("test topic");

    let result = strategy
        .search(&provider, "test topic", &mut context)
        .await
        .expect("search should succeed");

    assert!(result.is_complete);
    assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
}

#[tokio::test]
async fn test_standard_strategy_info_collection() {
    let strategy = rucora::deep_research::StandardStrategy::new();
    let provider = test_provider();
    let mut context = ResearchContext::new("test topic");

    let _result = strategy
        .search(&provider, "test topic", &mut context)
        .await
        .expect("search should succeed");

    // 信息应该被收集到上下文中
    assert!(!context.collected_info.is_empty());
}

// ===== FastStrategy 测试 =====

#[test]
fn test_fast_strategy_name() {
    let strategy = rucora::deep_research::FastStrategy::new();
    assert_eq!(strategy.name(), "fast");
}

#[test]
fn test_fast_strategy_max_iterations() {
    let strategy = rucora::deep_research::FastStrategy::new();
    assert_eq!(strategy.max_iterations(), Some(3));
}

#[test]
fn test_fast_strategy_defaults() {
    let strategy = rucora::deep_research::FastStrategy::new();
    assert_eq!(strategy.config().max_iterations, 3);
}

#[tokio::test]
async fn test_fast_strategy_search() {
    let strategy = rucora::deep_research::FastStrategy::new();
    let provider = test_provider();
    let mut context = ResearchContext::new("fast test");

    let result = strategy
        .search(&provider, "fast test", &mut context)
        .await
        .expect("search should succeed");

    assert!(result.is_complete);
    assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
}

// ===== AgenticStrategy 测试 =====

#[test]
fn test_agentic_strategy_name() {
    let strategy = rucora::deep_research::AgenticStrategy::new();
    assert_eq!(strategy.name(), "agentic");
}

#[test]
fn test_agentic_strategy_max_iterations() {
    let strategy = rucora::deep_research::AgenticStrategy::new();
    assert_eq!(strategy.max_iterations(), Some(20));
}

#[test]
fn test_agentic_strategy_defaults() {
    let strategy = rucora::deep_research::AgenticStrategy::new();
    assert_eq!(strategy.config().max_iterations, 20);
}

#[test]
fn test_agentic_strategy_with_config() {
    let config = rucora_core::research::ResearchConfig::fast();
    let strategy = rucora::deep_research::AgenticStrategy::with_config(&config);
    assert_eq!(strategy.config().max_iterations, 3);
}

#[test]
fn test_agentic_strategy_should_continue() {
    use rucora_core::research::StrategyResult;

    let strategy = rucora::deep_research::AgenticStrategy::new();

    let result = StrategyResult::default();
    assert!(strategy.should_continue(&result));

    let complete_result = StrategyResult::complete();
    assert!(!strategy.should_continue(&complete_result));
}

#[tokio::test]
async fn test_agentic_strategy_search() {
    let strategy = rucora::deep_research::AgenticStrategy::new();
    let provider = test_provider();
    let mut context = ResearchContext::new("agentic test");

    let result = strategy
        .search(&provider, "agentic test", &mut context)
        .await
        .expect("search should succeed");

    assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
}

// ===== ResearchEngine 集成测试 =====

#[tokio::test]
async fn test_research_engine_fast_config() {
    let engine = rucora::deep_research::DefaultResearchEngine::with_config(
        rucora_core::research::ResearchConfig::fast(),
    );
    let provider = test_provider();

    let report = engine
        .research(&provider, "fast research topic")
        .await
        .expect("research should succeed");

    assert_eq!(report.strategy, rucora_core::research::ResearchStrategy::Fast);
    assert!(!report.topic.is_empty());
    assert!(report.created_at <= report.updated_at);
}

#[tokio::test]
async fn test_research_engine_standard_config() {
    let engine = rucora::deep_research::DefaultResearchEngine::with_config(
        rucora_core::research::ResearchConfig::default(),
    );
    let provider = test_provider();

    let report = engine
        .research(&provider, "standard research topic")
        .await
        .expect("research should succeed");

    assert_eq!(report.strategy, rucora_core::research::ResearchStrategy::Standard);
}

// ===== QualityAssessor 集成测试 =====

#[test]
fn test_quality_score_calculate_bounds() {
    let info_pieces = vec![];
    let citations = vec![];
    let keywords = vec!["test".to_string()];

    let score = rucora_core::research::ResearchQualityScore::calculate(
        &info_pieces,
        &citations,
        0,
        &keywords,
    );

    assert!((0.0..=1.0).contains(&score.info_quality));
    assert!((0.0..=1.0).contains(&score.completeness));
    assert!((0.0..=1.0).contains(&score.confidence));
    assert!((0.0..=1.0).contains(&score.overall));
}

#[test]
fn test_quality_assessor_suggestions() {
    use rucora::deep_research::ResearchQualityAssessor;
    use rucora_core::research::SuggestionType;

    let assessor = ResearchQualityAssessor::with_default("test topic");
    let empty_score = rucora_core::research::ResearchQualityScore::zero();
    let suggestion = assessor.suggest(&empty_score);
    assert!(
        matches!(suggestion.suggestion_type, SuggestionType::NeedMoreInfo),
        "empty score should suggest more info, got {:?}",
        suggestion.suggestion_type
    );
}

// ===== ResearchLibrary 测试 =====

#[tokio::test]
async fn test_in_memory_research_library() {
    use rucora::deep_research::InMemoryResearchLibrary;

    let lib = InMemoryResearchLibrary::new();
    let report = ResearchReport::new(
        "test".to_string(),
        rucora_core::research::ResearchStrategy::Fast,
    );

    let id = lib.save(&report).await.expect("save should succeed");
    assert!(!id.is_empty());

    let found = lib
        .get(&id)
        .await
        .expect("get should succeed")
        .expect("report should be found");
    assert_eq!(found.id, report.id);
    assert_eq!(found.topic, "test");
}

#[tokio::test]
async fn test_in_memory_research_library_search() {
    use rucora::deep_research::InMemoryResearchLibrary;

    let lib = InMemoryResearchLibrary::new();

    let mut report1 = ResearchReport::new(
        "rust programming".to_string(),
        rucora_core::research::ResearchStrategy::Fast,
    );
    report1.summary = "关于 Rust 的研究".to_string();
    lib.save(&report1).await.expect("save should succeed");

    let mut report2 = ResearchReport::new(
        "python programming".to_string(),
        rucora_core::research::ResearchStrategy::Fast,
    );
    report2.summary = "关于 Python 的研究".to_string();
    lib.save(&report2).await.expect("save should succeed");

    let results = lib.search("rust").await.expect("search should succeed");
    assert_eq!(results.len(), 1);
    assert!(results[0].topic.contains("rust"));
}

#[tokio::test]
async fn test_in_memory_research_library_list() {
    use rucora::deep_research::InMemoryResearchLibrary;

    let lib = InMemoryResearchLibrary::new();

    for i in 0..5 {
        let report = ResearchReport::new(
            format!("topic_{}", i),
            rucora_core::research::ResearchStrategy::Fast,
        );
        lib.save(&report).await.expect("save should succeed");
    }

    let results = lib.list(3).await.expect("list should succeed");
    assert_eq!(results.len(), 3);
}

#[tokio::test]
async fn test_in_memory_research_library_delete() {
    use rucora::deep_research::InMemoryResearchLibrary;

    let lib = InMemoryResearchLibrary::new();
    let report = ResearchReport::new(
        "test".to_string(),
        rucora_core::research::ResearchStrategy::Fast,
    );

    let id = lib.save(&report).await.expect("save should succeed");
    lib.delete(&id).await.expect("delete should succeed");

    let found = lib.get(&id).await.expect("get should succeed");
    assert!(found.is_none(), "report should be deleted");
}

// ===== Report 生成测试 =====

#[test]
fn test_report_markdown_generation() {
    let mut report = ResearchReport::new(
        "test topic".to_string(),
        rucora_core::research::ResearchStrategy::Standard,
    );
    report.summary = "这是一个摘要".to_string();
    report.full_content = "这是完整内容".to_string();

    let markdown = report.to_markdown();
    assert!(markdown.contains("#"));
    assert!(markdown.contains("test topic"));
    assert!(markdown.contains("这是一个摘要"));
    assert!(markdown.contains("这是完整内容"));
}

#[test]
fn test_report_add_citations() {
    let mut report = ResearchReport::new(
        "test".to_string(),
        rucora_core::research::ResearchStrategy::Fast,
    );

    let citation1 = rucora_core::research::Citation::new(
        "https://example.com/1".to_string(),
        "Title 1".to_string(),
        "Snippet 1".to_string(),
    );
    let citation2 = rucora_core::research::Citation::new(
        "https://example.com/2".to_string(),
        "Title 2".to_string(),
        "Snippet 2".to_string(),
    );

    report.add_citations(vec![citation1.clone(), citation2.clone()]);
    assert_eq!(report.citations.len(), 2);
}

// ===== ResearchConfig 变体测试 =====

#[test]
fn test_research_config_academic() {
    let config = rucora_core::research::ResearchConfig::academic();
    assert_eq!(config.max_iterations, 15);
    assert!(config.include_citations);
}

#[test]
fn test_research_config_agentic() {
    let config = rucora_core::research::ResearchConfig::agentic();
    assert_eq!(config.max_iterations, 20);
    assert_eq!(config.max_concurrent_searches, 5);
}

// ===== ResearchPhase 显示测试 =====

#[test]
fn test_research_phase_display() {
    use rucora_core::research::ResearchPhase;

    assert_eq!(format!("{}", ResearchPhase::Init), "初始化");
    assert_eq!(format!("{}", ResearchPhase::Search), "搜索");
    assert_eq!(format!("{}", ResearchPhase::Complete), "完成");
}

// ===== ResearchStrategy 显示测试 =====

#[test]
fn test_research_strategy_display() {
    use rucora_core::research::ResearchStrategy;

    assert_eq!(format!("{}", ResearchStrategy::Fast), "快速模式");
    assert_eq!(format!("{}", ResearchStrategy::Standard), "标准模式");
    assert_eq!(format!("{}", ResearchStrategy::Agentic), "自主模式");
    assert_eq!(format!("{}", ResearchStrategy::Academic), "学术模式");
}

// ===== ScoreDetails 测试 =====

#[test]
fn test_score_details_default() {
    use rucora_core::research::ScoreDetails;

    let details = ScoreDetails::default();
    assert_eq!(details.info_count, 0);
    assert_eq!(details.high_quality_count, 0);
    assert_eq!(details.source_diversity, 0);
    assert_eq!(details.citation_count, 0);
    assert_eq!(details.search_rounds, 0);
    assert_eq!(details.topic_coverage, 0);
    assert_eq!(details.duplicate_ratio, 0.0);
}

// ===== Citation 构造测试 =====

#[test]
fn test_citation_creation() {
    use rucora_core::research::{Citation, SourceType};

    let citation = Citation::new(
        "https://example.com".to_string(),
        "Test Title".to_string(),
        "test snippet".to_string(),
    );

    assert_eq!(citation.url, "https://example.com");
    assert_eq!(citation.title, "Test Title");
    assert_eq!(citation.snippet, "test snippet");
    assert_eq!(citation.source_type, SourceType::Unknown);
}

// ===== InfoPiece 构造测试 =====

#[test]
fn test_info_piece_creation() {
    use rucora_core::research::{InfoPiece, SourceType};

    let info = InfoPiece::new(
        "content".to_string(),
        Some("https://example.com".to_string()),
        SourceType::News,
    );

    assert_eq!(info.content, "content");
    assert_eq!(info.source_url, Some("https://example.com".to_string()));
    assert_eq!(info.source_type, SourceType::News);
    assert_eq!(info.relevance_score, 0.5);
}