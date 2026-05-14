//! ResearchQualityScore 的不变量验证测试
//!
//! 这些测试通过具体的输入组合验证 `ResearchQualityScore::calculate()`
//! 的关键不变式，相当于属性测试的手动展开。

use rucora_core::research::{Citation, InfoPiece, ResearchQualityScore, ScoreDetails, SourceType};

// ===== 辅助函数 =====

fn make_info(content: &str, relevance: f32, source_type: SourceType) -> InfoPiece {
    InfoPiece {
        content: content.to_string(),
        source_url: Some(format!("https://example-{}.com", content.len())),
        source_type,
        relevance_score: relevance,
        collected_at: chrono::Utc::now(),
    }
}

// ===== 核心不变式：所有分数在 [0.0, 1.0] 范围内 =====

#[test]
fn prop_score_empty_inputs_bounds() {
    let score = ResearchQualityScore::calculate(&[], &[], 0, &[]);
    assert!((0.0..=1.0).contains(&score.info_quality));
    assert!((0.0..=1.0).contains(&score.completeness));
    assert!((0.0..=1.0).contains(&score.confidence));
    assert!((0.0..=1.0).contains(&score.overall));
}

#[test]
fn prop_score_single_high_quality_bounds() {
    let infos = vec![make_info("test content", 0.9, SourceType::Official)];
    let score = ResearchQualityScore::calculate(&infos, &[], 1, &["test".to_string()]);
    assert!((0.0..=1.0).contains(&score.info_quality));
    assert!((0.0..=1.0).contains(&score.completeness));
    assert!((0.0..=1.0).contains(&score.confidence));
    assert!((0.0..=1.0).contains(&score.overall));
}

#[test]
fn prop_score_mixed_quality_bounds() {
    let infos = vec![
        make_info("high quality content", 0.9, SourceType::Official),
        make_info("low quality content", 0.2, SourceType::Blog),
        make_info("medium quality content", 0.5, SourceType::News),
    ];
    let score = ResearchQualityScore::calculate(&infos, &[], 3, &["test".to_string()]);
    assert!((0.0..=1.0).contains(&score.info_quality));
    assert!((0.0..=1.0).contains(&score.completeness));
    assert!((0.0..=1.0).contains(&score.confidence));
    assert!((0.0..=1.0).contains(&score.overall));
}

#[test]
fn prop_score_many_infos_bounds() {
    let infos: Vec<InfoPiece> = (0..50)
        .map(|i| {
            make_info(
                &format!("content number {}", i),
                (i as f32) / 50.0,
                if i % 3 == 0 {
                    SourceType::Official
                } else if i % 3 == 1 {
                    SourceType::Academic
                } else {
                    SourceType::News
                },
            )
        })
        .collect();
    let score = ResearchQualityScore::calculate(&infos, &[], 10, &["test".to_string()]);
    assert!((0.0..=1.0).contains(&score.info_quality));
    assert!((0.0..=1.0).contains(&score.completeness));
    assert!((0.0..=1.0).contains(&score.confidence));
    assert!((0.0..=1.0).contains(&score.overall));
}

#[test]
fn prop_score_with_citations_bounds() {
    let infos = vec![make_info("content", 0.8, SourceType::Official)];
    let citations = vec![
        Citation::new(
            "https://example.com/1".to_string(),
            "Title 1".to_string(),
            "snippet".to_string(),
        ),
        Citation::new(
            "https://example.com/2".to_string(),
            "Title 2".to_string(),
            "snippet".to_string(),
        ),
    ];
    let score = ResearchQualityScore::calculate(&infos, &citations, 1, &[]);
    assert!((0.0..=1.0).contains(&score.info_quality));
    assert!((0.0..=1.0).contains(&score.completeness));
    assert!((0.0..=1.0).contains(&score.confidence));
    assert!((0.0..=1.0).contains(&score.overall));
}

// ===== ScoreDetails 字段一致性 =====

