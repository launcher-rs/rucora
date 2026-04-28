//! AgentKit 任务拆解示例
//!
//! 展示如何让 AI 拆解复杂问题为子问题，分别回答后再综合总结。
//!
//! ## 运行方法
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! # 或使用 Ollama
//! export OPENAI_BASE_URL=http://127.0.0.1:11434
//! cargo run --example 13_task_decomposition
//! ```
//!
//! ## 功能演示
//!
//! 1. **问题拆解** - AI 分析复杂问题并拆解为可执行的子问题
//! 2. **串行回答** - 对每个子问题进行独立回答
//! 3. **综合总结** - 整合所有子问题的答案形成完整回复
//! 4. **质量评估** - 评估答案的完整性和一致性

use agentkit::agent::{SimpleAgent, ToolAgent};
use agentkit::prelude::Agent;
use agentkit::provider::OpenAiProvider;
use agentkit::tools::{EchoTool, ShellTool};
use serde::{Deserialize, Serialize};
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

// ═══════════════════════════════════════════════════════════
// 数据结构定义
// ═══════════════════════════════════════════════════════════

/// 子问题
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SubQuestion {
    id: usize,
    question: String,
    description: Option<String>,
}

/// 子问题答案
#[derive(Debug, Clone)]
struct SubAnswer {
    question_id: usize,
    question: String,
    answer: String,
    confidence: u8,
}

/// 最终综合结果
#[derive(Debug, Clone)]
struct SynthesizedResult {
    sub_questions: Vec<SubQuestion>,
    sub_answers: Vec<SubAnswer>,
    summary: String,
    quality_score: u8,
}

// ═══════════════════════════════════════════════════════════
// 任务拆解器
// ═══════════════════════════════════════════════════════════

/// 任务拆解器 - 负责将复杂问题拆解为子问题
struct TaskDecomposer<P> {
    agent: SimpleAgent<P>,
}

impl<P> TaskDecomposer<P>
where
    P: agentkit_core::provider::LlmProvider + Send + Sync + 'static,
{
    fn new(provider: P, model: &str) -> Self {
        let agent = SimpleAgent::builder()
            .provider(provider)
            .model(model.to_string())
            .system_prompt(
                "你是一个专业的任务拆解专家。你的职责是：\n\
                 1. 分析复杂问题，识别核心要点\n\
                 2. 将问题拆解为 3-5 个独立的子问题\n\
                 3. 确保子问题之间逻辑清晰、相互独立\n\
                 4. 子问题应该覆盖原问题的所有重要方面\n\n\
                 请以 JSON 格式返回子问题列表，格式如下：\n\
                 [\n\
                   {\"id\": 1, \"question\": \"子问题 1\", \"description\": \"简要说明\"},\n\
                   {\"id\": 2, \"question\": \"子问题 2\", \"description\": \"简要说明\"}\n\
                 ]\n\n\
                 注意：只返回 JSON 数组，不要有其他内容。",
            )
            .build();
        Self { agent }
    }

    async fn decompose(&self, question: &str) -> anyhow::Result<Vec<SubQuestion>> {
        info!("🔍 正在拆解问题...");

        let prompt = format!("请拆解以下问题：\n\n{question}");
        let output = self.agent.run(prompt.into()).await?;

        let response = output.text().unwrap_or("[]");

        let sub_questions: Vec<SubQuestion> = serde_json::from_str(response).unwrap_or_else(|_| {
            info!("⚠ JSON 解析失败，使用默认拆解");
            vec![SubQuestion {
                id: 1,
                question: question.to_string(),
                description: Some("默认子问题".to_string()),
            }]
        });

        info!("✓ 拆解为 {} 个子问题", sub_questions.len());
        Ok(sub_questions)
    }
}

// ═══════════════════════════════════════════════════════════
// 答案综合器
// ═══════════════════════════════════════════════════════════

/// 答案综合器 - 负责整合多个子问题的答案
struct AnswerSynthesizer<P> {
    agent: SimpleAgent<P>,
}

