//! AgentKit 任务拆解示例
//!
//! 展示如何让 AI 拆解复杂问题为子问题，分别回答后再综合总结。
//!
//! ## 运行方法
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! cargo run --example 21_task_decomposition
//! ```
//!
//! ## 功能演示
//!
//! 1. **问题拆解** - AI 分析复杂问题并拆解为可执行的子问题
//! 2. **并行/串行回答** - 对每个子问题进行独立回答
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
    /// 子问题 ID
    id: usize,
    /// 子问题内容
    question: String,
    /// 子问题描述（可选）
    description: Option<String>,
}

/// 子问题答案
#[derive(Debug, Clone)]
struct SubAnswer {
    /// 子问题 ID
    question_id: usize,
    /// 问题内容
    question: String,
    /// 答案内容
    answer: String,
    /// 置信度（1-5）
    confidence: u8,
}

/// 最终综合结果
#[derive(Debug, Clone)]
struct SynthesizedResult {
    /// 子问题列表
    sub_questions: Vec<SubQuestion>,
    /// 子问题答案列表
    sub_answers: Vec<SubAnswer>,
    /// 综合总结
    summary: String,
    /// 质量评估（1-5）
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
    /// 创建新的任务拆解器
    fn new(provider: P) -> Self {
        let agent = SimpleAgent::builder()
            .provider(provider)
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

    /// 拆解问题
    async fn decompose(&self, question: &str) -> anyhow::Result<Vec<SubQuestion>> {
        info!("🔍 正在拆解问题...");

        let prompt = format!("请拆解以下问题：\n\n{}", question);
        let output = self.agent.run(prompt.into()).await?;

        let response = output.text().unwrap_or("[]");

        // 尝试解析 JSON
        let sub_questions: Vec<SubQuestion> = serde_json::from_str(response).unwrap_or_else(|_| {
            // 如果解析失败，返回默认的子问题
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
    /// 创建新的答案综合器
    fn new(provider: P) -> Self {
        let agent = SimpleAgent::builder()
            .provider(provider)
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

    /// 综合答案
    async fn synthesize(
        &self,
        original_question: &str,
        sub_answers: &[SubAnswer],
    ) -> anyhow::Result<(String, u8)> {
        info!("🔗 正在综合答案...");

        // 构建综合提示
        let mut prompt = format!("原始问题：{}\n\n", original_question);
        prompt.push_str("子问题答案：\n\n");

        for answer in sub_answers.iter() {
            prompt.push_str(&format!(
                "【子问题 {}】{}\n【答案】{}\n【置信度】{}/5\n\n",
                answer.question_id, answer.question, answer.answer, answer.confidence
            ));
        }

        prompt.push_str("\n请综合以上所有答案，回答原始问题。");

        let output = self.agent.run(prompt.into()).await?;
        let response = output.text().unwrap_or("无法综合答案").to_string();

        // 提取质量评分
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
    /// 创建新的回答器
    fn new(provider: P) -> Self {
        let agent = ToolAgent::builder()
            .provider(provider)
            .system_prompt(
                "你是一个专业的子问题回答专家。你的职责是：\n\
                 1. 准确理解子问题\n\
                 2. 提供详细、准确的答案\n\
                 3. 如有必要，使用工具获取额外信息\n\
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

    /// 回答子问题
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

        // 提取置信度
        let confidence = response
            .lines()
            .find(|line| line.contains("置信度"))
            .and_then(|line| {
                line.chars()
                    .find(|c| c.is_ascii_digit())
                    .and_then(|c| c.to_digit(10).map(|d| d as u8))
            })
            .unwrap_or(3);

        // 提取答案内容（移除置信度行）
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
    decomposer_factory: Box<dyn Fn() -> OpenAiProvider + Send + Sync>,
    answerer_factory: Box<dyn Fn() -> OpenAiProvider + Send + Sync>,
    synthesizer_factory: Box<dyn Fn() -> OpenAiProvider + Send + Sync>,
}

impl TaskDecompositionSystem {
    /// 创建新系统
    fn new() -> Self {
        Self {
            decomposer_factory: Box::new(|| OpenAiProvider::from_env().unwrap()),
            answerer_factory: Box::new(|| OpenAiProvider::from_env().unwrap()),
            synthesizer_factory: Box::new(|| OpenAiProvider::from_env().unwrap()),
        }
    }

    /// 处理复杂问题
    async fn process(&self, question: &str) -> anyhow::Result<SynthesizedResult> {
        info!("╔════════════════════════════════════════╗");
        info!("║   开始处理复杂问题                     ║");
        info!("╚════════════════════════════════════════╝\n");

        info!("📋 原始问题：{}\n", question);

        // 步骤 1: 拆解问题
        let decomposer = TaskDecomposer::new((self.decomposer_factory)());
        let sub_questions = decomposer.decompose(question).await?;
        info!("✓ 问题拆解完成\n");

        // 步骤 2: 回答每个子问题
        info!("═══════════════════════════════════════");
        info!("步骤 2: 回答子问题");
        info!("═══════════════════════════════════════\n");

        let mut sub_answers = Vec::new();
        for sub_question in &sub_questions {
            let answerer = SubQuestionAnswerer::new((self.answerer_factory)());
            let answer = answerer.answer(sub_question).await?;
            sub_answers.push(answer);
        }
        info!("✓ 所有子问题回答完成\n");

        // 步骤 3: 综合答案
        info!("═══════════════════════════════════════");
        info!("步骤 3: 综合答案");
        info!("═══════════════════════════════════════\n");

        let synthesizer = AnswerSynthesizer::new((self.synthesizer_factory)());
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

    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("╔════════════════════════════════════════╗");
    info!("║   AgentKit 任务拆解示例                ║");
    info!("╚════════════════════════════════════════╝\n");

    // 检查配置
    if std::env::var("OPENAI_API_KEY").is_err() && std::env::var("OPENAI_BASE_URL").is_err() {
        info!("⚠ 未设置 API 配置");
        info!("   使用 OpenAI: export OPENAI_API_KEY=sk-your-key");
        info!("   使用 Ollama: export OPENAI_BASE_URL=http://localhost:11434");
        return Ok(());
    }

    // ═══════════════════════════════════════════════════════════
    // 创建任务拆解系统
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("创建任务拆解系统");
    info!("═══════════════════════════════════════\n");

    info!("1. 创建任务拆解系统...");
    let system = TaskDecompositionSystem::new();
    info!("✓ 系统创建成功\n");

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

    // ═══════════════════════════════════════════════════════════
    // 系统架构总结
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("系统架构总结");
    info!("═══════════════════════════════════════\n");

    info!("```");
    info!("┌─────────────────────────────────────────┐");
    info!("│           复杂问题输入                   │");
    info!("└─────────────────┬───────────────────────┘");
    info!("                  │");
    info!("                  ▼");
    info!("┌─────────────────────────────────────────┐");
    info!("│         TaskDecomposer                   │");
    info!("│    (问题拆解专家)                         │");
    info!("│  - 分析问题结构                          │");
    info!("│  - 识别核心要点                          │");
    info!("│  - 生成独立子问题                        │");
    info!("└─────────────────┬───────────────────────┘");
    info!("                  │");
    info!("                  ▼");
    info!("┌─────────────────────────────────────────┐");
    info!("│    SubQuestion #1    SubQuestion #2 ... │");
    info!("└─────────────────┬───────────────────────┘");
    info!("                  │");
    info!("                  ▼");
    info!("┌─────────────────────────────────────────┐");
    info!("│      SubQuestionAnswerer                 │");
    info!("│    (子问题回答专家)                       │");
    info!("│  - 独立回答每个子问题                    │");
    info!("│  - 可使用工具获取信息                    │");
    info!("│  - 评估答案置信度                        │");
    info!("└─────────────────┬───────────────────────┘");
    info!("                  │");
    info!("                  ▼");
    info!("┌─────────────────────────────────────────┐");
    info!("│      Answer #1    Answer #2    Answer   │");
    info!("└─────────────────┬───────────────────────┘");
    info!("                  │");
    info!("                  ▼");
    info!("┌─────────────────────────────────────────┐");
    info!("│       AnswerSynthesizer                  │");
    info!("│    (答案综合专家)                         │");
    info!("│  - 整合所有答案                          │");
    info!("│  - 消除重复和矛盾                        │");
    info!("│  - 生成结构化总结                        │");
    info!("│  - 评估整体质量                          │");
    info!("└─────────────────┬───────────────────────┘");
    info!("                  │");
    info!("                  ▼");
    info!("┌─────────────────────────────────────────┐");
    info!("│         综合总结输出                     │");
    info!("│    + 质量评分 + 改进建议                 │");
    info!("└─────────────────────────────────────────┘");
    info!("```\n");

    // ═══════════════════════════════════════════════════════════
    // 使用场景
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("使用场景");
    info!("═══════════════════════════════════════\n");

    info!("1. 复杂问题解答:");
    info!("   - 技术问题需要多角度分析");
    info!("   - 学术研究需要全面调研");
    info!("   - 决策问题需要权衡利弊\n");

    info!("2. 报告生成:");
    info!("   - 市场调研报告");
    info!("   - 竞品分析报告");
    info!("   - 技术可行性报告\n");

    info!("3. 学习规划:");
    info!("   - 制定学习计划");
    info!("   - 知识体系梳理");
    info!("   - 技能路径规划\n");

    info!("4. 产品设计:");
    info!("   - 需求分析");
    info!("   - 架构设计");
    info!("   - 风险评估\n");

    // ═══════════════════════════════════════════════════════════
    // 优化建议
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("优化建议");
    info!("═══════════════════════════════════════\n");

    info!("1. 并行处理:");
    info!("   - 使用 tokio::join! 并行回答子问题");
    info!("   - 减少总体响应时间\n");

    info!("2. 缓存机制:");
    info!("   - 缓存常见子问题的答案");
    info!("   - 避免重复计算\n");

    info!("3. 迭代优化:");
    info!("   - 根据质量评分自动调整拆解策略");
    info!("   - 对低质量答案进行追问\n");

    info!("4. 专家路由:");
    info!("   - 根据子问题类型路由到不同专家");
    info!("   - 提高答案专业性\n");

    // ═══════════════════════════════════════════════════════════
    // 总结
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例完成！");
    info!("═══════════════════════════════════════\n");

    info!("📝 任务拆解系统总结：\n");

    info!("1. 核心优势:");
    info!("   - 结构化思考 - 将复杂问题拆解为可管理的小问题");
    info!("   - 专注回答 - 每个子问题独立回答，提高准确性");
    info!("   - 综合总结 - 整合信息形成完整答案");
    info!("   - 质量评估 - 自动评估答案质量\n");

    info!("2. 关键组件:");
    info!("   - TaskDecomposer - 问题拆解专家");
    info!("   - SubQuestionAnswerer - 子问题回答专家");
    info!("   - AnswerSynthesizer - 答案综合专家\n");

    info!("3. 适用场景:");
    info!("   - 复杂技术咨询");
    info!("   - 研究报告生成");
    info!("   - 决策支持");
    info!("   - 学习规划\n");

    info!("4. 扩展方向:");
    info!("   - 添加更多专业回答器");
    info!("   - 实现并行处理");
    info!("   - 集成 RAG 增强答案质量");
    info!("   - 添加记忆系统记住历史拆解\n");

    Ok(())
}
