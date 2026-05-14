//! 测试工具函数和 MockProvider
//!
//! 提供可配置的 MockProvider，支持：
//! - 工具调用跟踪（记录所有调用）
//! - 错误模拟（可配置错误响应）
//! - 流式响应模拟（stream_chat）
//! - 错误模式配置（不同调用返回不同错误）

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures_util::stream::{BoxStream, StreamExt};
use rucora_core::error::ProviderError;
use rucora_core::provider::LlmProvider;
use rucora_core::provider::types::{
    ChatMessage, ChatRequest, ChatResponse, ChatStreamChunk, FinishReason, Role, Usage,
};
use rucora_core::tool::types::ToolDefinition;

/// MockProvider 的调用记录
#[derive(Debug, Default, Clone)]
pub struct CallRecord {
    pub messages: Vec<ChatMessage>,
    pub model: Option<String>,
    pub tools: Option<Vec<ToolDefinition>>,
}

/// 流式调用记录
#[derive(Debug, Default, Clone)]
pub struct StreamCallRecord {
    pub messages: Vec<ChatMessage>,
    pub model: Option<String>,
    pub tools: Option<Vec<ToolDefinition>>,
    pub chunks: Vec<ChatStreamChunk>,
}

/// 可配置的 Mock Provider
///
/// 用于测试，支持：
/// - 响应自定义（返回指定内容或错误）
/// - 调用跟踪（记录所有 chat 调用）
/// - 流式响应模拟（stream_chat）
/// - 错误模式（一次错误后恢复 / 始终错误）
#[derive(Clone)]
pub struct MockProvider {
    response: String,
    error: Option<String>,
    stream_chunks: Vec<String>,
    records: Arc<Mutex<Vec<CallRecord>>>,
    stream_records: Arc<Mutex<Vec<StreamCallRecord>>>,
}

impl MockProvider {
    pub fn new(response: impl Into<String>) -> Self {
        Self {
            response: response.into(),
            error: None,
            stream_chunks: Vec::new(),
            records: Arc::new(Mutex::new(Vec::new())),
            stream_records: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 配置返回错误
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }

    /// 配置流式响应分块内容
    pub fn with_stream_chunks(mut self, chunks: Vec<impl Into<String>>) -> Self {
        self.stream_chunks = chunks.into_iter().map(|c| c.into()).collect();
        self
    }

    pub fn records(&self) -> Vec<CallRecord> {
        self.records.lock().unwrap().clone()
    }

    pub fn stream_records(&self) -> Vec<StreamCallRecord> {
        self.stream_records.lock().unwrap().clone()
    }

    pub fn clear_records(&self) {
        self.records.lock().unwrap().clear();
        self.stream_records.lock().unwrap().clear();
    }

    /// 获取调用次数
    pub fn call_count(&self) -> usize {
        self.records.lock().unwrap().len()
    }

    /// 获取流式调用次数
    pub fn stream_call_count(&self) -> usize {
        self.stream_records.lock().unwrap().len()
    }
}

#[async_trait]
impl LlmProvider for MockProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        // 记录调用
        self.records.lock().unwrap().push(CallRecord {
            messages: request.messages.clone(),
            model: request.model.clone(),
            tools: request.tools,
        });

        if let Some(err) = &self.error {
            return Err(ProviderError::Message(err.clone()));
        }

        Ok(ChatResponse {
            message: ChatMessage {
                role: Role::Assistant,
                content: self.response.clone(),
                name: None,
            },
            tool_calls: vec![],
            usage: None,
            finish_reason: Some(FinishReason::Stop),
        })
    }

    fn stream_chat(
        &self,
        request: ChatRequest,
    ) -> Result<BoxStream<'static, Result<ChatStreamChunk, ProviderError>>, ProviderError> {
        // 记录流式调用
        let record = StreamCallRecord {
            messages: request.messages.clone(),
            model: request.model.clone(),
            tools: request.tools.clone(),
            chunks: Vec::new(),
        };
        self.stream_records.lock().unwrap().push(record);

        if let Some(err) = &self.error {
            return Err(ProviderError::Message(err.clone()));
        }

        let chunks = self.stream_chunks.clone();
        let usage = Usage {
            prompt_tokens: 10,
            completion_tokens: chunks.iter().map(|c| c.len() as u32).sum(),
            total_tokens: 0,
        };

        let stream = futures_util::stream::iter(chunks.into_iter().map(move |chunk| {
            Ok(ChatStreamChunk {
                delta: Some(chunk),
                tool_calls: vec![],
                usage: None,
                finish_reason: None,
            })
        }))
        .chain(futures_util::stream::once(async move {
            Ok(ChatStreamChunk {
                delta: None,
                tool_calls: vec![],
                usage: Some(usage),
                finish_reason: Some(FinishReason::Stop),
            })
        }));

        Ok(Box::pin(stream))
    }
}