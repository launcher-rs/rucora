//! Runtime 基本功能测试

use agentkit_core::{
    agent::AgentInput,
    error::ProviderError,
    provider::types::{ChatMessage, ChatRequest, ChatResponse, Role},
    provider::LlmProvider,
    runtime::Runtime,
};
use agentkit_runtime::{DefaultRuntime, ToolRegistry};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

/// 测试 Provider - 返回固定回复
#[derive(Clone)]
struct TestProvider {
    call_count: Arc<Mutex<u32>>,
}

#[async_trait]
impl LlmProvider for TestProvider {
    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let mut count = self.call_count.lock().unwrap();
        *count += 1;

        Ok(ChatResponse {
            message: ChatMessage {
                role: Role::Assistant,
                content: format!("回复 (第{}次调用)", *count),
                name: None,
            },
            tool_calls: vec![],
            usage: None,
            finish_reason: None,
        })
    }

    fn stream_chat(
        &self,
        _request: ChatRequest,
    ) -> Result<
        futures_util::stream::BoxStream<'static, Result<agentkit_core::provider::types::ChatStreamChunk, ProviderError>>,
        ProviderError,
    > {
        // 简化测试，不支持流式
        Err(ProviderError::Message("流式不支持".to_string()))
    }
}

#[tokio::test]
async fn test_basic_runtime() {
    let provider = TestProvider {
        call_count: Arc::new(Mutex::new(0)),
    };

    let runtime = DefaultRuntime::new(
        Arc::new(provider.clone()),
        ToolRegistry::new(),
    ).with_system_prompt("你是有用的助手");

    let input = AgentInput::new("你好");
    let output = runtime.run(input).await.expect("运行失败");

    // 验证输出包含回复内容
    assert!(output.text().is_some());
    assert!(output.value.get("content").is_some());
}

#[tokio::test]
async fn test_agent_input_builder() {
    let provider = TestProvider {
        call_count: Arc::new(Mutex::new(0)),
    };

    let runtime = DefaultRuntime::new(
        Arc::new(provider.clone()),
        ToolRegistry::new(),
    );

    // 测试 builder 模式
    let input = AgentInput::builder("帮我查询天气")
        .with_context("location", "北京")
        .build();

    let output = runtime.run(input).await.expect("运行失败");

    assert!(output.text().is_some());
}

#[tokio::test]
async fn test_multiple_calls() {
    let provider = TestProvider {
        call_count: Arc::new(Mutex::new(0)),
    };

    let runtime = DefaultRuntime::new(
        Arc::new(provider.clone()),
        ToolRegistry::new(),
    );

    // 多次调用验证计数器
    for i in 1..=3 {
        let input = AgentInput::new(format!("消息{}", i));
        let _output = runtime.run(input).await.expect("运行失败");
        
        let count = *provider.call_count.lock().unwrap();
        assert_eq!(count, i, "调用次数应该递增");
    }
}

#[tokio::test]
async fn test_output_structure() {
    let provider = TestProvider {
        call_count: Arc::new(Mutex::new(0)),
    };

    let runtime = DefaultRuntime::new(
        Arc::new(provider.clone()),
        ToolRegistry::new(),
    );

    let input = AgentInput::new("测试");
    let output = runtime.run(input).await.expect("运行失败");

    // 验证输出结构
    assert!(output.value.is_object());
    assert!(output.value.get("content").is_some());
    assert_eq!(output.message_count(), 1);
    assert_eq!(output.tool_call_count(), 0);
}
