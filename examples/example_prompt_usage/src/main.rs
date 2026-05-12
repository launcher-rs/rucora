//! rucora-prompt 与 rucora 结合使用示例
//!
//! 展示如何在 rucora Agent 中使用 prompt 模板
//!
//! 运行方式:
//! ```bash
//! # 设置环境变量
//! export OPENAI_API_KEY=your_key
//!
//! # 运行示例
//! cargo run --example example_prompt_usage
//! ```

use rucora_prompt::prompt;
use std::collections::HashMap;

fn main() -> anyhow::Result<()> {
    println!("\n=== rucora-prompt + rucora 使用示例 ===\n");

    // 示例 1: Agent 使用 tool prompt
    println!("【示例 1】使用 tool prompt 构建搜索指令");
    tool_prompt_demo()?;

    // 示例 2: Agent 使用 research prompt
    println!("\n【示例 2】使用 research prompt 构建研究任务");
    research_prompt_demo()?;

    // 示例 3: Agent 使用 summarize prompt
    println!("\n【示例 3】使用 summarize prompt 构建总结任务");
    summarize_prompt_demo()?;

    // 示例 4: 动态 prompt 选择
    println!("\n【示例 4】根据任务类型动态选择 prompt");
    dynamic_prompt_demo()?;

    // 示例 5: 英文 prompt
    println!("\n【示例 5】使用英文 prompt");
    english_prompt_demo()?;

    println!("\n=== 完成 ===\n");

    println!("提示: 要运行完整的 rucora 示例，请设置 OPENAI_API_KEY 环境变量");
    println!("  export OPENAI_API_KEY=your_key");

    Ok(())
}

/// 示例 1: 使用 tool prompt 构建搜索指令
fn tool_prompt_demo() -> anyhow::Result<()> {
    // 使用搜索 prompt
    let tmpl = prompt("tool_search");
    println!("  Prompt 名称: {}", tmpl.name);
    println!(
        "  System: {}...",
        tmpl.system.chars().take(50).collect::<String>()
    );

    // 渲染变量
    let mut vars = HashMap::new();
    vars.insert("query".to_string(), "Python 异步编程".to_string());
    vars.insert("goal".to_string(), "学习最佳实践".to_string());

    let output = tmpl.render(&vars);
    println!("  渲染结果:\n    {}", output);

    // 在实际使用中，可以将渲染结果作为 Agent 的输入
    // let agent = ToolAgent::builder()
    //     .system_prompt(tmpl.system.as_str())
    //     .build();
    // agent.run(output).await?;

    Ok(())
}

/// 示例 2: 使用 research prompt 构建研究任务
fn research_prompt_demo() -> anyhow::Result<()> {
    // 使用研究 prompt
    let tmpl = prompt("research_default");

    // 渲染变量
    let mut vars = HashMap::new();
    vars.insert("topic".to_string(), "人工智能在医疗领域的应用".to_string());
    vars.insert("collected".to_string(), "已收集10条相关信息".to_string());

    let msgs = tmpl.messages(&vars);

    println!("  Prompt 名称: {}", tmpl.name);
    println!(
        "  System Message:\n    {}",
        msgs[0].content.chars().take(80).collect::<String>()
    );
    println!(
        "  User Message:\n    {}",
        msgs[1].content.chars().take(100).collect::<String>()
    );

    // 在实际使用中
    // let agent = ToolAgent::builder()
    //     .system_prompt(msgs[0].content.as_str())
    //     .build();
    // agent.run(msgs[1].content.clone()).await?;

    Ok(())
}

/// 示例 3: 使用 summarize prompt 构建总结任务
fn summarize_prompt_demo() -> anyhow::Result<()> {
    // 模拟要总结的内容
    let content = r#"
    深度学习是机器学习的一个分支，它是一种以人工神经网络为架构，对数据进行表征学习的算法。
    深度学习在计算机视觉、语音识别、自然语言处理等领域取得了显著的成果。
    卷积神经网络(CNN)是深度学习中常用的模型之一，特别适用于图像处理。
    循环神经网络(RNN)适用于序列数据的处理，在自然语言处理中有广泛应用。
    "#;

    // 使用总结 prompt
    let tmpl = prompt("tool_summarize");

    let mut vars = HashMap::new();
    vars.insert("content".to_string(), content.to_string());
    vars.insert("requirements".to_string(), "简洁，不超过100字".to_string());

    let output = tmpl.render(&vars);
    println!("  Prompt 名称: {}", tmpl.name);
    println!("  渲染结果:\n    {}", output);

    Ok(())
}

/// 示例 4: 动态 prompt 选择
fn dynamic_prompt_demo() -> anyhow::Result<()> {
    // 定义任务
    let tasks = vec![
        ("search", "今天天气怎么样", "获取实时天气信息"),
        ("summarize", "总结这段话", "提取核心观点"),
        ("translate", "Hello 翻译成中文", "语言转换"),
        ("classify", "这篇文章属于哪个类别", "内容分类"),
    ];

    for (task_type, input, goal) in tasks {
        // 根据任务类型选择对应的 prompt
        let prompt_name = match task_type {
            "search" => "tool_search",
            "summarize" => "tool_summarize",
            "translate" => "tool_translate",
            "classify" => "filter_classify",
            _ => "agent_simple",
        };

        let tmpl = prompt(prompt_name);

        // 构建任务变量
        let mut vars = HashMap::new();
        match task_type {
            "search" => {
                vars.insert("query".to_string(), input.to_string());
                vars.insert("goal".to_string(), goal.to_string());
            }
            "summarize" => {
                vars.insert("content".to_string(), input.to_string());
                vars.insert("requirements".to_string(), goal.to_string());
            }
            "translate" => {
                vars.insert("content".to_string(), input.to_string());
                vars.insert("to".to_string(), goal.to_string());
            }
            "classify" => {
                vars.insert("content".to_string(), input.to_string());
                vars.insert(
                    "categories".to_string(),
                    "科技, 娱乐, 体育, 财经".to_string(),
                );
            }
            _ => {
                vars.insert("input".to_string(), input.to_string());
            }
        }

        let output = tmpl.render(&vars);
        println!("  任务: {} -> Prompt: {}", task_type, tmpl.name);

        // 实际使用
        // let agent = ToolAgent::builder().system_prompt(tmpl.system.as_str()).build();
        // agent.run(output).await?;
    }

    Ok(())
}

/// 示例 5: 使用英文 prompt
fn english_prompt_demo() -> anyhow::Result<()> {
    // 使用英文版 prompt
    let tmpl = prompt("agent_tool_en");

    let mut vars = HashMap::new();
    vars.insert("input".to_string(), "What's the weather today?".to_string());
    vars.insert("tools".to_string(), "search, browse".to_string());

    let msgs = tmpl.messages(&vars);
    println!("  英文版 Prompt: {}", tmpl.name);
    println!(
        "  System:\n    {}",
        msgs[0].content.chars().take(60).collect::<String>()
    );
    println!(
        "  User:\n    {}",
        msgs[1].content.chars().take(60).collect::<String>()
    );

    Ok(())
}
