//! 研究质量评分示例
//!
//! 展示如何使用评分系统评估研究质量并生成改进建议。

use rucora_core::research::{
    InfoPiece, Citation, ResearchQualityAssessor, ScoringConfig, SourceType,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    println!("=== 研究质量评分系统示例 ===\n");

    // 示例 1: 基本评分
    println!("【示例 1】基本评分");
    basic_scoring_demo().await?;

    // 示例 2: 改进建议生成
    println!("\n【示例 2】改进建议生成");
    suggestion_demo().await?;

    // 示例 3: 自定义配置
    println!("\n【示例 3】自定义评分配置");
    custom_config_demo().await?;

    // 示例 4: 动态评估循环
    println!("\n【示例 4】动态评估循环");
    dynamic_evaluation_demo().await?;

    println!("\n=== 示例完成 ===");
    Ok(())
}

/// 基本评分演示
async fn basic_scoring_demo() -> anyhow::Result<()> {
    // 创建评估器
    let assessor = ResearchQualityAssessor::with_default("Rust 异步编程");

    // 模拟收集的信息
    let info_pieces = vec![
        InfoPiece::new(
            "Rust 的 async/await 语法提供了简洁的异步编程方式".to_string(),
            Some("https://doc.rust-lang.org".to_string()),
            SourceType::Official,
        ),
        InfoPiece::new(
            "Tokio 是 Rust 最流行的异步运行时".to_string(),
            Some("https://tokio.rs".to_string()),
            SourceType::Official,
        ),
        InfoPiece::new(
            "async/await 基于 futures 特性实现".to_string(),
            None,
            SourceType::Blog,
        ),
        InfoPiece::new(
            "Rust 异步编程需要了解 Future trait".to_string(),
            None,
            SourceType::Blog,
        ),
    ];

    // 模拟引用
    let citations = vec![
        Citation::new(
            "https://doc.rust-lang.org".to_string(),
            "Rust 官方文档".to_string(),
            "Rust 官方 async 文档".to_string(),
        ),
        Citation::new(
            "https://tokio.rs".to_string(),
            "Tokio 文档".to_string(),
            "Tokio 官方文档".to_string(),
        ),
    ];

    // 进行评分
    let score = assessor.assess(&info_pieces, &citations, 2);

    println!("  主题: Rust 异步编程");
    println!("  信息数量: {}", score.details.info_count);
    println!("  高质量信息: {}", score.details.high_quality_count);
    println!("  来源多样性: {}", score.details.source_diversity);
    println!("  引用数量: {}", score.details.citation_count);
    println!("  搜索轮次: {}", score.details.search_rounds);
    println!("  重复率: {:.1}%", score.details.duplicate_ratio * 100.0);
    println!();
    println!("  --- 评分结果 ---");
    println!("  信息质量: {:.2}", score.info_quality);
    println!("  完整性: {:.2}", score.completeness);
    println!("  置信度: {:.2}", score.confidence);
    println!("  综合评分: {:.2} ({})", score.overall, score.level());

    Ok(())
}

