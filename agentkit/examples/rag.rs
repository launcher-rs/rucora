//! RAG (Retrieval-Augmented Generation) 使用示例
//!
//! 展示如何结合 Embedding、VectorStore 和 Retrieval 实现完整的 RAG 功能
//!
//! # 运行方式
//!
//! ```bash
//! # 使用 OpenAI Embedding
//! export OPENAI_API_KEY=sk-your-key
//! export CHROMA_URL=http://localhost:8000
//!
//! # 或使用 Ollama Embedding（本地）
//! export OLLAMA_BASE_URL=http://localhost:11434
//!
//! cargo run --example 09_rag -p agentkit
//! ```

use agentkit::embed::OllamaEmbedding;
use agentkit::prelude::*;
use agentkit::provider::OpenAiProvider;
use agentkit::agent::DefaultAgent;
use agentkit::rag::{chunk_text, index_chunks, retrieve, Citation};
use agentkit::retrieval::{ChromaVectorStore, InMemoryVectorStore};
use agentkit_core::embed::EmbeddingProvider;
use agentkit_core::retrieval::{VectorStore, VectorQuery};
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

    info!("╔═══════════════════════════════════════════════════════════╗");
    info!("║         AgentKit RAG 使用示例                             ║");
    info!("╚═══════════════════════════════════════════════════════════╝\n");

    // 示例 1: 使用 InMemoryVectorStore 演示 RAG 流程
    info!("=== 示例 1: InMemoryVectorStore (内存向量存储) ===\n");
    test_in_memory_rag().await?;

    // 示例 2: 使用 ChromaVectorStore (生产环境)
    info!("\n=== 示例 2: ChromaVectorStore (生产环境) ===\n");
    test_chroma_rag().await?;

    // 示例 3: RAG + Agent 结合使用
    info!("\n=== 示例 3: RAG + Agent 结合使用 ===\n");
    test_rag_with_agent().await?;

    // 示例 4: 文档管理和检索
    info!("\n=== 示例 4: 文档管理和检索 ===\n");
    test_document_management().await?;

    info!("\n=== 所有示例完成 ===");

    Ok(())
}

/// 示例 1: 使用 InMemoryVectorStore 演示 RAG 流程
async fn test_in_memory_rag() -> anyhow::Result<()> {
    info!("1. 创建 Embedding Provider 和向量存储...");

    // 使用 Ollama Embedding（本地）
    let embedder = OllamaEmbedding::from_env();
    info!("✓ Embedding Provider: Ollama");

    // 使用内存向量存储
    let store = InMemoryVectorStore::new();
    info!("✓ 向量存储：InMemoryVectorStore\n");

    // 准备文档
    info!("2. 准备文档...");
    let documents = vec![
        ("doc1", "Rust 是一种系统编程语言，注重安全性和性能。它通过所有权系统保证内存安全。"),
        ("doc2", "人工智能（AI）是计算机科学的一个分支，致力于创建能够执行需要人类智能的任务的系统。"),
        ("doc3", "机器学习是人工智能的一个子集，使用统计技术让计算机从数据中学习。"),
        ("doc4", "深度学习是机器学习的一个分支，使用多层神经网络来模拟人类学习过程。"),
        ("doc5", "自然语言处理（NLP）是 AI 的一个领域，专注于计算机与人类语言之间的交互。"),
    ];
    info!("✓ 准备了 {} 个文档\n", documents.len());

    // 索引文档
    info!("3. 索引文档（分块 + 嵌入 + 存储）...");
    for (doc_id, text) in &documents {
        // 分块
        let chunks = chunk_text(*doc_id, *text, 500, 50);
        
        // 嵌入并存储
        index_chunks(&embedder, &store, &chunks).await?;
        
        info!("   ✓ 索引文档：{} ({} 个块)", doc_id, chunks.len());
    }
    info!("✓ 文档索引完成\n");

    // 检索测试
    info!("4. 检索测试...\n");

    let queries = vec![
        "Rust 语言的特点是什么？",
        "人工智能和机器学习有什么区别？",
        "什么是自然语言处理？",
    ];

    for query in queries {
        info!("查询：{}", query);
        
        let citations = retrieve(&embedder, &store, query, 3).await?;
        
        info!("检索到 {} 个相关片段:", citations.len());
        for (i, cite) in citations.iter().enumerate() {
            info!("  {}. [得分：{:.2}] {}", 
                i + 1, 
                cite.score, 
                cite.text.as_deref().unwrap_or("").chars().take(50).collect::<String>()
            );
        }
        info!("");
    }

    Ok(())
}

