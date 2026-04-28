//! AgentKit Prompt 模板示例
//!
//! 展示如何使用 Prompt 模板系统。
//!
//! ## 运行方法
//! ```bash
//! cargo run --example 09_prompt
//! ```
//!
//! ## 功能演示
//!
//! 1. **简单变量替换** - 基础模板渲染
//! 2. **系统提示词模板** - 复杂的系统提示词
//! 3. **Few-Shot 模板** - 示例学习模板
//! 4. **条件渲染** - 根据条件显示内容
//! 5. **循环渲染** - 遍历列表渲染
//! 6. **模板组合** - 多个模板组合使用

use agentkit::prompt::PromptTemplate;
use serde_json::json;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("╔════════════════════════════════════════╗");
    info!("║   AgentKit Prompt 模板示例            ║");
    info!("╚════════════════════════════════════════╝\n");

    // ═══════════════════════════════════════════════════════════
    // 示例 1: 简单变量替换
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 1: 简单变量替换");
    info!("═══════════════════════════════════════\n");

    let template1 = PromptTemplate::from_string("你好，{{name}}！你是{{role}}。");

    info!("模板：你好，{{{{name}}}}！你是{{{{role}}}}");
    info!("变量：{:?}", template1.variables());

    let result1 = template1.render(&json!({
        "name": "张三",
        "role": "工程师"
    }))?;

    info!("渲染后：{}\n", result1);

    // ═══════════════════════════════════════════════════════════
    // 示例 2: 系统提示词模板
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 2: 系统提示词模板");
    info!("═══════════════════════════════════════\n");

    let system_template = PromptTemplate::from_string(
        "你是{{company}}的{{role}}助手。
你的职责是：
{% for duty in duties %}
- {{duty}}
{% endfor %}

请专业、友好地回答用户问题。",
    );

    info!("模板：\n系统提示词模板包含变量和循环\n");

    let system_prompt = system_template.render(&json!({
        "company": "AgentKit",
        "role": "技术",
        "duties": ["解答技术问题", "提供代码示例", "帮助调试程序"]
    }))?;

    info!("渲染后：\n{}\n", system_prompt);

    // ═══════════════════════════════════════════════════════════
    // 示例 3: Few-Shot 模板
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 3: Few-Shot 模板");
    info!("═══════════════════════════════════════\n");

    let few_shot_template = PromptTemplate::from_string(
        "请将以下中文翻译成英文：

{% for example in examples %}
示例 {{example.index}}:
中文：{{example.cn}}
英文：{{example.en}}

{% endfor %}
示例 {{last_index}}:
中文：{{input}}
英文：",
    );

    let few_shot_prompt = few_shot_template.render(&json!({
        "examples": [
            {"index": 1, "cn": "你好", "en": "Hello"},
            {"index": 2, "cn": "再见", "en": "Goodbye"},
            {"index": 3, "cn": "谢谢", "en": "Thank you"}
        ],
        "last_index": 4,
        "input": "不客气"
    }))?;

    info!("模板：\nFew-Shot 翻译模板\n");
    info!("渲染后：\n{}\n", few_shot_prompt);

    // ═══════════════════════════════════════════════════════════
    // 示例 4: 条件渲染
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 4: 条件渲染");
    info!("═══════════════════════════════════════\n");

    let conditional_template = PromptTemplate::from_string(
        "你好，{{name}}！
{% if title %}
您的头衔是：{{title}}
{% endif %}
{% if company %}
您在{{company}}工作
{% endif %}
{% if bio %}
个人简介：{{bio}}
{% endif %}
请告诉我如何帮助您？",
    );

    info!("模板：\n条件渲染模板\n");

    // 完整信息
    let full_prompt = conditional_template.render(&json!({
        "name": "李四",
        "title": "高级工程师",
        "company": "科技公司",
        "bio": "热爱编程"
    }))?;

    info!("完整信息：\n{}\n", full_prompt);

    // 部分信息
    let partial_prompt = conditional_template.render(&json!({
        "name": "王五",
        "company": "创业公司"
    }))?;

    info!("部分信息：\n{}\n", partial_prompt);

    // ═══════════════════════════════════════════════════════════
    // 示例 5: 循环渲染
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 5: 循环渲染");
    info!("═══════════════════════════════════════\n");

    let loop_template = PromptTemplate::from_string(
        "以下是今天的待办事项：

{% for task in tasks %}
{{loop.index}}. [{{task.priority}}] {{task.name}}
   描述：{{task.description}}
   截止时间：{{task.deadline}}
{% endfor %}

共 {{loop.length}} 项任务。",
    );

    let loop_prompt = loop_template.render(&json!({
        "tasks": [
            {
                "name": "完成项目报告",
                "description": "编写 Q4 项目总结报告",
                "priority": "高",
                "deadline": "2024-01-15"
            },
            {
                "name": "代码审查",
                "description": "审查团队的 PR",
                "priority": "中",
                "deadline": "2024-01-16"
            },
            {
                "name": "团队会议",
                "description": "周例会讨论进度",
                "priority": "低",
                "deadline": "2024-01-17"
            }
        ]
    }))?;

    info!("渲染后：\n{}\n", loop_prompt);

    // ═══════════════════════════════════════════════════════════
    // 示例 6: 模板组合使用
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 6: 模板组合使用");
    info!("═══════════════════════════════════════\n");

    // 角色定义模板
    let role_template =
        PromptTemplate::from_string("你是{{role_name}}，一名{{role_description}}。");

    // 约束条件模板
    let constraint_template = PromptTemplate::from_string(
        "请遵循以下约束：
{% for constraint in constraints %}
- {{constraint}}
{% endfor %}",
    );

    // 输出格式模板
    let format_template = PromptTemplate::from_string(
        "请按以下格式输出：
{{format_description}}",
    );

    // 组合使用
    let role_prompt = role_template.render(&json!({
        "role_name": "Python 专家",
        "role_description": "经验丰富的 Python 开发者，擅长 Web 开发和数据分析"
    }))?;

    let constraint_prompt = constraint_template.render(&json!({
        "constraints": [
            "代码必须符合 PEP 8 规范",
            "必须添加类型注解",
            "必须包含文档字符串",
            "必须处理异常情况"
        ]
    }))?;

    let format_prompt = format_template.render(&json!({
        "format_description": "1. 代码实现\n2. 使用说明\n3. 测试示例"
    }))?;

    // 完整的系统提示词
    let full_system_prompt = format!("{role_prompt}\n\n{constraint_prompt}\n\n{format_prompt}");

    info!("角色定义：\n{}\n", role_prompt);
    info!("约束条件：\n{}\n", constraint_prompt);
    info!("输出格式：\n{}\n", format_prompt);
    info!("完整系统提示词：\n{}\n", full_system_prompt);

    // ═══════════════════════════════════════════════════════════
    // 示例 7: 与 Agent 集成
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 7: 与 Agent 集成");
    info!("═══════════════════════════════════════\n");

    use agentkit::agent::SimpleAgent;
    use agentkit::prelude::Agent;
    use agentkit::provider::OpenAiProvider;

    // 检查配置
    if std::env::var("OPENAI_API_KEY").is_err() && std::env::var("OPENAI_BASE_URL").is_err() {
        info!("⚠ 未设置 API 配置");
        info!("   使用 OpenAI: export OPENAI_API_KEY=sk-your-key");
        info!("   使用 Ollama: export OPENAI_BASE_URL=http://localhost:11434");
        info!("\n注意：以下 Agent 演示将跳过实际 API 调用\n");
    } else {
        let model_name = std::env::var("MODEL_NAME").unwrap_or_else(|_| "gpt-4o-mini".to_string());

        info!("7.1 创建 Provider...");
        let provider = OpenAiProvider::from_env()?;
        info!("✓ Provider 创建成功\n");

        info!("7.2 使用模板创建系统提示词...");
        let agent_template = PromptTemplate::from_string(
            "你是{{role}}，专注于{{specialty}}。
你的目标是帮助用户{{goal}}。
请用{{tone}}的语气回答。",
        );

        let system_prompt = agent_template.render(&json!({
            "role": "技术写作助手",
            "specialty": "技术文档和教程编写",
            "goal": "写出清晰、易懂的技术内容",
            "tone": "专业但友好"
        }))?;

        info!("✓ 系统提示词生成成功\n");
        info!("系统提示词：\n{}\n", system_prompt);

        info!("7.3 创建 Agent...");
        let agent = SimpleAgent::builder()
            .provider(provider)
            .model(&model_name)
            .system_prompt(system_prompt)
            .build();
        info!("✓ Agent 创建成功\n");

        info!("7.4 测试 Agent...");
        let task = "请帮我写一个 Python 函数的文档字符串示例";
        info!("任务：\"{}\"\n", task);

        match agent.run(task.into()).await {
            Ok(output) => {
                if let Some(text) = output.text() {
                    info!("回答：\n{}\n", text);
                }
            }
            Err(e) => {
                info!("Agent 运行出错：{}\n", e);
            }
        }
    }

    // ═══════════════════════════════════════════════════════════
    // 总结
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例完成！");
    info!("═══════════════════════════════════════\n");

    info!("📝 Prompt 模板系统总结：\n");

    info!("1. 变量替换:");
    info!("   - 使用 {{variable}} 语法");
    info!("   - 支持嵌套对象访问\n");

    info!("2. 条件渲染:");
    info!("   - 使用 {{% if condition %}}...{{% endif %}}");
    info!("   - 支持 {{% else %}} 分支\n");

    info!("3. 循环渲染:");
    info!("   - 使用 {{% for item in list %}}...{{% endfor %}}");
    info!("   - 支持 loop.index、loop.length 等变量\n");

    info!("4. 模板组合:");
    info!("   - 多个小模板组合成大模板");
    info!("   - 便于维护和复用\n");

    info!("5. 最佳实践:");
    info!("   - 将系统提示词模板化");
    info!("   - 使用 Few-Shot 提高准确性");
    info!("   - 分离角色定义、约束和格式");
    info!("   - 使用条件渲染处理可选内容\n");

    Ok(())
}