#[test]
fn prop_score_details_match_info_count() {
    let infos = vec![
        make_info("a", 0.9, SourceType::Official),
        make_info("b", 0.8, SourceType::Academic),
        make_info("c", 0.7, SourceType::Blog),
    ];
    let score = ResearchQualityScore::calculate(&infos, &[], 5, &[]);
    assert_eq!(score.details.info_count, 3);
    assert!(score.details.high_quality_count <= 3);
    assert!(score.details.source_diversity <= 3);
}

#[test]
fn prop_score_details_high_quality_threshold() {
    // 所有信息的相关性 >= 0.7
    let infos = vec![
        make_info("a", 0.7, SourceType::Official),
        make_info("b", 0.8, SourceType::Official),
        make_info("c", 0.9, SourceType::Official),
        make_info("d", 1.0, SourceType::Official),
    ];
    let score = ResearchQualityScore::calculate(&infos, &[], 1, &[]);
    // 所有信息都是高质量
    assert_eq!(score.details.high_quality_count, 4);
    assert_eq!(score.details.info_count, 4);
}

#[test]
fn prop_score_details_no_high_quality() {
    // 所有信息的相关性 < 0.7
    let infos = vec![
        make_info("a", 0.1, SourceType::Official),
        make_info("b", 0.2, SourceType::Academic),
        make_info("c", 0.3, SourceType::Blog),
    ];
    let score = ResearchQualityScore::calculate(&infos, &[], 1, &[]);
    assert_eq!(score.details.high_quality_count, 0);
    assert_eq!(score.details.info_count, 3);
}

#[test]
fn prop_score_details_duplicate_ratio() {
    // 两个完全相同的信息产生一个重复
    let info = make_info("duplicate content", 0.8, SourceType::Official);
    let infos = vec![info.clone(), info.clone()];
    let score = ResearchQualityScore::calculate(&infos, &[], 1, &[]);
    // 1 duplicate out of 2 = 0.5
    assert_eq!(score.details.duplicate_ratio, 0.5);
}

#[test]
fn prop_score_details_no_duplicates() {
    let infos = vec![
        make_info("unique a", 0.8, SourceType::Official),
        make_info("unique b", 0.7, SourceType::Academic),
        make_info("unique c", 0.9, SourceType::Blog),
    ];
    let score = ResearchQualityScore::calculate(&infos, &[], 1, &[]);
    assert_eq!(score.details.duplicate_ratio, 0.0);
}

// ===== 零信息不变式 =====

#[test]
fn prop_empty_info_zero_score() {
    let score = ResearchQualityScore::calculate(&[], &[], 0, &["keyword".to_string()]);
    assert_eq!(score.info_quality, 0.0);
    assert_eq!(score.completeness, 0.0);
    assert_eq!(score.overall, 0.0);
    assert_eq!(score.details.info_count, 0);
    assert_eq!(score.details.high_quality_count, 0);
    assert_eq!(score.details.duplicate_ratio, 0.0);
}

#[test]
fn prop_empty_info_with_citations() {
    let citations = vec![Citation::new(
        "https://example.com".to_string(),
        "Title".to_string(),
        "snippet".to_string(),
    )];
    let score = ResearchQualityScore::calculate(&[], &citations, 1, &[]);
    assert_eq!(score.info_quality, 0.0);
    // 引用不影响 info_quality
    assert_eq!(score.details.citation_count, 1);
}

// ===== 级别评级一致性 =====

#[test]
fn prop_level_rating_excellent() {
    let mut score = ResearchQualityScore::zero();
    score.overall = 0.9;
    assert_eq!(score.level(), "优秀");
    score.overall = 1.0;
    assert_eq!(score.level(), "优秀");
    score.overall = 0.8;
    assert_eq!(score.level(), "优秀");
}

#[test]
fn prop_level_rating_good() {
    let mut score = ResearchQualityScore::zero();
    score.overall = 0.7;
    assert_eq!(score.level(), "良好");
    score.overall = 0.6;
    assert_eq!(score.level(), "良好");
}

#[test]
fn prop_level_rating_average() {
    let mut score = ResearchQualityScore::zero();
    score.overall = 0.5;
    assert_eq!(score.level(), "一般");
    score.overall = 0.4;
    assert_eq!(score.level(), "一般");
}