impl<P> AnswerSynthesizer<P>
where
    P: agentkit_core::provider::LlmProvider + Send + Sync + 'static,
{
    fn new(provider: P, model: &str) -> Self {
        let agent = SimpleAgent::builder()
            .provider(provider)
            .model(model.to_string())
            .system_prompt(
                "你是一个专业的答案综合专家。你的职责是：\n\
                 1. 阅读所有子问题及其答案\n\
                 2. 整合信息，形成连贯、完整的综合回答\n\
                 3. 消除重复和矛盾的信息\n\
                 4. 结构化组织内容，使其易于理解\n\
                 5. 评估答案的整体质量（1-5 分）\n\n\
                 请以以下格式返回：\n\
                 【综合总结】\n\
                 (你的综合回答)\n\n\
                 【质量评分】X/5\n\n\
                 【改进建议】\n\
                 (可选的改进建议)",
            )
            .build();
        Self { agent }
    }

    async fn synthesize(
        &self,
        original_question: &str,
        sub_answers: &[SubAnswer],
    ) -> anyhow::Result<(String, u8)> {
        info!("🔗 正在综合答案...");

        let mut prompt = format!("原始问题：{original_question}\n\n");
        prompt.push_str("子问题答案：\n\n");

        for answer in sub_answers {
            prompt.push_str(&format!(
                "【子问题 {}】{}\n【答案】{}\n【置信度】{}/5\n\n",
                answer.question_id, answer.question, answer.answer, answer.confidence
            ));
        }

        prompt.push_str("\n请综合以上所有答案，回答原始问题。");

        let output = self.agent.run(prompt.into()).await?;
        let response = output.text().unwrap_or("无法综合答案").to_string();

        let quality_score = response
            .lines()
            .find(|line| line.contains("质量评分"))
            .and_then(|line| {
                line.chars()
                    .find(|c| c.is_ascii_digit())
                    .and_then(|c| c.to_digit(10).map(|d| d as u8))
            })
            .unwrap_or(3);

        info!("✓ 答案综合完成，质量评分：{}/5", quality_score);
        Ok((response, quality_score))
    }
}

// ═══════════════════════════════════════════════════════════
// 子问题回答器
// ═══════════════════════════════════════════════════════════

/// 子问题回答器 - 负责回答单个子问题
struct SubQuestionAnswerer<P> {
    agent: ToolAgent<P>,
}

impl<P> SubQuestionAnswerer<P>
where
    P: agentkit_core::provider::LlmProvider + Send + Sync + 'static,
{
    fn new(provider: P, model: &str) -> Self {
        let agent = ToolAgent::builder()
            .provider(provider)
            .model(model.to_string())
            .system_prompt(
                "你是一个专业的子问题回答专家。你的职责是：\n\
                 1. 准确理解子问题\n\
                 2. 提供详细、准确的答案\n\
                 3. 如有必要，使用可用的工具获取额外信息\n\
                 4. 在答案末尾评估你的置信度（1-5 分）\n\n\
                 请以以下格式返回：\n\
                 【答案】\n\
                 (你的详细回答)\n\n\
                 【置信度】X/5",
            )
            .tool(EchoTool)
            .tool(ShellTool::new())
            .max_steps(5)
            .build();
        Self { agent }
    }

    async fn answer(&self, sub_question: &SubQuestion) -> anyhow::Result<SubAnswer> {
        info!(
            "  📝 回答子问题 #{}: {}",
            sub_question.id, sub_question.question
        );

        let prompt = if let Some(desc) = &sub_question.description {
            format!(
                "问题：{}\n说明：{}\n\n请详细回答。",
                sub_question.question, desc
            )
        } else {
            format!("问题：{}\n\n请详细回答。", sub_question.question)
        };

        let output = self.agent.run(prompt.into()).await?;
        let response = output.text().unwrap_or("无法回答");

        let confidence = response
            .lines()
            .find(|line| line.contains("置信度"))
            .and_then(|line| {
                line.chars()
                    .find(|c| c.is_ascii_digit())
                    .and_then(|c| c.to_digit(10).map(|d| d as u8))
            })
            .unwrap_or(3);

        let answer = response
            .lines()
            .filter(|line| !line.contains("置信度") && !line.contains("【置信度】"))
            .collect::<Vec<_>>()
            .join("\n");

        info!("  ✓ 完成，置信度：{}/5", confidence);

        Ok(SubAnswer {
            question_id: sub_question.id,
            question: sub_question.question.clone(),
            answer,
            confidence,
        })
    }
}

