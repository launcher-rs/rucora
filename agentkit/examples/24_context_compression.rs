//! AgentKit 分层上下文压缩引擎示例
//!
//! 展示如何使用分层压缩引擎智能压缩对话上下文，
//! 保护关键信息并避免上下文溢出。
//!
//! ## 运行方法
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! cargo run --example 24_context_compression
//! ```
//!
//! ## 功能演示
//!
//! 1. **分层保护** - 头部/尾部分离保护
//! 2. **结构化摘要** - 使用模板提取关键信息
//! 3. **迭代更新** - 后续压缩时更新先前摘要
//! 4. **成本控制** - 避免过度压缩导致信息丢失
//!
//! ## 注意事项
//!
//! 分层压缩引擎的内部方法（如 split_messages）是私有的，
//! 用户应通过 `compress()` 方法使用完整压缩流程。

use agentkit::{CompressionConfig, LayeredCompressor};
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

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
    info!("║   AgentKit 分层上下文压缩引擎示例     ║");
    info!("╚════════════════════════════════════════╝\n");

    // ═══════════════════════════════════════════════════════════
    // 示例 1: 压缩配置说明
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 1: 压缩配置说明");
    info!("═══════════════════════════════════════\n");

    info!("CompressionConfig 配置项:\n");

    info!("  strategy: 压缩策略");
    info!("    - Aggressive: 激进压缩（适合长对话）");
    info!("    - Balanced: 平衡压缩（默认）");
    info!("    - Conservative: 保守压缩（适合短对话）\n");

    info!("  protect_head_count: 保护头部消息数量");
    info!("    - 这些消息永远不会被压缩\n");

    info!("  protect_tail_tokens: 保护尾部消息 Token 数");
    info!("    - 最近的消息保留这么多 token\n");

    info!("  compression_threshold: 触发压缩的使用率阈值");
    info!("    - 0.85 = 85% 时触发\n");

    info!("  target_usage_ratio: 压缩后的目标使用率");
    info!("    - 0.60 = 压缩到 60%\n");

    info!("  max_iterations: 最大压缩迭代次数");
    info!("    - 防止过度压缩\n");

    info!("  summary_cooldown_seconds: 摘要失败冷却期");
    info!("    - 防止频繁重试\n");

    // ═══════════════════════════════════════════════════════════
    // 示例 2: 使用不同的压缩策略
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 2: 使用不同的压缩策略");
    info!("═══════════════════════════════════════\n");

    let strategies = vec![
        (
            "Aggressive (激进)",
            CompressionConfig::aggressive(),
        ),
        ("Balanced (平衡)", CompressionConfig::default()),
        (
            "Conservative (保守)",
            CompressionConfig::conservative(),
        ),
    ];

    for (name, config) in &strategies {
        info!("策略: {}", name);
        info!(
            "  protect_head_count: {}",
            config.protect_head_count
        );
        info!(
            "  protect_tail_tokens: {}",
            config.protect_tail_tokens
        );
        info!(
            "  compression_threshold: {:.0}%",
            config.compression_threshold * 100.0
        );
        info!(
            "  target_usage_ratio: {:.0}%",
            config.target_usage_ratio * 100.0
        );
        info!("");
    }

    // ═══════════════════════════════════════════════════════════
    // 示例 3: 判断是否需要压缩
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 3: 判断是否需要压缩");
    info!("═══════════════════════════════════════\n");

    let engine = LayeredCompressor::default_engine();

    let test_cases = vec![
        (10_000, 128_000, "10K/128K (7.8%)"),
        (50_000, 128_000, "50K/128K (39.1%)"),
        (100_000, 128_000, "100K/128K (78.1%)"),
        (110_000, 128_000, "110K/128K (85.9%)"),
        (120_000, 128_000, "120K/128K (93.8%)"),
        (10_000, 8_192, "10K/8K (122.1%)"),
    ];

    info!("GPT-4o 上下文窗口: 128K tokens");
    info!("GPT-4 上下文窗口: 8K tokens\n");

    for (tokens, window, desc) in &test_cases {
        let should = engine.should_compress(*tokens, *window);
        let usage = *tokens as f64 / *window as f64 * 100.0;
        info!(
            "  {}: {:.1}% -> 压缩: {}",
            desc, usage, should
        );
    }
    info!("");

    // ═══════════════════════════════════════════════════════════
    // 示例 4: 消息分层概念
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 4: 消息分层概念");
    info!("═══════════════════════════════════════\n");

    info!("压缩引擎会将消息分为三层:\n");

    info!("1. 头部 (Head):");
    info!("   - 包含系统提示词和首次交互");
    info!("   - 默认保护前 3 条消息");
    info!("   - 这些消息永远不会被压缩\n");

    info!("2. 中间 (Middle):");
    info!("   - 对话的中间部分");
    info!("   - 这是会被压缩的部分");
    info!("   - 使用结构化摘要替换\n");

    info!("3. 尾部 (Tail):");
    info!("   - 最近的对话内容");
    info!("   - 默认保护最近 20K tokens");
    info!("   - 保持对话的连续性\n");

    info!("分层的好处:");
    info!("  - 保护关键信息（系统提示、用户偏好）");
    info!("  - 保留最近上下文（避免断裂）");
    info!("  - 只压缩中间部分（最大化效率）\n");

    // ═══════════════════════════════════════════════════════════
    // 示例 5: 结构化摘要模板
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 5: 结构化摘要模板");
    info!("═══════════════════════════════════════\n");

    info!("压缩引擎使用以下结构化摘要模板:\n");

    let template_sections = vec![
        ("Goal", "用户试图完成什么"),
        ("Constraints & Preferences", "用户偏好、编码风格"),
        ("Progress", "Done / In Progress / Blocked"),
        ("Key Decisions", "重要技术决策"),
        ("Resolved Questions", "已回答的问题"),
        ("Pending User Asks", "未回答的问题"),
        ("Relevant Files", "读取/修改/创建的文件"),
        ("Remaining Work", "剩余工作"),
        ("Critical Context", "不能丢失的具体值"),
        ("Tools & Patterns", "使用过的工具及有效用法"),
    ];

    for (section, desc) in &template_sections {
        info!("  ## {} - {}", section, desc);
    }
    info!("");

    info!("结构化摘要的优势:");
    info!("  1. 信息密度高，保留关键内容");
    info!("  2. 结构一致，便于模型理解");
    info!("  3. 支持迭代更新，保持信息新鲜");
    info!("  4. 防止信息丢失，特别是决策和问题\n");

    // ═══════════════════════════════════════════════════════════
    // 示例 6: 迭代压缩
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 6: 迭代压缩机制");
    info!("═══════════════════════════════════════\n");

    info!("迭代压缩工作流程:\n");

    info!("首次压缩:");
    info!("  1. 修剪旧工具结果");
    info!("  2. 分离头部/中间/尾部");
    info!("  3. 使用结构化模板生成摘要");
    info!("  4. 替换中间消息为摘要\n");

    info!("后续压缩:");
    info!("  1. 保留先前摘要");
    info!("  2. 添加新的对话内容");
    info!("  3. 更新摘要以反映新进展");
    info!("  4. 避免信息丢失\n");

    info!("冷却期机制:");
    info!("  - 默认 600 秒 (10 分钟)");
    info!("  - 防止频繁压缩导致信息不稳定");
    info!("  - 可配置调整\n");

    // ═══════════════════════════════════════════════════════════
    // 示例 7: 工具结果修剪
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 7: 工具结果修剪");
    info!("═══════════════════════════════════════\n");

    info!("不同策略保留的工具结果数量:\n");

    let aggressive_engine = LayeredCompressor::new(CompressionConfig::aggressive());
    let balanced_engine = LayeredCompressor::default_engine();
    let conservative_engine = LayeredCompressor::new(CompressionConfig::conservative());

    info!("  Aggressive: 最多保留 2 个工具结果");
    info!("  Balanced: 最多保留 4 个工具结果");
    info!("  Conservative: 最多保留 6 个工具结果\n");

    info!("工具结果修剪的优势:");
    info!("  - 工具结果通常占用大量 token");
    info!("  - 旧的工具结果对当前任务价值低");
    info!("  - 保留最近的结果即可\n");

    // ═══════════════════════════════════════════════════════════
    // 示例 8: 实际应用建议
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 8: 实际应用建议");
    info!("═══════════════════════════════════════\n");

    info!("与 AgentKit 集成建议:\n");

    info!("1. 在 Agent 运行循环中检查上下文:");
    info!("   ```rust");
    info!("   if compressor.should_compress(tokens, window) {{");
    info!("       messages = compressor.compress(&provider, messages, window).await?;");
    info!("   }}");
    info!("   ```\n");

    info!("2. 根据对话长度选择策略:");
    info!("   - 短对话 (< 50 轮): Conservative");
    info!("   - 中对话 (50-200 轮): Balanced");
    info!("   - 长对话 (> 200 轮): Aggressive\n");

    info!("3. 监控压缩效果:");
    info!("   - 记录压缩前后的 token 数");
    info!("   - 监控压缩率");
    info!("   - 评估信息保留质量\n");

    info!("4. 结合 Usage 追踪:");
    info!("   - 使用 output.usage() 获取 token 统计");
    info!("   - 根据实际使用量调整压缩阈值\n");

    // ═══════════════════════════════════════════════════════════
    // 总结
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例完成！");
    info!("═══════════════════════════════════════\n");

    info!("📝 分层上下文压缩总结:\n");

    info!("1. 核心优势:");
    info!("   - 智能分层保护关键信息");
    info!("   - 结构化摘要保持信息密度");
    info!("   - 迭代更新避免信息丢失\n");

    info!("2. 三种策略:");
    info!("   - Aggressive: 最大压缩比，适合长对话");
    info!("   - Balanced: 平衡压缩比和信息保留");
    info!("   - Conservative: 最小压缩，保护更多信息\n");

    info!("3. 关键参数:");
    info!("   - protect_head_count: 保护头部消息");
    info!("   - protect_tail_tokens: 保护尾部 token");
    info!("   - compression_threshold: 触发阈值\n");

    info!("4. 与 Hermes Agent 的对比:");
    info!("   - 相同的分层保护思想");
    info!("   - 相同的结构化摘要模板");
    info!("   - Rust 实现带来类型安全和性能优势\n");

    info!("5. 最佳实践:");
    info!("   - 根据对话长度选择策略");
    info!("   - 监控压缩效果");
    info!("   - 结合 Usage 追踪调整参数");
    info!("   - 定期评估信息保留质量\n");

    Ok(())
}
