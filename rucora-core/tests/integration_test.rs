//! 研究模块集成测试
//!
//! 测试深度研究核心模块的协同工作，包括：
//! - ResearchContext 状态管理
//! - StrategyResult 构建
//! - ResearchQualityAssessor 评分逻辑
//! - DefaultCitationHandler 引用处理
//! - ResearchReport 构建

use rucora_core::research::{
    CitationHandler, InfoPiece, ResearchConfig, ResearchContext, ResearchQualityAssessor,
    ResearchReport, ResearchSuggestion, SourceType, StrategyResult, SuggestionType,
};

// ===== ResearchContext 测试 =====

#[test]
fn test_research_context_initialization() {
    let ctx = ResearchContext::new("test topic");
    assert_eq!(ctx.topic, "test topic");
    assert!(ctx.collected_info.is_empty());
    assert!(ctx.visited_urls.is_empty());
    assert!(ctx.citations.is_empty());
}

#[test]
fn test_research_context_add_info() {
    let mut ctx = ResearchContext::new("test");
    let info = InfoPiece::new("content".to_string(), None, SourceType::News);

    ctx.add_info(info.clone());
    assert_eq!(ctx.collected_info.len(), 1);
    assert_eq!(ctx.collected_info[0].content, "content");
    assert_eq!(ctx.total_content_length(), 7);
}

#[test]
fn test_research_context_visit_url() {
    let mut ctx = ResearchContext::new("test");
    ctx.visit_url("https://example.com".to_string());
    assert!(ctx.has_visited("https://example.com"));
    assert!(!ctx.has_visited("https://other.com"));

    // 重复访问不应重复添加
    ctx.visit_url("https://example.com".to_string());
    assert_eq!(ctx.visited_urls.len(), 1);
}

#[test]
fn test_research_context_add_citation() {
    let mut ctx = ResearchContext::new("test");
    let citation = rucora_core::research::Citation::new(
        "https://example.com".to_string(),
        "Test Title".to_string(),
        "test snippet".to_string(),
    );
    ctx.add_citation(citation.clone());
    assert_eq!(ctx.citations.len(), 1);

    // 重复引用不应添加
    ctx.add_citation(citation);
    assert_eq!(ctx.citations.len(), 1);
}

#[test]
fn test_research_context_state_management() {
    let mut ctx = ResearchContext::new("test");
    ctx.set_state("key1", serde_json::json!("value1"));
    ctx.set_state("key2", serde_json::json!(42));

    assert_eq!(ctx.get_state("key1"), Some(&serde_json::json!("value1")));
    assert_eq!(ctx.get_state("key2"), Some(&serde_json::json!(42)));
    assert_eq!(ctx.get_state("nonexistent"), None);
}

// ===== StrategyResult 测试 =====

#[test]
fn test_strategy_result_default() {
    let result = StrategyResult::default();
    assert!(!result.is_complete);
    assert!(result.new_info.is_empty());
    assert!(result.discovered_urls.is_empty());
    assert_eq!(result.confidence, 0.0);
}

#[test]
fn test_strategy_result_complete() {
    let result = StrategyResult::complete();
    assert!(result.is_complete);
    assert_eq!(result.confidence, 0.0);
}

#[test]
fn test_strategy_result_complete_with() {
    let result = StrategyResult::complete()
        .with_confidence(0.85)
        .with_tokens(100)
        .with_info(vec![]);
    assert!(result.is_complete);
    assert_eq!(result.confidence, 0.85);
    assert_eq!(result.tokens_used, 100);
}

#[test]
fn test_strategy_result_builder_methods() {
    let info = InfoPiece::new("test".to_string(), None, SourceType::News);
    let result = StrategyResult::default()
        .with_info(vec![info.clone()])
        .with_confidence(0.7);

    assert_eq!(result.new_info.len(), 1);
    assert_eq!(result.confidence, 0.7);
}

// ===== ResearchQualityAssessor 测试 =====

#[test]
fn test_quality_assessor_creation() {
    let assessor = ResearchQualityAssessor::with_default("test topic");
    assert!(assessor.config.min_info_count > 0);
    assert!(assessor.config.confidence_threshold > 0.0);
    assert!(!assessor.topic_keywords.is_empty());
}

#[test]
fn test_quality_assessor_empty_info() {
    let assessor = ResearchQualityAssessor::with_default("test topic");
    let info = vec![];
    let citations = vec![];
    let score = assessor.assess(&info, &citations, 1);

    assert_eq!(score.info_quality, 0.0);
    assert!(score.overall < 0.5);
    assert_eq!(score.details.info_count, 0);
}

