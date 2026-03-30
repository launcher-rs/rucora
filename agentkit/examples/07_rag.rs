//! AgentKit RAG（检索增强生成）示例
//!
//! 展示 RAG 的基本概念和使用方法。
//!
//! ## 运行方法
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! cargo run --example 07_rag
//! ```

use agentkit::retrieval::InMemoryVectorStore;
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
    info!("║   AgentKit RAG 示例                   ║");
    info!("╚════════════════════════════════════════╝\n");

    info!("RAG（Retrieval-Augmented Generation）检索增强生成\n");

    info!("═══════════════════════════════════════");
    info!("RAG 工作流程:");
    info!("═══════════════════════════════════════");
    info!("1. 文档分块（Chunking）: 将长文档分割成小块");
    info!("2. 向量化（Embedding）: 将文本转换为向量");
    info!("3. 索引（Indexing）: 将向量存储到向量数据库");
    info!("4. 检索（Retrieval）: 根据查询找到相关文档");
    info!("5. 增强生成（Generation）: 结合检索结果生成回答");
    info!("═══════════════════════════════════════\n");

    info!("1. 创建向量存储...\n");

    let _vector_store = InMemoryVectorStore::new();

    info!("✓ 向量存储创建成功\n");

    info!("提示：完整的 RAG 功能需要:");
    info!("  - Embedding Provider（用于向量化）");
    info!("  - 文档加载器（用于加载文档）");
    info!("  - 分块策略（用于文档分块）");
    info!("  - LLM Provider（用于生成回答）\n");

    info!("示例完成！");
    info!("参考文档：docs/user_guide.md 中的 RAG 章节");

    Ok(())
}
