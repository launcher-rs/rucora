//! agentkit-a2a - A2A（Agent-to-Agent）协议支持

use agentkit_core::error::ToolError;
use agentkit_core::tool::{Tool, ToolCategory};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

// 重新导出 ra2a 的客户端和服务器
pub use ra2a::{client, server};

/// A2A 协议核心数据结构
pub mod types {
    pub use ra2a::types::*;
}

/// A2A 协议模型定义
pub mod protocol;

/// A2A 传输层
pub mod transport;

/// A2A 工具适配器
pub struct A2AToolAdapter {
    name: String,
    description: String,
    parameters: Value,
    client: Arc<ra2a::client::Client>,
}

impl A2AToolAdapter {
    pub fn new(
        name: String,
        description: String,
        parameters: Value,
        client: ra2a::client::Client,
    ) -> Self {
        Self {
            name,
            description,
            parameters,
            client: Arc::new(client),
        }
    }
}

#[async_trait]
impl Tool for A2AToolAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> Option<&str> {
        Some(&self.description)
    }

    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::External]
    }

    fn input_schema(&self) -> Value {
        self.parameters.clone()
    }

    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        use ra2a::types::{Message, Part, SendMessageRequest};

        let message_text = input.get("message").and_then(|v| v.as_str()).unwrap_or("");

        let msg = Message::user(vec![Part::text(message_text.to_string())]);
        let req = SendMessageRequest::new(msg);

        let result = self
            .client
            .send_message(&req)
            .await
            .map_err(|e| ToolError::Message(format!("A2A 调用失败：{}", e)))?;

        let result_json = serde_json::to_value(&result)
            .map_err(|e| ToolError::Message(format!("序列化失败：{}", e)))?;

        let response = result_json
            .get("result")
            .or_else(|| result_json.get("status"))
            .or_else(|| result_json.get("message"))
            .and_then(|v| v.get("message"))
            .or_else(|| result_json.get("message"))
            .and_then(|m| m.get("parts"))
            .and_then(|parts| parts.as_array())
            .and_then(|arr| arr.first())
            .and_then(|p| p.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("无响应")
            .to_string();

        Ok(serde_json::json!({ "response": response }))
    }
}
