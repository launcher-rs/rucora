//! 集成测试共享工具：脚本化 Mock Provider 与临时工作区。
// 各测试二进制只用到共享工具的一部分，关闭死代码检查
#![allow(dead_code)]

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Mutex;

use async_trait::async_trait;
use futures_util::stream::{self, BoxStream, StreamExt};
use rucora_core::error::ProviderError;
use rucora_core::provider::LlmProvider;
use rucora_core::provider::types::{ChatRequest, ChatResponse, ChatStreamChunk, FinishReason};
use rucora_core::tool::types::ToolCall;

/// 单次流式调用的脚本。
pub enum Script {
    /// 按序回放的固定分块。
    Chunks(Vec<ChatStreamChunk>),
    /// 先发一个增量然后永久挂起（用于取消测试）。
    HangAfterFirstDelta,
}

/// 脚本化 Mock Provider：按调用次序回放预设的流式分块。
pub struct ScriptedProvider {
    pub(crate) scripts: Mutex<VecDeque<Script>>,
    /// 捕获每次 stream_chat 收到的请求（供断言上下文/历史注入）。
    pub(crate) captured: Mutex<Vec<ChatRequest>>,
}

impl ScriptedProvider {
    pub fn new(scripts: Vec<Vec<ChatStreamChunk>>) -> Self {
        Self {
            scripts: Mutex::new(VecDeque::from(
                scripts.into_iter().map(Script::Chunks).collect::<Vec<_>>(),
            )),
            captured: Mutex::new(Vec::new()),
        }
    }

    /// 直接推入脚本（如挂起流）。
    pub fn push_script(&self, script: Script) {
        self.scripts.lock().expect("脚本队列锁中毒").push_back(script);
    }

    /// 生成"先发一个增量再挂起"的脚本（取消测试用）。
    pub fn hang_script() -> Script {
        Script::HangAfterFirstDelta
    }

    /// 清空脚本队列。
    pub fn clear_scripts(&self) {
        self.scripts.lock().expect("脚本队列锁中毒").clear();
    }

    /// 取出捕获的请求列表。
    pub fn captured_requests(&self) -> Vec<ChatRequest> {
        self.captured.lock().expect("捕获锁中毒").clone()
    }
}

#[async_trait]
impl LlmProvider for ScriptedProvider {
    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        Err(ProviderError::Message(
            "ScriptedProvider 仅支持流式路径".to_string(),
        ))
    }

    fn stream_chat(
        &self,
        request: ChatRequest,
    ) -> Result<BoxStream<'static, Result<ChatStreamChunk, ProviderError>>, ProviderError> {
        self.captured.lock().expect("捕获锁中毒").push(request);
        let script = self
            .scripts
            .lock()
            .expect("脚本队列锁中毒")
            .pop_front()
            .ok_or_else(|| ProviderError::Message("脚本已耗尽".to_string()))?;
        let s: BoxStream<'static, Result<ChatStreamChunk, ProviderError>> = match script {
            Script::Chunks(chunks) => Box::pin(stream::iter(chunks.into_iter().map(Ok))),
            Script::HangAfterFirstDelta => {
                // 先发一个增量，然后永久挂起（用于取消测试）
                let chunk = ChatStreamChunk {
                    delta: Some("开始工作…".to_string()),
                    tool_calls: vec![],
                    usage: None,
                    finish_reason: None,
                };
                Box::pin(stream::iter([Ok(chunk)]).chain(stream::pending()))
            }
        };
        Ok(s)
    }
}

/// 文本增量分块。
pub fn text_chunk(text: &str) -> ChatStreamChunk {
    ChatStreamChunk {
        delta: Some(text.to_string()),
        tool_calls: vec![],
        usage: None,
        finish_reason: None,
    }
}

/// 工具调用分块。
pub fn tool_call_chunk(call: ToolCall) -> ChatStreamChunk {
    ChatStreamChunk {
        delta: None,
        tool_calls: vec![call],
        usage: None,
        finish_reason: Some(FinishReason::ToolCall),
    }
}

/// 创建临时工作区目录。
pub fn temp_workspace(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "rucora-coding-it-{tag}-{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&dir).expect("创建临时工作区");
    dir
}
