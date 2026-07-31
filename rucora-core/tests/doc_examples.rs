//! 文档示例集成测试
//!
//! 将 `Agent` trait 文档中的示例转换为实际运行验证的集成测试，
//! 确保文档中的用法始终与代码保持一致（对应 improvements 6.2）。

use rucora_core::agent::{Agent, AgentContext, AgentDecision, AgentInput};
use rucora_core::provider::types::{ChatMessage, ChatRequest};
use async_trait::async_trait;
use futures_util::StreamExt;

/// 纯推理 Agent（无 LLM/工具调用）。
///
/// 对应 `Agent::run()` 文档中的 EchoAgent 示例。
struct EchoAgent;

#[async_trait]
impl Agent for EchoAgent {
    async fn think(&self, context: &AgentContext) -> AgentDecision {
        let value = serde_json::json!({"content": context.input.text()});
        AgentDecision::Return(value)
    }

    fn name(&self) -> &str {
        "echo"
    }
}

/// 有限步数纯推理 Agent（用于测试 max_steps 上限）。
struct StepAgent {
    /// 返回 `ThinkAgain` 的次数。
    steps: usize,
}

#[async_trait]
impl Agent for StepAgent {
    async fn think(&self, context: &AgentContext) -> AgentDecision {
        if context.step >= self.steps {
            AgentDecision::Return(serde_json::json!({"content": "done"}))
        } else {
            AgentDecision::ThinkAgain
        }
    }

    fn name(&self) -> &str {
        "step"
    }
}

/// 使用 MockProvider 的流式 Agent（用于测试 run_stream 默认实现）。
struct StreamAgent;

#[async_trait]
impl Agent for StreamAgent {
    async fn think(&self, _context: &AgentContext) -> AgentDecision {
        AgentDecision::chat(ChatRequest::from_user_text("你好"))
    }

    fn name(&self) -> &str {
        "stream"
    }
}

// ===== Agent::run() 纯推理示例 =====

#[tokio::test]
async fn test_doc_echo_agent_run() {
    let output = EchoAgent.run(AgentInput::new("你好").unwrap()).await.unwrap();
    assert_eq!(output.text().unwrap(), "你好");
}

#[tokio::test]
async fn test_doc_agent_input_new() {
    let input = AgentInput::new("hello").unwrap();
    assert_eq!(input.text(), "hello");
}

// ===== Agent::run_with_timeout() 示例 =====

#[tokio::test]
async fn test_doc_run_with_timeout_success() {
    use std::time::Duration;

    let output = EchoAgent
        .run_with_timeout(AgentInput::new("测试超时").unwrap(), Duration::from_secs(30))
        .await
        .unwrap();
    assert_eq!(output.text().unwrap(), "测试超时");
}

// ===== Agent::run_batch() 示例 =====

#[tokio::test]
async fn test_doc_run_batch() {
    let agent = std::sync::Arc::new(EchoAgent);
    let inputs = vec![
        AgentInput::new("Hello").unwrap(),
        AgentInput::new("World").unwrap(),
    ];
    let outputs = agent.run_batch(inputs, 4).await;
    assert_eq!(outputs.len(), 2);
    for result in &outputs {
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_doc_run_batch_respects_concurrency() {
    let agent = std::sync::Arc::new(StepAgent { steps: 5 });
    let inputs = (0..8)
        .map(|i| AgentInput::new(format!("input-{i}")).unwrap())
        .collect::<Vec<_>>();
    let outputs = agent.run_batch(inputs, 2).await;
    assert_eq!(outputs.len(), 8);
    for result in &outputs {
        let output = result.as_ref().unwrap();
        assert_eq!(output.text().unwrap(), "done");
    }
}

// ===== Agent::run_stream() 默认实现示例 =====

#[tokio::test]
async fn test_doc_run_stream_default_yields_error() {
    // 纯推理 Agent 的默认 run_stream 应产生一个错误事件
    let mut stream = EchoAgent.run_stream(AgentInput::new("你好").unwrap());
    let event = stream.next().await.unwrap();
    assert!(event.is_err());
}

#[tokio::test]
async fn test_doc_run_stream_unsupported_agent() {
    // 需要 LLM 的 Agent 使用默认 run_stream 也会产生错误
    let mut stream = StreamAgent.run_stream(AgentInput::new("你好").unwrap());
    let event = stream.next().await.unwrap();
    assert!(event.is_err());
}

// ===== AgentDecision 构造器示例 =====

#[tokio::test]
async fn test_agent_decision_builders() {
    let request = ChatRequest::new(vec![ChatMessage::user("你好")]);

    let chat = AgentDecision::chat(request.clone());
    match &chat {
        AgentDecision::Chat {
            requests,
            max_concurrency,
            mode,
        } => {
            assert_eq!(requests.len(), 1);
            assert_eq!(*max_concurrency, 1);
            assert!(matches!(mode, rucora_core::agent::ChatMode::Single));
        }
        _ => panic!("应为 Chat 变体"),
    }

    let map_all = AgentDecision::map_all(vec![request.clone(), request.clone()], 2);
    match &map_all {
        AgentDecision::Chat {
            requests,
            max_concurrency,
            mode,
        } => {
            assert_eq!(requests.len(), 2);
            assert_eq!(*max_concurrency, 2);
            assert!(matches!(mode, rucora_core::agent::ChatMode::MapAll));
        }
        _ => panic!("应为 Chat 变体"),
    }

    let reduce = AgentDecision::reduce(request);
    match &reduce {
        AgentDecision::Chat {
            requests,
            max_concurrency,
            mode,
        } => {
            assert_eq!(requests.len(), 1);
            assert_eq!(*max_concurrency, 1);
            assert!(matches!(mode, rucora_core::agent::ChatMode::Reduce));
        }
        _ => panic!("应为 Chat 变体"),
    }
}

// ===== AgentOutput 文本访问器示例 =====

#[tokio::test]
async fn test_agent_output_text_accessors() {
    let output = EchoAgent.run(AgentInput::new("abc").unwrap()).await.unwrap();
    assert_eq!(output.text().unwrap(), "abc");
    assert_eq!(output.text_unwrap(), "abc");
    assert_eq!(output.text_or("default"), "abc");
}

// ===== max_steps 限制示例 =====

#[tokio::test]
async fn test_agent_run_respects_max_steps() {
    use rucora_core::agent::AgentError;

    // think 始终返回 ThinkAgain，应触发 MaxStepsExceeded（默认 20）
    let agent = StepAgent { steps: usize::MAX };
    let result = agent.run(AgentInput::new("loop").unwrap()).await;
    assert!(matches!(result, Err(AgentError::MaxStepsExceeded { .. })));
}