/// 示例 2: 使用 ChromaVectorStore (生产环境)
async fn test_chroma_rag() -> anyhow::Result<()> {
    info!("1. 创建 Chroma 向量存储...");

    // 检查 Chroma 是否可用
    match ChromaVectorStore::from_env() {
        Ok(store) => {
            info!("✓ ChromaVectorStore 已连接");
            
            // 创建 Embedding Provider
            let embedder = OllamaEmbedding::from_env();
            info!("✓ Embedding Provider: Ollama\n");

            // 创建测试集合
            let collection_name = "rag_test_collection";
            
            info!("2. 创建测试集合...");
            // 注意：实际使用中需要先创建 collection
            // 这里仅做演示
            info!("   提示：需要先创建 Chroma collection: {}\n", collection_name);

            // 检索示例
            info!("3. 检索示例...");
            let query = "什么是机器学习？";
            info!("查询：{}", query);
            
            match retrieve(&embedder, &store, query, 5).await {
                Ok(citations) => {
                    info!("检索到 {} 个相关片段", citations.len());
                    for (i, cite) in citations.iter().take(3).enumerate() {
                        info!("  {}. [得分：{:.2}] {}", 
                            i + 1, 
                            cite.score, 
                            cite.text.as_deref().unwrap_or("").chars().take(50).collect::<String>()
                        );
                    }
                }
                Err(e) => {
                    info!("⚠ 检索失败（可能 collection 不存在）：{}", e);
                }
            }
        }
        Err(e) => {
            info!("⚠ Chroma 不可用：{}", e);
            info!("   提示：请确保 Chroma 服务正在运行 (默认端口 8000)");
        }
    }

    Ok(())
}

/// 示例 3: RAG + Agent 结合使用
async fn test_rag_with_agent() -> anyhow::Result<()> {
    info!("1. 准备知识库...");

    // 创建 Embedding 和向量存储
    let embedder = OllamaEmbedding::from_env();
    let store = InMemoryVectorStore::new();

    // 准备专业知识库
    let knowledge_base = vec![
        ("rust_intro", "Rust 是一门系统编程语言，专注于安全性、并发性和性能。它适用于构建可靠且高效的软件。"),
        ("rust_ownership", "Rust 的所有权系统是其核心特性。每个值都有一个所有者，当所有者离开作用域时，值会被丢弃。"),
        ("rust_borrowing", "Rust 的借用系统允许你引用数据而不获取所有权。使用 & 符号创建引用。"),
        ("rust_lifetimes", "Rust 的生命周期确保引用在使用期间始终有效。它们是编译时检查的。"),
        ("rust_concurrency", "Rust 的类型系统保证了并发安全。'无数据竞争' 是其核心保证之一。"),
    ];

    // 索引知识库
    for (doc_id, text) in &knowledge_base {
        let chunks = chunk_text(*doc_id, *text, 500, 50);
        index_chunks(&embedder, &store, &chunks).await?;
    }
    info!("✓ 知识库已建立 ({} 个文档)\n", knowledge_base.len());

    // 创建 Agent
    info!("2. 创建 RAG Agent...");
    let provider = OpenAiProvider::from_env()?;
    let model = "qwen3.5:9b";
    
    let agent = DefaultAgent::builder()
        .provider(provider)
        .model(model)
        .system_prompt(
            "你是一个 Rust 编程专家。请根据提供的上下文信息回答问题。
            如果上下文中有相关信息，请引用并详细解释。
            如果上下文中没有相关信息，请说明你不知道。"
        )
        .build();
    
    info!("✓ Agent 已创建 (模型：{})\n", model);

    // RAG 问答
    info!("3. RAG 问答测试...\n");

    let questions = vec![
        "Rust 的所有权系统是什么？",
        "Rust 如何保证内存安全？",
        "什么是 Rust 的生命周期？",
    ];

    for question in questions {
        info!("问题：{}", question);
        
        // 1. 检索相关上下文
        let citations = retrieve(&embedder, &store, question, 3).await?;
        
        // 2. 构建增强提示词
        let context = build_context(&citations);
        let enhanced_prompt = format!(
            "请根据以下上下文回答问题：\n\n{}\n\n问题：{}\n\n回答：",
            context, question
        );

        // 3. 使用 Agent 生成答案
        let input = AgentInput::new(enhanced_prompt);
        match agent.run(input).await {
            Ok(output) => {
                if let Some(answer) = output.text() {
                    info!("回答：{}", answer.chars().take(100).collect::<String>());
                }
            }
            Err(e) => {
                info!("✗ 错误：{}", e);
            }
        }

        // 4. 显示引用的来源
        if !citations.is_empty() {
            info!("参考来源:");
            for (i, cite) in citations.iter().enumerate() {
                info!("  {}. {}", i + 1, cite.render());
            }
        }
        info!("");
    }

    Ok(())
}

