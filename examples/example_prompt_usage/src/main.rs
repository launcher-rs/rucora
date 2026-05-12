//! rucora-prompt 使用示例
//!
//! 展示如何使用 prompt 静态常量
//!
//! 运行方式:
//! ```bash
//! cargo run --example example_prompt_usage
//! ```

use rucora_prompt::prompts;
use std::collections::HashMap;

fn main() -> anyhow::Result<()> {
    println!("\n=== rucora-prompt 使用示例 ===\n");

    // 示例 1: Agent 使用 tool prompt
    println!("【示例 1】使用 tool prompt 构建搜索指令");
    tool_prompt_demo()?;

    // 示例 2: Agent 使用 research prompt
    println!("\n【示例 2】使用 research prompt 构建研究任务");
    research_prompt_demo()?;

    // 示例 3: Agent 使用 summarize prompt
    println!("\n【示例 3】使用 summarize prompt 构建总结任务");
    summarize_prompt_demo()?;

    // 示例 4: 英文 prompt
    println!("\n【示例 4】使用英文 prompt");
    english_prompt_demo()?;

    println!("\n=== 完成 ===\n");

    Ok(())
}

/// 示例 1: 使用 tool prompt 构建搜索指令
fn tool_prompt_demo() -> anyhow::Result<()> {
    // 直接使用静态常量
    let system = prompts::tool::search::SYSTEM;
    println!(
        "  System: {}...",
        system.chars().take(50).collect::<String>()
    );

    // 渲染变量
    let mut vars = HashMap::new();
    vars.insert("query".to_string(), "Python 异步编程".to_string());
    vars.insert("goal".to_string(), "学习最佳实践".to_string());
    vars.insert("keywords".to_string(), "".to_string());

    let tmpl = prompts::tool::search::template();
    let output = tmpl.render(&vars);
    println!("  渲染结果:\n    {}", output);

    Ok(())
}

/// 示例 2: 使用 research prompt 构建研究任务
fn research_prompt_demo() -> anyhow::Result<()> {
    // 使用研究 prompt
    let tmpl = prompts::research::default::template();

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
    let tmpl = prompts::tool::summarize::template();

    let mut vars = HashMap::new();
    vars.insert("content".to_string(), content.to_string());
    vars.insert("requirements".to_string(), "简洁，不超过100字".to_string());

    let output = tmpl.render(&vars);
    println!("  Prompt 名称: {}", tmpl.name);
    println!("  渲染结果:\n    {}", output);

    Ok(())
}

/// 示例 4: 使用英文 prompt
fn english_prompt_demo() -> anyhow::Result<()> {
    // 使用英文版 prompt
    let tmpl = prompts::agent::tool_en::template();

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