#[test]
fn test_quality_score_level_rating() {
    let mut score = rucora_core::research::ResearchQualityScore::zero();

    score.overall = 0.9;
    assert_eq!(score.level(), "优秀");

    score.overall = 0.7;
    assert_eq!(score.level(), "良好");

    score.overall = 0.5;
    assert_eq!(score.level(), "一般");

    score.overall = 0.2;
    assert_eq!(score.level(), "需改进");
}

#[test]
fn test_quality_score_is_sufficient() {
    let mut score = rucora_core::research::ResearchQualityScore::zero();
    score.overall = 0.8;
    assert!(score.is_sufficient(0.7));
    assert!(!score.is_sufficient(0.9));
}

// ===== SuggestionType 测试 =====

#[test]
fn test_suggestion_creation() {
    let sufficient = ResearchSuggestion::sufficient();
    assert!(matches!(
        sufficient.suggestion_type,
        SuggestionType::Sufficient
    ));
    assert_eq!(sufficient.priority, 5);

    let need_info = ResearchSuggestion::need_more_info(2, 5);
    assert!(matches!(
        need_info.suggestion_type,
        SuggestionType::NeedMoreInfo
    ));
    assert!(need_info.description.contains("2/5"));

    let need_sources = ResearchSuggestion::need_more_sources(1usize);
    assert!(matches!(
        need_sources.suggestion_type,
        SuggestionType::NeedMoreSources
    ));

    let need_angle = ResearchSuggestion::need_new_angle(0.8);
    assert!(matches!(
        need_angle.suggestion_type,
        SuggestionType::NeedNewAngle
    ));

    let need_validation = ResearchSuggestion::need_validation(0.5);
    assert!(matches!(
        need_validation.suggestion_type,
        SuggestionType::NeedValidation
    ));
}

// ===== ResearchConfig 测试 =====

#[test]
fn test_research_config_variants() {
    let default = ResearchConfig::default();
    assert_eq!(default.max_iterations, 10);

    let fast = ResearchConfig::fast();
    assert_eq!(fast.max_iterations, 3);
    assert!(fast.max_output_length < default.max_output_length);

    let agentic = ResearchConfig::agentic();
    assert_eq!(agentic.max_iterations, 20);
    assert!(agentic.max_output_length > default.max_output_length);

    let academic = ResearchConfig::academic();
    assert_eq!(academic.max_iterations, 15);
}

// ===== CitationHandler 测试 =====

#[test]
fn test_default_citation_handler_extract() {
    use rucora_core::research::DefaultCitationHandler;

    let handler = DefaultCitationHandler::new();
    let content = "访问 https://example.com 和 http://test.org 获取更多信息";

    let citations = handler.extract_citations(content);
    assert_eq!(citations.len(), 2);
    assert_eq!(citations[0].url, "https://example.com");
    assert_eq!(citations[1].url, "http://test.org");
}

#[test]
fn test_default_citation_handler_no_duplicates() {
    use rucora_core::research::DefaultCitationHandler;

    let handler = DefaultCitationHandler::new();
    let content = "见 https://example.com 和 https://example.com";

    let citations = handler.extract_citations(content);
    assert_eq!(citations.len(), 1);
}

#[test]
fn test_citation_format_apa() {
    let mut citation = rucora_core::research::Citation::new(
        "https://example.com".to_string(),
        "Rust Programming".to_string(),
        "A great language".to_string(),
    );
    citation.source_type = SourceType::Official;

    let apa = citation.format_apa();
    assert!(apa.contains("Rust Programming"));
    assert!(apa.contains("https://example.com"));
}

// ===== ResearchReport 测试 =====

#[test]
fn test_research_report_creation() {
    use rucora_core::research::ResearchStrategy;

    let report = ResearchReport::new("test topic".to_string(), ResearchStrategy::Standard);
    assert_eq!(report.topic, "test topic");
    assert_eq!(report.citations.len(), 0);
    assert!(report.summary.is_empty());
    assert!(report.full_content.is_empty());
}

#[test]
fn test_research_report_builder_methods() {
    use rucora_core::research::{Citation, ResearchStrategy};

    let mut report = ResearchReport::new("test".to_string(), ResearchStrategy::Fast);

    report.add_citation(Citation::new(
        "https://example.com".to_string(),
        "Title".to_string(),
        "snippet".to_string(),
    ));
    assert_eq!(report.citations.len(), 1);

    let report = report.with_summary("summary".to_string());
    assert_eq!(report.summary, "summary");

    let report = report.with_content("content".to_string());
    assert_eq!(report.full_content, "content");
}
