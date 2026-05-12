//! rucora-prompt 使用示例
//!
//! 展示如何在不同场景下使用 prompt 模板

use rucora_prompt::prompt;
use std::collections::HashMap;

fn main() -> anyhow::Result<()> {
    println!("=== rucora-prompt 使用示例 ===\n");

    // 示例 1: Agent 简单问答
    println!("【示例 1】Agent 简单问答");
    agent_simple_demo()?;

    // 示例 2: Agent 工具调用
    println!("\n【示例 2】Agent 工具调用");
    agent_tool_demo()?;

    // 示例 3: 工具 - 搜索
    println!("\n【示例 3】工具 - 搜索");
    tool_search_demo()?;

    // 示例 4: 工具 - 总结
    println!("\n【示例 4】工具 - 总结");
    tool_summarize_demo()?;

    // 示例 5: 工具 - 翻译
    println!("\n【示例 5】工具 - 翻译");
    tool_translate_demo()?;

    // 示例 6: 研究 - 默认
    println!("\n【示例 6】研究 - 默认");
    research_default_demo()?;

    // 示例 7: 研究 - 学术
    println!("\n【示例 7】研究 - 学术");
    research_academic_demo()?;

    // 示例 8: 过滤 - 分类
    println!("\n【示例 8】过滤 - 分类");
    filter_classify_demo()?;

    // 示例 9: 英文 prompt
    println!("\n【示例 9】英文 prompt");
    english_prompt_demo()?;

    println!("\n=== 完成 ===");
    Ok(())
}

/// Agent 简单问答
fn agent_simple_demo() -> anyhow::Result<()> {
    let tmpl = prompt("agent_simple");
    let mut vars = HashMap::new();
    vars.insert("input".to_string(), "什么是人工智能?".to_string());

    let output = tmpl.render(&vars);
    println!("  输入: 什么是人工智能?");
    println!("  输出:\n{}", output);

    Ok(())
}

/// Agent 工具调用
fn agent_tool_demo() -> anyhow::Result<()> {
    let tmpl = prompt("agent_tool");
    let mut vars = HashMap::new();
    vars.insert("input".to_string(), "帮我查下北京今天的天气".to_string());
    vars.insert("tools".to_string(), "search, browse".to_string());

    let msgs = tmpl.messages(&vars);
    println!(
        "  System:\n    {}",
        msgs[0].content.chars().take(50).collect::<String>()
    );
    println!(
        "  User:\n    {}",
        msgs[1].content.chars().take(100).collect::<String>()
    );

    Ok(())
}

/// 工具 - 搜索
fn tool_search_demo() -> anyhow::Result<()> {
    let tmpl = prompt("tool_search");
    let mut vars = HashMap::new();
    vars.insert("query".to_string(), "Python 异步编程".to_string());
    vars.insert("goal".to_string(), "学习最佳实践".to_string());

    let output = tmpl.render(&vars);
    println!("  模板内容:\n{}", output);

    Ok(())
}

/// 工具 - 总结
fn tool_summarize_demo() -> anyhow::Result<()> {
    let tmpl = prompt("tool_summarize");
    let mut vars = HashMap::new();
    vars.insert(
        "content".to_string(),
        "这是一篇关于机器学习的文章。它介绍了监督学习、无监督学习和强化学习三种主要类型..."
            .to_string(),
    );
    vars.insert("requirements".to_string(), "简洁，不超过50字".to_string());

    let output = tmpl.render(&vars);
    println!("  模板内容:\n{}", output);

    Ok(())
}

/// 工具 - 翻译
fn tool_translate_demo() -> anyhow::Result<()> {
    let tmpl = prompt("tool_translate");
    let mut vars = HashMap::new();
    vars.insert("content".to_string(), "Hello, world!".to_string());
    vars.insert("to".to_string(), "中文".to_string());

    let output = tmpl.render(&vars);
    println!("  模板内容:\n{}", output);

    Ok(())
}

/// 研究 - 默认
fn research_default_demo() -> anyhow::Result<()> {
    let tmpl = prompt("research_default");
    let mut vars = HashMap::new();
    vars.insert("topic".to_string(), "人工智能在医疗领域的应用".to_string());
    vars.insert("collected".to_string(), "已收集10条相关信息...".to_string());

    let msgs = tmpl.messages(&vars);
    println!(
        "  System (前100字):\n    {}",
        msgs[0].content.chars().take(100).collect::<String>()
    );
    println!(
        "  User:\n    {}",
        msgs[1].content.chars().take(100).collect::<String>()
    );

    Ok(())
}

/// 研究 - 学术
fn research_academic_demo() -> anyhow::Result<()> {
    let tmpl = prompt("research_academic");
    let mut vars = HashMap::new();
    vars.insert(
        "topic".to_string(),
        "深度学习在图像识别中的应用".to_string(),
    );
    vars.insert(
        "sources".to_string(),
        "arXiv 论文: 2103.00001, 2103.00002".to_string(),
    );

    let output = tmpl.render(&vars);
    println!(
        "  模板内容 (前200字):\n    {}",
        output.chars().take(200).collect::<String>()
    );

    Ok(())
}

/// 过滤 - 分类
fn filter_classify_demo() -> anyhow::Result<()> {
    let tmpl = prompt("filter_classify");
    let mut vars = HashMap::new();
    vars.insert(
        "categories".to_string(),
        "科技, 娱乐, 体育, 财经".to_string(),
    );
    vars.insert("content".to_string(), "苹果发布新款iPhone手机".to_string());

    let output = tmpl.render(&vars);
    println!("  模板内容:\n{}", output);

    Ok(())
}

/// 英文 prompt
fn english_prompt_demo() -> anyhow::Result<()> {
    let tmpl = prompt("agent_simple_en");
    let mut vars = HashMap::new();
    vars.insert("input".to_string(), "What is AI?".to_string());

    let output = tmpl.render(&vars);
    println!("  使用英文版:");
    println!("    {}", output);

    Ok(())
}
