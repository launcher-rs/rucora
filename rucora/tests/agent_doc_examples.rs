//! 文档示例集成测试（rucora 聚合 crate）
//!
//! 将 `SimpleAgent`/`ToolAgent` 等文档中的构建器 + 运行示例转换为
//! 实际运行验证的集成测试，使用 `MockProvider` 替代真实 API
//! （对应 improvements 6.2）。

use rucora::agent::SimpleAgent;
use rucora::prelude::Agent;
use rucora_core::test_utils::MockProvider;

/// 对应 SimpleAgent 文档示例（改用 MockProvider）。
#[tokio::test]
async fn test_doc_simple_agent_run() {
    let agent = SimpleAgent::builder()
        .provider(MockProvider)
        .model("mock-model")
        .system_prompt("你是一个翻译助手")
        .temperature(0.3)
        .try_build()
        .unwrap();

    let output = agent.run("把'Hello'翻译成中文".into()).await.unwrap();
    assert_eq!(output.text().unwrap(), "Mock response");
}

/// 对应 SimpleAgent 文档示例（流式）。
#[tokio::test]
async fn test_doc_simple_agent_run_stream() {
    use futures_util::StreamExt;

    let agent = SimpleAgent::builder()
        .provider(MockProvider)
        .model("mock-model")
        .system_prompt("你好")
        .try_build()
        .unwrap();

    let mut stream = agent.run_stream("你好".into());
    while let Some(event) = stream.next().await {
        // MockProvider 无流式数据，流可能为空或产生完成事件，不 panic 即可
        let _ = event;
    }
}

/// 对应 SimpleAgent 文档示例（流式文本拼接）。
#[tokio::test]
async fn test_doc_simple_agent_run_stream_text() {
    let agent = SimpleAgent::builder()
        .provider(MockProvider)
        .model("mock-model")
        .try_build()
        .unwrap();

    // MockProvider 无流式数据，返回空字符串而非错误
    let text = agent.run_stream_text("你好").await.unwrap();
    assert_eq!(text, "");
}

/// SimpleAgent 一次调用直接返回结果（无工具、无循环）。
#[tokio::test]
async fn test_doc_simple_agent_single_call() {
    let agent = SimpleAgent::builder()
        .provider(MockProvider)
        .model("mock-model")
        .try_build()
        .unwrap();

    let output = agent.run("question".into()).await.unwrap();
    assert!(output.message_count() > 0);
    assert_eq!(output.tool_call_count(), 0);
    assert_eq!(output.text().unwrap(), "Mock response");
}