// ═══════════════════════════════════════════════════════════
// 主流程
// ═══════════════════════════════════════════════════════════

/// 任务拆解与综合系统
struct TaskDecompositionSystem {
    make_provider: Box<dyn Fn() -> anyhow::Result<OpenAiProvider> + Send + Sync>,
    model: String,
}

impl TaskDecompositionSystem {
    fn new(
        make_provider: impl Fn() -> anyhow::Result<OpenAiProvider> + Send + Sync + 'static,
        model: &str,
    ) -> Self {
        Self {
            make_provider: Box::new(make_provider),
            model: model.to_string(),
        }
    }

    async fn process(&self, question: &str) -> anyhow::Result<SynthesizedResult> {
        info!("╔════════════════════════════════════════╗");
        info!("║   开始处理复杂问题                     ║");
        info!("╚════════════════════════════════════════╝\n");

        info!("📋 原始问题：{}\n", question);

        // 步骤 1: 拆解问题
        let provider = (self.make_provider)()?;
        let decomposer = TaskDecomposer::new(provider, &self.model);
        let sub_questions = decomposer.decompose(question).await?;
        info!("✓ 问题拆解完成\n");

        // 步骤 2: 回答每个子问题
        info!("═══════════════════════════════════════");
        info!("步骤 2: 回答子问题");
        info!("═══════════════════════════════════════\n");

        let mut sub_answers = Vec::new();
        for sub_question in &sub_questions {
            let provider = (self.make_provider)()?;
            let answerer = SubQuestionAnswerer::new(provider, &self.model);
            let answer = answerer.answer(sub_question).await?;
            sub_answers.push(answer);
        }
        info!("✓ 所有子问题回答完成\n");

        // 步骤 3: 综合答案
        info!("═══════════════════════════════════════");
        info!("步骤 3: 综合答案");
        info!("═══════════════════════════════════════\n");

        let provider = (self.make_provider)()?;
        let synthesizer = AnswerSynthesizer::new(provider, &self.model);
        let (summary, quality_score) = synthesizer.synthesize(question, &sub_answers).await?;
        info!("✓ 答案综合完成\n");

        Ok(SynthesizedResult {
            sub_questions,
            sub_answers,
            summary,
            quality_score,
        })
    }
}