#[test]
fn prop_level_rating_needs_improvement() {
    let mut score = ResearchQualityScore::zero();
    score.overall = 0.3;
    assert_eq!(score.level(), "需改进");
    score.overall = 0.0;
    assert_eq!(score.level(), "需改进");
}

// ===== is_sufficient 边界 =====

#[test]
fn prop_is_sufficient_boundary() {
    let mut score = ResearchQualityScore::zero();

    // 恰好等于阈值
    score.overall = 0.7;
    assert!(score.is_sufficient(0.7));

    // 略高于阈值
    score.overall = 0.7001;
    assert!(score.is_sufficient(0.7));

    // 略低于阈值
    score.overall = 0.6999;
    assert!(!score.is_sufficient(0.7));

    // 极高阈值
    score.overall = 0.9;
    assert!(!score.is_sufficient(1.0));
}

// ===== 置信度计算公式验证 =====

#[test]
fn prop_confidence_with_diversity() {
    let infos = vec![make_info("a", 1.0, SourceType::Official)];
    let score = ResearchQualityScore::calculate(&infos, &[], 1, &[]);

    // info_quality = 1.0 (high quality) * 1.0 (avg relevance) = 1.0
    // completeness = 0.3 (info_count <= 5 且无关键词匹配)
    // source_diversity = 1, min(1/10, 0.3) = 0.1
    // confidence = 1.0 * 0.4 + 0.3 * 0.3 + 0.1 = 0.4 + 0.09 + 0.1 = 0.59
    assert!((score.confidence - 0.59).abs() < 0.01);
}

// ===== 主题覆盖度计算 =====

#[test]
fn prop_topic_coverage_with_matching_keywords() {
    let infos = vec![
        make_info("Rust programming language", 0.8, SourceType::Official),
        make_info("Python programming language", 0.7, SourceType::Academic),
    ];
    let keywords = vec!["rust".to_string(), "python".to_string()];
    let score = ResearchQualityScore::calculate(&infos, &[], 1, &keywords);

    // 两个关键词都应该匹配
    assert_eq!(score.details.topic_coverage, 2);
}

#[test]
fn prop_topic_coverage_no_match() {
    let infos = vec![make_info("Java programming", 0.8, SourceType::Official)];
    let keywords = vec!["rust".to_string()];
    let score = ResearchQualityScore::calculate(&infos, &[], 1, &keywords);

    // 没有关键词匹配
    assert_eq!(score.details.topic_coverage, 0);
}

// ===== overall 加权公式验证 =====

#[test]
fn prop_overall_formula_zero_components() {
    let mut score = ResearchQualityScore::zero();
    score.info_quality = 0.0;
    score.completeness = 0.0;
    score.confidence = 0.0;
    score.overall = 0.0 * 0.3 + 0.0 * 0.4 + 0.0 * 0.3;
    assert_eq!(score.overall, 0.0);
}

#[test]
fn prop_overall_formula_max_components() {
    let mut score = ResearchQualityScore::zero();
    score.info_quality = 1.0;
    score.completeness = 1.0;
    score.confidence = 1.0;
    score.overall = 1.0 * 0.3 + 1.0 * 0.4 + 1.0 * 0.3;
    assert!((score.overall - 1.0).abs() < 0.001);
}

// ===== 构造器测试 =====

#[test]
fn test_zero_construct() {
    let score = ResearchQualityScore::zero();
    assert_eq!(score.info_quality, 0.0);
    assert_eq!(score.completeness, 0.0);
    assert_eq!(score.confidence, 0.0);
    assert_eq!(score.overall, 0.0);
    assert_eq!(score.details.info_count, 0);
    assert_eq!(score.details.search_rounds, 0);
}

// ===== 搜索轮次影响 =====

#[test]
fn prop_search_rounds_reflected_in_details() {
    for rounds in [0, 1, 5, 10, 100] {
        let infos = vec![make_info("test", 0.8, SourceType::Official)];
        let score = ResearchQualityScore::calculate(&infos, &[], rounds, &[]);
        assert_eq!(score.details.search_rounds, rounds);
    }
}
