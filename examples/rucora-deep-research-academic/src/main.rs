//! rucora 学术研究示例
//!
//! 专注于学术论文搜索和引用格式。

use anyhow::Result;
use rucora::provider::OpenAiProvider;
use rucora_core::provider::LlmProvider;
use rucora_core::research::{Citation, ResearchReport, ResearchStrategy};
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

const DEFAULT_TOPIC: &str = "深度学习在医学影像诊断中的应用";

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .compact()
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    print_banner();

    let provider = create_provider()?;
    let topic = DEFAULT_TOPIC;

    println!("{}", console::style("━━━ 研究主题 ━━━").green().bold());
    println!("  {}", console::style(topic).cyan().bold());

    println!(
        "\n{}",
        console::style("━━━ 执行学术研究 ━━━").blue().bold()
    );
    info!("开始学术研究: {}", topic);

    let report = run_academic_research(topic).await;

    println!("\n{}", console::style("━━━ 研究报告 ━━━").green().bold());
    println!("{}", report.to_markdown());

    Ok(())
}

fn print_banner() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║          rucora 学术研究示例 v1.0                        ║");
    println!("║  专注学术论文搜索和引用                                   ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");
}

fn create_provider() -> Result<Arc<dyn LlmProvider + Send + Sync>> {
    let api_key = std::env::var("OPENAI_API_KEY")
        .or_else(|_| std::env::var("API_KEY"))
        .expect("请设置 OPENAI_API_KEY 或 API_KEY 环境变量");

    let model = std::env::var("OPENAI_DEFAULT_MODEL")
        .or_else(|_| std::env::var("MODEL"))
        .unwrap_or_else(|_| "gpt-4o-mini".to_string());

    let base_url = std::env::var("OPENAI_BASE_URL")
        .or_else(|_| std::env::var("BASE_URL"))
        .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());

    Ok(Arc::new(
        OpenAiProvider::new(&base_url, &api_key).with_default_model(&model),
    ))
}

/// 模拟学术研究（实际需要使用 Arxiv/PubMed 工具）
async fn run_academic_research(topic: &str) -> ResearchReport {
    let mut report = ResearchReport::new(topic.to_string(), ResearchStrategy::Academic);

    // 模拟学术引用
    let citations = vec![
        Citation::new(
            "https://arxiv.org/abs/2103.00001".to_string(),
            "Deep Learning for Medical Image Analysis: A Survey".to_string(),
            "深度学习在医学影像分析中的综述".to_string(),
        )
        .with_source_type(rucora_core::research::SourceType::Academic),
        Citation::new(
            "https://pubmed.ncbi.nlm.nih.gov/12345678/".to_string(),
            "Transformer-based Methods for Medical Image Classification".to_string(),
            "基于 Transformer 的医学图像分类方法".to_string(),
        )
        .with_source_type(rucora_core::research::SourceType::Academic),
        Citation::new(
            "https://www.semanticscholar.org/paper/123456".to_string(),
            "Attention Mechanisms in Medical Imaging".to_string(),
            "医学影像中的注意力机制".to_string(),
        )
        .with_source_type(rucora_core::research::SourceType::Academic),
    ];

    for c in citations {
        report.add_citation(c);
    }

    // 模拟研究内容
    let content = format!(
        r#"# {} - 学术研究报告

## 摘要

本报告综述了深度学习技术在医学影像诊断领域的最新研究进展。

## 1. 引言

医学影像诊断是临床诊断的重要组成部分。传统的影像诊断依赖经验丰富的放射科医生，
但随着医学影像数据量的急剧增长，人工诊断面临着越来越大的挑战。

## 2. 研究方法

本研究采用系统性综述方法，主要检索了 arXiv、PubMed、Semantic Scholar 等学术数据库。

## 3. 主要发现

### 3.1 深度学习模型

- CNN（卷积神经网络）在医学影像分类中表现优异
- Transformer 模型在图像分割任务中展现出强大能力
- 注意力机制帮助模型聚焦于关键区域

### 3.2 应用场景

- 肺部 CT 扫描的肺癌检测
- 视网膜图像的糖尿病视网膜病变筛查
- 乳腺 X 光片的乳腺癌检测

## 4. 结论

深度学习技术在医学影像诊断领域具有巨大潜力，但仍面临数据标注、模型可解释性等挑战。

## 参考来源

1. arXiv:2103.00001 - Deep Learning for Medical Image Analysis
2. PubMed:12345678 - Transformer-based Methods
3. Semantic Scholar - Attention Mechanisms in Medical Imaging
"#,
        topic
    );

    report = report.with_content(content);
    report = report.with_summary(format!(
        "本报告综述了 {} 领域的深度学习技术应用。",
        topic
    ));

    report
}