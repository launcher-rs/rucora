//! AgentKit RAG（检索增强生成）示例
//!
//! 展示 RAG（Retrieval-Augmented Generation）的完整流程。
//!
//! ## 运行方法
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! cargo run --example 07_rag
//! ```
//!
//! ## 功能演示
//!
//! 1. **文档分块** - 将长文档分割成小块
//! 2. **向量化** - 将文本转换为向量
//! 3. **索引** - 将向量存储到向量数据库
//! 4. **检索** - 根据查询找到相关文档
//! 5. **增强生成** - 结合检索结果生成回答

use agentkit::rag::{chunk_text, index_chunks, index_text, retrieve};
use agentkit_core::retrieval::VectorStore;
use agentkit_embed::openai::OpenAiEmbeddingProvider;
use agentkit_retrieval::in_memory::InMemoryVectorStore;
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
    info!("║   AgentKit RAG（检索增强生成）示例    ║");
    info!("╚════════════════════════════════════════╝\n");

    // 检查配置
    if std::env::var("OPENAI_API_KEY").is_err() && std::env::var("OPENAI_BASE_URL").is_err() {
        info!("⚠ 未设置 API 配置");
        info!("   使用 OpenAI: export OPENAI_API_KEY=sk-your-key");
        info!("   使用 Ollama: export OPENAI_BASE_URL=http://localhost:11434");
        info!("\n注意：RAG 需要 Embedding Provider 来向量化文本");
        return Ok(());
    }

    // ═══════════════════════════════════════════════════════════
    // RAG 工作流程说明
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("RAG 工作流程:");
    info!("═══════════════════════════════════════");
    info!("1. 文档分块（Chunking）: 将长文档分割成小块");
    info!("2. 向量化（Embedding）: 将文本转换为向量");
    info!("3. 索引（Indexing）: 将向量存储到向量数据库");
    info!("4. 检索（Retrieval）: 根据查询找到相关文档");
    info!("5. 增强生成（Generation）: 结合检索结果生成回答");
    info!("═══════════════════════════════════════\n");

    // ═══════════════════════════════════════════════════════════
    // 示例 1: 文档分块
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 1: 文档分块");
    info!("═══════════════════════════════════════\n");

    let long_document = "
    Rust 是一门系统编程语言，专注于安全、并发和性能。它由 Mozilla 研究团队开发，
    于 2010 年首次发布。Rust 的最大特点是所有权（Ownership）系统，它能够在编译时
    保证内存安全，而不需要垃圾回收机制。

    Rust 的核心概念包括：
    1. 所有权（Ownership）：每个值都有一个所有者，当所有者离开作用域时，值会被丢弃。
    2. 借用（Borrowing）：通过引用来访问值，而不获取所有权。
    3. 生命周期（Lifetimes）：确保引用在有效期间内使用。
    4. 模式匹配（Pattern Matching）：强大的模式匹配能力。
    5. 零成本抽象（Zero-cost Abstractions）：高级抽象不带来运行时开销。

    Rust 的应用场景非常广泛：
    - 系统编程：操作系统、驱动程序、嵌入式系统
    - Web 后端：高性能 Web 服务器和 API
    - 命令行工具：快速、可靠的 CLI 工具
    - 网络服务：分布式系统、微服务架构
    - 游戏开发：游戏引擎、图形渲染
    - 区块链：智能合约、加密货币

    Rust 的生态系统正在快速增长，Cargo 包管理器提供了便捷的依赖管理和构建工具。
    crates.io 上有数以万计的开源库，涵盖了各种应用场景。
    ";

    info!("1.1 原始文档长度：{} 字符\n", long_document.len());

    info!("1.2 分块处理（每块 200 字符，重叠 30 字符）...");
    let chunks = chunk_text("rust_doc", long_document, 200, 30);
    info!("✓ 分块完成，共 {} 个块\n", chunks.len());

    for (i, chunk) in chunks.iter().enumerate() {
        info!("  块 {} [{}]:", i + 1, chunk.id);
        info!(
            "    内容：{}...\n",
            chunk.text.chars().take(50).collect::<String>()
        );
    }

    // ═══════════════════════════════════════════════════════════
    // 示例 2: 向量化和索引
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 2: 向量化和索引");
    info!("═══════════════════════════════════════\n");

    info!("2.1 创建向量存储...");
    let vector_store = InMemoryVectorStore::new();
    info!("✓ 向量存储创建成功\n");

    info!("2.2 创建 Embedding Provider...");
    let embedder = OpenAiEmbeddingProvider::from_env()?;
    info!("✓ Embedding Provider 创建成功\n");

    info!("2.3 索引分块...");
    index_chunks(&embedder, &vector_store, &chunks).await?;
    info!("✓ 索引完成\n");

    let count = vector_store.count().await?;
    info!("  向量存储中当前有 {} 个向量\n", count);

    // ═══════════════════════════════════════════════════════════
    // 示例 3: 完整 RAG 流程（索引 + 检索）
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 3: 完整 RAG 流程");
    info!("═══════════════════════════════════════\n");

    // 索引另一篇文档
    info!("3.1 索引新文档...");
    let python_doc = "
    Python 是一种高级编程语言，由 Guido van Rossum 于 1991 年创建。
    Python 的设计哲学强调代码可读性，使用缩进来表示代码块。
    
    Python 的特点：
    1. 简洁易读：语法简单，接近自然语言
    2. 解释型语言：不需要编译，直接运行
    3. 动态类型：变量类型在运行时确定
    4. 丰富的库：标准库和第三方库非常庞大
    5. 跨平台：可在 Windows、Linux、macOS 上运行
    
    Python 的应用领域：
    - 数据科学：数据分析、机器学习、人工智能
    - Web 开发：Django、Flask 等框架
    - 自动化：脚本编写、任务自动化
    - 科学计算：NumPy、SciPy、Pandas
    - 网络爬虫：Scrapy、BeautifulSoup
    ";

    let python_chunks =
        index_text(&embedder, &vector_store, "python_doc", python_doc, 150, 20).await?;

    info!("✓ 已索引 Python 文档，共 {} 个块\n", python_chunks.len());

    // 检索
    info!("3.2 检索相关文档...");
    let queries = vec![
        ("Rust 的所有权是什么？", "Rust 相关"),
        ("Python 适合做什么？", "Python 相关"),
        ("系统编程语言有哪些特点？", "跨文档检索"),
    ];

    for (query, description) in queries {
        info!("  查询：\"{}\"（{}）", query, description);

        let citations = retrieve(&embedder, &vector_store, query, 3).await?;

        info!("  找到 {} 条相关引用:", citations.len());
        for (i, citation) in citations.iter().enumerate() {
            info!(
                "    {}. [{}] {} (相似度：{:.3})",
                i + 1,
                citation.render(),
                citation
                    .text
                    .as_ref()
                    .unwrap_or(&"无内容".to_string())
                    .chars()
                    .take(40)
                    .collect::<String>(),
                citation.score
            );
        }
        info!("");
    }

    // ═══════════════════════════════════════════════════════════
    // 示例 4: RAG 最佳实践
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 4: RAG 最佳实践");
    info!("═══════════════════════════════════════\n");

    info!("4.1 分块大小建议:");
    info!("  - 小文本（< 1000 字符）: 不需要分块");
    info!("  - 中等文本（1000-5000 字符）: 500 字符/块，50 字符重叠");
    info!("  - 大文本（> 5000 字符）: 1000 字符/块，100 字符重叠\n");

    info!("4.2 TopK 选择建议:");
    info!("  - 精确查询：top_k = 3-5");
    info!("  - 模糊查询：top_k = 5-10");
    info!("  - 探索性查询：top_k = 10-20\n");

    info!("4.3 性能优化建议:");
    info!("  - 使用缓存减少重复嵌入");
    info!("  - 批量嵌入优于单次嵌入");
    info!("  - 定期清理向量存储\n");

    // ═══════════════════════════════════════════════════════════
    // 示例 5: 结合 LLM 生成回答
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 5: RAG + LLM 生成回答");
    info!("═══════════════════════════════════════\n");

    use agentkit::agent::SimpleAgent;
    use agentkit::prelude::Agent;
    use agentkit::provider::OpenAiProvider;

    info!("5.1 创建 LLM Provider...");
    let model_name = std::env::var("MODEL_NAME").unwrap_or_else(|_| "gpt-4o-mini".to_string());
    let llm_provider = OpenAiProvider::from_env()?;
    info!("✓ LLM Provider 创建成功\n");

    info!("5.2 创建 Agent...");
    let agent = SimpleAgent::builder()
        .provider(llm_provider)
        .model(&model_name)
        .system_prompt(
            "你是一个基于检索结果的问答助手。请根据提供的上下文信息回答问题。\n\
             如果上下文中没有相关信息，请诚实地说明。\n\
             回答时请引用相关的来源。",
        )
        .build();
    info!("✓ Agent 创建成功\n");

    // 模拟 RAG 增强的查询
    let query = "Rust 的所有权系统是如何工作的？";
    info!("5.3 查询：\"{}\"", query);

    // 检索相关文档
    let citations = retrieve(&embedder, &vector_store, query, 3).await?;

    // 构建上下文
    let context = citations
        .iter()
        .map(|c| {
            format!(
                "[{}]: {}",
                c.render(),
                c.text.as_ref().unwrap_or(&"无内容".to_string())
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    info!("  检索到 {} 条相关文档", citations.len());
    info!("  上下文长度：{} 字符\n", context.len());

    // 构建增强提示词
    let enhanced_prompt = format!(
        "请根据以下上下文信息回答问题。\n\n\
         === 上下文 ===\n\
         {}\n\n\
         === 问题 ===\n\
         {}\n\n\
         请提供详细、准确的回答，并引用相关来源。",
        context, query
    );

    info!("5.4 生成回答...");
    let output = agent.run(enhanced_prompt.into()).await?;

    if let Some(text) = output.text() {
        info!("  回答：\n{}\n", text);
    }

    // ═══════════════════════════════════════════════════════════
    // 总结
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例完成！");
    info!("═══════════════════════════════════════\n");

    info!("📝 RAG 核心概念总结：\n");

    info!("1. 文档分块（Chunking）:");
    info!("   - 将长文档分割成适合向量化的小块");
    info!("   - 重叠分块保持上下文连续性\n");

    info!("2. 向量化（Embedding）:");
    info!("   - 将文本转换为数值向量");
    info!("   - 语义相似的文本向量距离更近\n");

    info!("3. 索引（Indexing）:");
    info!("   - 将向量存储到向量数据库");
    info!("   - 支持高效的相似度搜索\n");

    info!("4. 检索（Retrieval）:");
    info!("   - 将查询向量化");
    info!("   - 找到最相关的文档块\n");

    info!("5. 增强生成（Generation）:");
    info!("   - 将检索结果作为上下文");
    info!("   - LLM 基于上下文生成准确回答\n");

    info!("💡 使用建议:");
    info!("   - 选择合适的分块大小（通常 200-1000 字符）");
    info!("   - 设置合理的重叠（通常为分块大小的 10-20%）");
    info!("   - 根据查询类型调整 top_k");
    info!("   - 使用缓存优化性能\n");

    Ok(())
}
