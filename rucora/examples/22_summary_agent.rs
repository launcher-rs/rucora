//! rucora Summary Agent 示例
//!
//! 展示如何使用 SummaryAgent 进行文本摘要，包括多模式输出和自定义提示词。
//!
//! ## 运行方法
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! # 或使用 Ollama
//! export OPENAI_BASE_URL=http://127.0.0.1:11434
//! cargo run --example 22_summary_agent
//! ```

use rucora::agent::{SummaryAgent, SummaryMode};
use rucora::prelude::Agent;
use rucora::provider::OpenAiProvider;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("╔════════════════════════════════════════╗");
    info!("║   rucora Summary Agent 示例          ║");
    info!("╚════════════════════════════════════════╝\n");

    if std::env::var("OPENAI_API_KEY").is_err() && std::env::var("OPENAI_BASE_URL").is_err() {
        info!("⚠ 未设置 API 配置");
        info!("   使用 OpenAI: export OPENAI_API_KEY=sk-your-key");
        info!("   使用 Ollama: export OPENAI_BASE_URL=http://localhost:11434");
        return Ok(());
    }

    let model = std::env::var("MODEL_NAME").unwrap_or_else(|_| "gpt-4o-mini".to_string());
    info!("使用模型: {model}\n");

    info!("═══════════════════════════════════════");
    info!("SummaryAgent 模式说明:");
    info!("═══════════════════════════════════════");
    info!("1. Concise - 简洁摘要（2-3 句话）");
    info!("2. Detailed - 详细摘要");
    info!("3. BulletPoints - 要点列表");
    info!("4. KeyPoints - 关键信息提取");
    info!("5. Custom - 自定义指令");
    info!("═══════════════════════════════════════\n");

    let provider = OpenAiProvider::from_env()?;

    // 测试文本
    let long_text = r#"
随着人工智能技术的快速发展，大语言模型在各个领域的应用越来越广泛。
从最初的文本生成、翻译、问答，到现在的代码生成、数据分析、科学研究，LLM 正在改变我们工作和学习的方式。

在软件开发领域，AI 辅助编程工具正在帮助开发者提高效率。
GitHub Copilot、Cursor 等工具能够根据上下文自动补全代码、生成单元测试、重构代码结构。
这些工具不仅提高了开发速度，还帮助开发者学习新的编程语言和框架。

在数据分析方面，LLM 能够帮助分析师快速处理大量文本数据。
无论是情感分析、主题建模还是信息提取，AI 都能在短时间内完成传统方法需要数小时甚至数天的工作。
这使得企业能够更快地从数据中获取洞察，做出更好的决策。

在内容创作领域，AI 助手正在协助作家、营销人员和设计师。
它们可以帮助生成文章大纲、优化文案、创作社交媒体内容。
虽然 AI 还不能完全替代人类的创造力，但它确实能够极大地提高生产效率。

然而，我们也需要注意 AI 技术的潜在风险。
包括数据隐私、算法偏见、信息准确性等问题都需要认真对待。
负责任地使用 AI 技术，建立适当的监管框架，是我们面临的重要课题。
"#;

    // ═══════════════════════════════════════════════════════════
    // 演示 1: 简洁摘要
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("演示 1: 简洁摘要（Concise）");
    info!("═══════════════════════════════════════\n");

    let agent = SummaryAgent::builder()
        .provider(provider.clone())
        .model(&model)
        .mode(SummaryMode::Concise)
        .temperature(0.3)
        .build();

    info!("文本内容（{} 字）：\n{}\n", long_text.chars().count(), long_text);

    match agent.run(long_text.into()).await {
        Ok(output) => {
            if let Some(text) = output.text() {
                info!("简洁摘要：\n{text}\n");
            }
        }
        Err(e) => {
            info!("❌ 处理失败：{e}\n");
        }
    }

    // ═══════════════════════════════════════════════════════════
    // 演示 2: 要点列表
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("演示 2: 要点列表（BulletPoints）");
    info!("═══════════════════════════════════════\n");

    let agent = SummaryAgent::builder()
        .provider(provider.clone())
        .model(&model)
        .mode(SummaryMode::BulletPoints)
        .temperature(0.3)
        .build();

    match agent.run(long_text.into()).await {
        Ok(output) => {
            if let Some(text) = output.text() {
                info!("要点列表：\n{text}\n");
            }
        }
        Err(e) => {
            info!("❌ 处理失败：{e}\n");
        }
    }

    // ═══════════════════════════════════════════════════════════
    // 演示 3: 自定义提示词 + 自定义模式
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("演示 3: 自定义提示词 + 英文输出");
    info!("═══════════════════════════════════════\n");

    let agent = SummaryAgent::builder()
        .provider(provider.clone())
        .model(&model)
        .mode("Please summarize the following text in English, focusing on key technical details.")
        .prompt_template("Summarize this text:\n\n{text}\n\n{mode}")
        .chunk_template("[Part {index}/{total}]\n{text}\n\nPlease summarize this part in English.")
        .combine_template("Now combine all {total} partial summaries into a complete English summary.\n\n{mode}")
        .temperature(0.3)
        .build();

    match agent.run(long_text.into()).await {
        Ok(output) => {
            if let Some(text) = output.text() {
                info!("英文摘要：\n{text}\n");
            }
        }
        Err(e) => {
            info!("❌ 处理失败：{e}\n");
        }
    }

    // ═══════════════════════════════════════════════════════════
    // 演示 4: 长文本分块（map-reduce）
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("演示 4: 长文本自动分块（chunk_size=200）");
    info!("═══════════════════════════════════════\n");

    let mut mega_text = String::new();
    for i in 1..=5 {
        mega_text.push_str(&format!(
            "## 章节 {i}\n\n这是第 {i} 章的内容。这里包含一些详细描述，用于演示 \
             SummaryAgent 的长文本自动分块功能。当文本超过 chunk_size 时，\
             Agent 会自动将文本分割成多个块，分别总结后合并。\
             这样可以处理超出上下文窗口的长文档。\n\n"
        ));
    }

    let agent = SummaryAgent::builder()
        .provider(provider.clone())
        .model(&model)
        .mode(SummaryMode::Detailed)
        .chunk_size(200)
        .temperature(0.3)
        .build();

    info!("长文本（{} 字节）：\n{mega_text}\n", mega_text.len());

    match agent.run(mega_text.into()).await {
        Ok(output) => {
            if let Some(text) = output.text() {
                info!("自动分块摘要：\n{text}\n");
            }
        }
        Err(e) => {
            info!("❌ 处理失败：{e}\n");
        }
    }

    info!("═══════════════════════════════════════");
    info!("示例完成！");
    info!("═══════════════════════════════════════\n");

    info!("SummaryAgent 总结：\n");
    info!("1. 多模式输出: Concise / Detailed / BulletPoints / KeyPoints / Custom");
    info!("2. 自定义模板: 通过 prompt_template / chunk_template / combine_template 定制提示词");
    info!("3. 长文本支持: 自动分块合并 (map-reduce)，通过 chunk_size 控制分块大小");
    info!("4. 适用场景: 文档总结、会议纪要、论文摘要、信息提取");

    Ok(())
}