/// 示例 4: 文档管理和检索
async fn test_document_management() -> anyhow::Result<()> {
    info!("1. 创建文档索引...");

    let embedder = OllamaEmbedding::from_env();
    let store = InMemoryVectorStore::new();

    // 模拟技术文档库
    let tech_docs = vec![
        ("microservices", "微服务架构是一种将单一应用程序开发为一组小型服务的方法。每个服务运行在自己的进程中，并通过轻量级机制（通常是 HTTP）进行通信。"),
        ("containers", "容器是一种轻量级的虚拟化技术，可以打包代码及其所有依赖项，确保软件在任何环境中都能一致运行。Docker 是最流行的容器平台。"),
        ("kubernetes", "Kubernetes 是一个开源的容器编排平台，用于自动化部署、扩展和管理容器化应用程序。它最初由 Google 设计。"),
        ("devops", "DevOps 是开发（Development）和运维（Operations）的结合，旨在缩短系统开发周期并提供高质量的持续交付。"),
        ("ci_cd", "CI/CD（持续集成/持续交付）是一种通过自动化构建、测试和部署来频繁交付代码变更的软件开发实践。"),
    ];

    // 批量索引
    info!("2. 批量索引文档...");
    let mut total_chunks = 0;
    for (doc_id, text) in &tech_docs {
        let chunks = chunk_text(*doc_id, *text, 500, 50);
        total_chunks += chunks.len();
        index_chunks(&embedder, &store, &chunks).await?;
    }
    info!("✓ 索引完成：{} 个文档，{} 个块\n", tech_docs.len(), total_chunks);

    // 高级检索
    info!("3. 高级检索功能...\n");

    // 3.1 相似度阈值过滤
    info!("3.1 相似度阈值过滤");
    let query = "容器编排";
    info!("查询：{}", query);
    
    // 手动检索并过滤
    let query_vector = embedder.embed(query).await?;
    let results: Vec<_> = store.search(
        VectorQuery::new(query_vector)
            .with_top_k(10)
    ).await?;
    
    info!("全部结果：{} 个", results.len());
    info!("高相似度结果（>0.7）:");
    for result in results.iter().filter(|r| r.score > 0.7) {
        info!("  - [得分：{:.2}] {}", 
            result.score,
            result.text.as_deref().unwrap_or("").chars().take(40).collect::<String>()
        );
    }
    info!("");

    // 3.2 多文档检索
    info!("3.2 多主题检索");
    let topics = vec!["微服务", "DevOps", "CI/CD"];
    
    for topic in topics {
        let citations = retrieve(&embedder, &store, topic, 2).await?;
        info!("主题 '{}': {} 个相关片段", topic, citations.len());
    }
    info!("");

    // 3.3 文档统计
    info!("3.3 文档统计");
    info!("总文档数：{}", tech_docs.len());
    info!("总块数：{}", total_chunks);
    info!("平均每个文档：{:.1} 个块", total_chunks as f64 / tech_docs.len() as f64);

    Ok(())
}

/// 构建上下文文本
fn build_context(citations: &[Citation]) -> String {
    citations
        .iter()
        .map(|cite| {
            cite.text
                .as_deref()
                .unwrap_or("")
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}