/// 改进建议生成演示
async fn suggestion_demo() -> anyhow::Result<()> {
    let assessor = ResearchQualityAssessor::with_default("机器学习");

    // 场景 1: 信息不足
    println!("  场景 1: 信息不足");
    let info_pieces = vec![
        InfoPiece::new("机器学习是人工智能的子领域".to_string(), None, SourceType::Unknown),
    ];
    let score = assessor.assess(&info_pieces, &[], 1);
    let suggestion = assessor.suggest(&score);
    println!("    评分: {:.2}, 建议: {}", score.overall, suggestion.description);

    // 场景 2: 来源单一
    println!("  场景 2: 来源单一");
    let info_pieces = vec![
        InfoPiece::new("ML 内容 A".to_string(), None, SourceType::Blog),
        InfoPiece::new("ML 内容 B".to_string(), None, SourceType::Blog),
        InfoPiece::new("ML 内容 C".to_string(), None, SourceType::Blog),
    ];
    let score = assessor.assess(&info_pieces, &[], 1);
    let suggestion = assessor.suggest(&score);
    println!("    评分: {:.2}, 建议: {}", score.overall, suggestion.description);

    // 场景 3: 重复信息过多
    println!("  场景 3: 重复信息过多");
    let info_pieces = vec![
        InfoPiece::new("机器学习是 AI 的子领域".to_string(), None, SourceType::Blog),
        InfoPiece::new("机器学习是 AI 的子领域".to_string(), None, SourceType::Blog),
        InfoPiece::new("机器学习是 AI 的子领域".to_string(), None, SourceType::Blog),
    ];
    let score = assessor.assess(&info_pieces, &[], 1);
    let suggestion = assessor.suggest(&score);
    println!("    评分: {:.2}, 建议: {}", score.overall, suggestion.description);

    // 场景 4: 质量优秀
    println!("  场景 4: 质量优秀");
    let info_pieces = vec![
        InfoPiece::new("ML 内容 1".to_string(), None, SourceType::Official),
        InfoPiece::new("ML 内容 2".to_string(), None, SourceType::Academic),
        InfoPiece::new("ML 内容 3".to_string(), None, SourceType::Blog),
        InfoPiece::new("ML 内容 4".to_string(), None, SourceType::News),
        InfoPiece::new("ML 内容 5".to_string(), None, SourceType::Blog),
    ];
    let score = assessor.assess(&info_pieces, &[], 3);
    let suggestion = assessor.suggest(&score);
    println!("    评分: {:.2}, 建议: {}", score.overall, suggestion.description);

    Ok(())
}

/// 自定义配置演示
async fn custom_config_demo() -> anyhow::Result<()> {
    // 创建自定义配置
    let config = ScoringConfig {
        quality_threshold: 0.8,      // 提高质量阈值
        confidence_threshold: 0.9,    // 提高置信度阈值
        duplicate_threshold: 0.2,     // 更严格控制重复率
        min_info_count: 10,          // 要求更多信息
        min_source_diversity: 3,     // 要求更多来源
    };

    let assessor = ResearchQualityAssessor::new(config, vec!["深度学习".to_string()]);

    let info_pieces = vec![
        InfoPiece::new("深度学习内容".to_string(), None, SourceType::Official),
        InfoPiece::new("深度学习内容".to_string(), None, SourceType::Official),
    ];
    let score = assessor.assess(&info_pieces, &[], 1);
    let suggestion = assessor.suggest(&score);

    println!("  使用自定义配置:");
    println!("    质量阈值: 0.8");
    println!("    综合评分: {:.2}", score.overall);
    println!("    建议: {}", suggestion.description);
    println!("    优先级: {}", suggestion.priority);

    Ok(())
}

/// 动态评估循环演示
async fn dynamic_evaluation_demo() -> anyhow::Result<()> {
    let topic = "Python Web 框架";
    let assessor = ResearchQualityAssessor::with_default(topic);
    let max_rounds = 5;

    let mut all_info = Vec::new();

    println!("  模拟多轮研究过程:");

    for round in 1..=max_rounds {
        // 模拟每轮收集新信息
        let new_info = InfoPiece::new(
            format!("{} 相关研究信息 #{}", topic, round),
            Some("https://example.com".to_string()),
            match round {
                1 => SourceType::Official,
                2 => SourceType::Academic,
                3 => SourceType::Blog,
                _ => SourceType::News,
            },
        );
        all_info.push(new_info);

        // 评估当前质量
        let score = assessor.assess(&all_info, &[], round as usize);
        let suggestion = assessor.suggest(&score);

        println!(
            "    第 {} 轮: 评分={:.2} ({}) | {}",
            round,
            score.overall,
            score.level(),
            suggestion.description
        );

        // 检查是否应该继续
        if !assessor.should_continue(&score, round, max_rounds) {
            println!("    → 研究已达到目标，停止搜索");
            break;
        }

        // 获取下一轮搜索提示
        let next_hint = assessor.get_next_search_hint(&score, topic);
        println!("    → 下一轮搜索提示: {}", next_hint);
    }

    // 最终评分
    let final_score = assessor.assess(&all_info, &[], max_rounds as usize);
    println!("\n  最终结果: 综合评分={:.2} ({})", final_score.overall, final_score.level());
    println!("    信息数量: {}", final_score.details.info_count);
    println!("    来源多样性: {}", final_score.details.source_diversity);

    Ok(())
}