// ═══════════════════════════════════════════════════════════
// 主函数
// ═══════════════════════════════════════════════════════════

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("╔════════════════════════════════════════╗");
    info!("║   AgentKit 任务拆解示例                ║");
    info!("╚════════════════════════════════════════╝\n");

    if std::env::var("OPENAI_API_KEY").is_err() && std::env::var("OPENAI_BASE_URL").is_err() {
        info!("⚠ 未设置 API 配置");
        info!("   使用 OpenAI: export OPENAI_API_KEY=sk-your-key");
        info!("   使用 Ollama: export OPENAI_BASE_URL=http://localhost:11434");
        return Ok(());
    }

    // let model = std::env::var("MODEL_NAME").unwrap_or_else(|_| "gpt-4o".to_string());
    let model = "qwen3.5:9b";
    info!("使用模型: {}\n", model);

    let make_provider = || Ok(OpenAiProvider::from_env()?);
    let system = TaskDecompositionSystem::new(make_provider, model);

    // ═══════════════════════════════════════════════════════════
    // 演示任务 1: 技术调研
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("演示任务 1: 技术调研");
    info!("═══════════════════════════════════════\n");

    let question1 =
        "如何设计一个高可用的分布式系统？请从架构设计、容错机制、数据一致性等方面分析。";

    match system.process(question1).await {
        Ok(result) => {
            info!("╔════════════════════════════════════════╗");
            info!("║   任务 1 结果                           ║");
            info!("╚════════════════════════════════════════╝\n");

            info!("📋 拆解的子问题:");
            for sq in &result.sub_questions {
                info!(
                    "   #{}: {} - {}",
                    sq.id,
                    sq.question,
                    sq.description.as_deref().unwrap_or("")
                );
            }
            info!("");

            info!("📝 子问题答案摘要:");
            for ans in &result.sub_answers {
                let preview: String = ans.answer.chars().take(50).collect();
                info!(
                    "   #{}: {}... (置信度：{}/5)",
                    ans.question_id, preview, ans.confidence
                );
            }
            info!("");

            info!("🔗 综合总结:");
            info!("{}\n", result.summary);

            info!("⭐ 质量评分：{}/5\n", result.quality_score);
        }
        Err(e) => {
            info!("❌ 处理失败：{}\n", e);
        }
    }

    // ═══════════════════════════════════════════════════════════
    // 演示任务 2: 学习计划制定
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("演示任务 2: 学习计划制定");
    info!("═══════════════════════════════════════\n");

    let question2 = "我想在 3 个月内学会 Rust 编程并能够开发实际项目，请帮我制定详细的学习计划，包括学习资源、实践项目和时间安排。";

    match system.process(question2).await {
        Ok(result) => {
            info!("╔════════════════════════════════════════╗");
            info!("║   任务 2 结果                           ║");
            info!("╚════════════════════════════════════════╝\n");

            info!("📋 拆解的子问题:");
            for sq in &result.sub_questions {
                info!(
                    "   #{}: {} - {}",
                    sq.id,
                    sq.question,
                    sq.description.as_deref().unwrap_or("")
                );
            }
            info!("");

            info!("📝 子问题答案摘要:");
            for ans in &result.sub_answers {
                let preview: String = ans.answer.chars().take(50).collect();
                info!(
                    "   #{}: {}... (置信度：{}/5)",
                    ans.question_id, preview, ans.confidence
                );
            }
            info!("");

            info!("🔗 综合总结:");
            info!("{}\n", result.summary);

            info!("⭐ 质量评分：{}/5\n", result.quality_score);
        }
        Err(e) => {
            info!("❌ 处理失败：{}\n", e);
        }
    }

    // ═══════════════════════════════════════════════════════════
    // 演示任务 3: 产品分析
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("演示任务 3: 产品分析");
    info!("═══════════════════════════════════════\n");

    let question3 = "分析开发一个 AI 驱动的个人知识管理应用需要考虑的关键因素，包括技术选型、用户体验、数据安全等。";

    match system.process(question3).await {
        Ok(result) => {
            info!("╔════════════════════════════════════════╗");
            info!("║   任务 3 结果                           ║");
            info!("╚════════════════════════════════════════╝\n");

            info!("📋 拆解的子问题:");
            for sq in &result.sub_questions {
                info!(
                    "   #{}: {} - {}",
                    sq.id,
                    sq.question,
                    sq.description.as_deref().unwrap_or("")
                );
            }
            info!("");

            info!("📝 子问题答案摘要:");
            for ans in &result.sub_answers {
                let preview: String = ans.answer.chars().take(50).collect();
                info!(
                    "   #{}: {}... (置信度：{}/5)",
                    ans.question_id, preview, ans.confidence
                );
            }
            info!("");

            info!("🔗 综合总结:");
            info!("{}\n", result.summary);

            info!("⭐ 质量评分：{}/5\n", result.quality_score);
        }
        Err(e) => {
            info!("❌ 处理失败：{}\n", e);
        }
    }

    info!("═══════════════════════════════════════");
    info!("示例完成！");
    info!("═══════════════════════════════════════\n");

    info!("系统架构：\n");
    info!(
        "  复杂问题 → TaskDecomposer(拆解) → SubQuestionAnswerer(逐个回答) → AnswerSynthesizer(综合)"
    );
    info!("");
    info!("适用场景：复杂技术咨询、研究报告、决策支持、学习规划");

    Ok(())
}
