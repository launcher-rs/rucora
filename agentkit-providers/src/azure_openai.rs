//! Azure OpenAI Provider 实现。
//!
//! 约定：
//! - API Key 从 `AZURE_OPENAI_API_KEY` 环境变量读取
//! - Endpoint 从 `AZURE_OPENAI_ENDPOINT` 环境变量读取
//! - Deployment ID 从 `AZURE_OPENAI_DEPLOYMENT_ID` 环境变量读取（可选）
//! - API 版本默认 `2024-02-15-preview`
//! - 使用 Azure OpenAI 特定的 URL 格式

use std::env;

use crate::{helpers::parse_finish_reason, http_config::build_client, preview};
use agentkit_core::{
    error::ProviderError,
    provider::{
        LlmProvider,
        types::{
            ChatMessage, ChatRequest, ChatResponse, ChatStreamChunk, ResponseFormat, Role, Usage,
        },
    },
    tool::types::{ToolCall, ToolDefinition},
};
use async_trait::async_trait;
use futures_util::{StreamExt, stream::BoxStream};
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use serde_json::{Value, json};
use tracing::debug;

/// Azure OpenAI 默认 Deployment ID。
pub const AZURE_OPENAI_DEFAULT_DEPLOYMENT: &str = "gpt-4";

/// Azure OpenAI Provider。
///
/// 支持 Azure OpenAI 服务：
/// - GPT-4
/// - GPT-35-Turbo
/// - 以及其他 Azure 部署的模型
///
/// # 使用示例
///
/// ```rust,no_run
/// use agentkit::provider::AzureOpenAiProvider;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // 从环境变量加载
/// let provider = AzureOpenAiProvider::from_env()?;
///
/// // 或手动配置
/// let provider = AzureOpenAiProvider::new(
///     "https://your-resource.openai.azure.com",
///     "your-api-key",
///     "your-deployment-id",
/// );
///
/// // 使用特定部署
/// let provider = provider.with_default_deployment("gpt-4");
/// # Ok(())
/// # }
/// ```
///
/// # 环境变量
///
/// | 变量名 | 说明 | 示例 |
/// |--------|------|------|
/// | `AZURE_OPENAI_API_KEY` | Azure OpenAI API Key | `...` |
/// | `AZURE_OPENAI_ENDPOINT` | Azure OpenAI Endpoint | `https://your-resource.openai.azure.com` |
/// | `AZURE_OPENAI_DEPLOYMENT_ID` | 默认 Deployment ID | `gpt-4` |
/// | `AZURE_OPENAI_DEFAULT_DEPLOYMENT` | 默认 Deployment ID（优先级高于 `AZURE_OPENAI_DEPLOYMENT_ID`） | `gpt-4` |
/// | `AZURE_OPENAI_API_VERSION` | API 版本 | `2024-02-15-preview` |
pub struct AzureOpenAiProvider {
    client: reqwest::Client,
    endpoint: String,
    api_version: String,
    default_deployment_id: String,
}

impl AzureOpenAiProvider {
    /// 从环境变量创建 Provider。
    ///
    /// 默认 Deployment 优先级：
    /// 1. `AZURE_OPENAI_DEFAULT_DEPLOYMENT` 环境变量
    /// 2. `AZURE_OPENAI_DEPLOYMENT_ID` 环境变量
    /// 3. 内置默认值 `gpt-4`
    pub fn from_env() -> Result<Self, ProviderError> {
        let api_key = env::var("AZURE_OPENAI_API_KEY")
            .map_err(|_| ProviderError::Message("缺少环境变量 AZURE_OPENAI_API_KEY".to_string()))?;
        let endpoint = env::var("AZURE_OPENAI_ENDPOINT").map_err(|_| {
            ProviderError::Message("缺少环境变量 AZURE_OPENAI_ENDPOINT".to_string())
        })?;
        // 优先使用 AZURE_OPENAI_DEFAULT_DEPLOYMENT，其次使用 AZURE_OPENAI_DEPLOYMENT_ID
        let default_deployment = env::var("AZURE_OPENAI_DEFAULT_DEPLOYMENT")
            .or_else(|_| env::var("AZURE_OPENAI_DEPLOYMENT_ID"))
            .unwrap_or_else(|_| AZURE_OPENAI_DEFAULT_DEPLOYMENT.to_string());
        let api_version = env::var("AZURE_OPENAI_API_VERSION")
            .unwrap_or_else(|_| "2024-02-15-preview".to_string());

        let mut provider = Self::with_deployment(endpoint, api_key, default_deployment);
        provider.api_version = api_version;

        Ok(provider)
    }

    /// 创建 Provider。
    ///
    /// 使用内置默认 Deployment ID `gpt-4`。
    pub fn new(
        endpoint: impl Into<String>,
        api_key: impl Into<String>,
        deployment_id: impl Into<String>,
    ) -> Self {
        Self::with_deployment(endpoint, api_key, deployment_id)
    }

    /// 创建 Provider 并指定默认 Deployment ID。
    pub fn with_deployment(
        endpoint: impl Into<String>,
        api_key: impl Into<String>,
        deployment_id: impl Into<String>,
    ) -> Self {
        let api_key = api_key.into();
        let deployment_id_str = deployment_id.into();
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        // Azure 使用 api-key 头部
        if let Ok(v) = HeaderValue::from_str(&api_key) {
            headers.insert("api-key", v);
        }

        let client = build_client(headers);

        Self {
            client,
            endpoint: endpoint.into(),
            api_version: "2024-02-15-preview".to_string(),
            default_deployment_id: deployment_id_str,
        }
    }

    /// 仅使用 API Key 和 Endpoint 创建 Provider（使用默认 Deployment ID）。
    pub fn with_endpoint_and_key(endpoint: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self::with_deployment(
            endpoint,
            api_key,
            AZURE_OPENAI_DEFAULT_DEPLOYMENT.to_string(),
        )
    }

    /// 设置默认 Deployment ID。
    pub fn with_default_deployment(mut self, deployment_id: impl Into<String>) -> Self {
        self.default_deployment_id = deployment_id.into();
        self
    }

    /// 获取默认 Deployment ID。
    pub fn default_deployment_id(&self) -> &str {
        &self.default_deployment_id
    }

    /// 设置 API 版本。
    pub fn with_api_version(mut self, version: impl Into<String>) -> Self {
        self.api_version = version.into();
        self
    }

    fn map_role(role: &Role) -> &'static str {
        match role {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::Tool => "tool",
        }
    }

    fn build_messages(messages: &[ChatMessage]) -> Vec<Value> {
        messages
            .iter()
            .map(|m| {
                let mut obj = json!({
                    "role": Self::map_role(&m.role),
                    "content": m.content,
                });
                if let Some(name) = &m.name
                    && let Some(map) = obj.as_object_mut()
                {
                    map.insert("name".to_string(), Value::String(name.clone()));
                }
                obj
            })
            .collect()
    }

    fn build_response_format(fmt: &ResponseFormat) -> Value {
        match fmt {
            ResponseFormat::JsonObject => json!({"type": "json_object"}),
            ResponseFormat::JsonSchema {
                name,
                schema,
                strict,
            } => {
                let mut obj = json!({
                    "type": "json_schema",
                    "json_schema": {
                        "name": name,
                        "schema": schema,
                    }
                });
                if let Some(strict) = strict
                    && let Some(root) = obj.as_object_mut()
                    && let Some(js) = root.get_mut("json_schema").and_then(|v| v.as_object_mut())
                {
                    js.insert("strict".to_string(), json!(strict));
                }
                obj
            }
        }
    }

    fn build_tools(tools: &[ToolDefinition]) -> Vec<Value> {
        tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.input_schema,
                    }
                })
            })
            .collect()
    }

    fn parse_tool_calls(message: &Value) -> Vec<ToolCall> {
        let mut out = Vec::new();
        let Some(tool_calls) = message.get("tool_calls") else {
            return out;
        };
        let Some(arr) = tool_calls.as_array() else {
            return out;
        };

        for item in arr {
            let id = item
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let function = item.get("function").cloned().unwrap_or(Value::Null);
            let name = function
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let args_raw = function
                .get("arguments")
                .and_then(|v| v.as_str())
                .unwrap_or("{}");

            let input: Value = serde_json::from_str(args_raw)
                .unwrap_or_else(|_| Value::String(args_raw.to_string()));

            if !id.is_empty() && !name.is_empty() {
                out.push(ToolCall { id, name, input });
            }
        }

        out
    }
}

#[async_trait]
impl LlmProvider for AzureOpenAiProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let deployment_id = request
            .model
            .clone()
            .unwrap_or_else(|| self.default_deployment_id.clone());

        // Azure OpenAI URL 格式：
        // {endpoint}/openai/deployments/{deployment-id}/chat/completions?api-version={api_version}
        let url = format!(
            "{}/openai/deployments/{}/chat/completions?api-version={}",
            self.endpoint.trim_end_matches('/'),
            deployment_id,
            self.api_version
        );

        let messages = Self::build_messages(&request.messages);

        let last_user_preview = request
            .messages
            .iter()
            .rev()
            .find(|m| m.role == Role::User)
            .map(|m| preview(&m.content, 600));

        debug!(
            provider = "azure_openai",
            url = %url,
            deployment_id = %deployment_id,
            messages_len = request.messages.len(),
            tools_len = request.tools.as_ref().map(|t| t.len()).unwrap_or(0),
            last_user = last_user_preview.as_deref().unwrap_or(""),
            "provider.chat.start"
        );

        let mut body = json!({
            "messages": messages,
        });

        if let Some(tools) = request.tools.as_ref()
            && let Some(map) = body.as_object_mut()
        {
            map.insert("tools".to_string(), Value::Array(Self::build_tools(tools)));
        }
        if let Some(t) = request.temperature
            && let Some(map) = body.as_object_mut()
        {
            map.insert("temperature".to_string(), json!(t));
        }
        if let Some(max_tokens) = request.max_tokens
            && let Some(map) = body.as_object_mut()
        {
            map.insert("max_tokens".to_string(), json!(max_tokens));
        }
        if let Some(fmt) = request.response_format.as_ref()
            && let Some(map) = body.as_object_mut()
        {
            map.insert(
                "response_format".to_string(),
                Self::build_response_format(fmt),
            );
        }

        debug!(
            provider = "azure_openai",
            deployment_id = %deployment_id,
            body = %preview(&body.to_string(), 1200),
            "provider.chat.request_body"
        );

        let start = std::time::Instant::now();

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Message(e.to_string()))?;

        let status = resp.status();
        let data: Value = resp
            .json()
            .await
            .map_err(|e| ProviderError::Message(e.to_string()))?;

        let elapsed_ms = start.elapsed().as_millis() as u64;
        debug!(
            provider = "azure_openai",
            status = %status,
            elapsed_ms,
            "provider.chat.http.done"
        );
        debug!(
            provider = "azure_openai",
            status = %status,
            body = %preview(&data.to_string(), 1200),
            "provider.chat.response_body"
        );

        if !status.is_success() {
            return Err(ProviderError::Message(format!(
                "Azure OpenAI 请求失败：status={} body={}",
                status, data
            )));
        }

        let message = data
            .get("choices")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|c| c.get("message"))
            .cloned()
            .unwrap_or_else(|| json!({}));

        let content = message
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let tool_calls = Self::parse_tool_calls(&message);

        let usage = data.get("usage").map(|u| Usage {
            prompt_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            completion_tokens: u
                .get("completion_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32,
            total_tokens: u.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        });

        let finish_reason_str = data
            .get("choices")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|c| c.get("finish_reason"))
            .and_then(|v| v.as_str())
            .unwrap_or("stop")
            .to_string();

        Ok(ChatResponse {
            message: ChatMessage {
                role: Role::Assistant,
                content,
                name: None,
            },
            tool_calls,
            usage,
            finish_reason: Some(parse_finish_reason(&finish_reason_str)),
        })
    }

    fn stream_chat(
        &self,
        request: ChatRequest,
    ) -> Result<BoxStream<'static, Result<ChatStreamChunk, ProviderError>>, ProviderError> {
        let deployment_id = request
            .model
            .clone()
            .unwrap_or_else(|| self.default_deployment_id.clone());

        let url = format!(
            "{}/openai/deployments/{}/chat/completions?api-version={}",
            self.endpoint.trim_end_matches('/'),
            deployment_id,
            self.api_version
        );

        let messages = Self::build_messages(&request.messages);

        let mut body = json!({
            "messages": messages,
            "stream": true,
        });

        if let Some(tools) = request.tools.as_ref()
            && let Some(map) = body.as_object_mut()
        {
            map.insert("tools".to_string(), Value::Array(Self::build_tools(tools)));
        }
        if let Some(t) = request.temperature
            && let Some(map) = body.as_object_mut()
        {
            map.insert("temperature".to_string(), json!(t));
        }
        if let Some(max_tokens) = request.max_tokens
            && let Some(map) = body.as_object_mut()
        {
            map.insert("max_tokens".to_string(), json!(max_tokens));
        }

        let client = self.client.clone();
        let stream = async_stream::try_stream! {
            let resp = client
                .post(&url)
                .json(&body)
                .send()
                .await
                .map_err(|e| ProviderError::Message(e.to_string()))?;

            let status = resp.status();
            if !status.is_success() {
                Err(ProviderError::Message(format!(
                    "Azure OpenAI 流式请求失败：status={}",
                    status
                )))?;
            }

            let mut buf = String::new();
            let mut bytes_stream = resp.bytes_stream();

            while let Some(item) = bytes_stream.next().await {
                let bytes = item.map_err(|e| ProviderError::Message(e.to_string()))?;
                let chunk = String::from_utf8_lossy(&bytes);
                buf.push_str(&chunk);

                while let Some(idx) = buf.find("\n\n") {
                    let event = buf[..idx].to_string();
                    buf = buf[idx + 2..].to_string();

                    let mut data_lines: Vec<&str> = Vec::new();
                    for line in event.lines() {
                        let line = line.trim();
                        if let Some(rest) = line.strip_prefix("data:") {
                            data_lines.push(rest.trim());
                        }
                    }

                    if data_lines.is_empty() {
                        continue;
                    }

                    let data = data_lines.join("\n");
                    if data == "[DONE]" {
                        break;
                    }

                    let v: Value = serde_json::from_str(&data)
                        .map_err(|e| ProviderError::Message(format!("SSE 解析失败：{} data={}", e, data)))?;

                    let delta = v
                        .get("choices")
                        .and_then(|c| c.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|c0| c0.get("delta"))
                        .and_then(|d| d.get("content"))
                        .and_then(|s| s.as_str())
                        .map(|s| s.to_string());

                    if delta.is_some() {
                        yield ChatStreamChunk {
                            delta,
                            tool_calls: vec![],
                            usage: None,
                            finish_reason: None,
                        };
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_azure_openai_provider_creation() {
        let provider = AzureOpenAiProvider::with_deployment(
            "https://test.openai.azure.com",
            "test-key",
            "test-deployment",
        );
        assert!(provider.endpoint.contains("azure.com"));
        assert_eq!(provider.default_deployment_id(), "test-deployment");
    }

    #[test]
    fn test_azure_openai_provider_with_custom_version() {
        let provider = AzureOpenAiProvider::with_deployment(
            "https://test.openai.azure.com",
            "test-key",
            "test-deployment",
        )
        .with_api_version("2024-03-01-preview");
        assert_eq!(provider.api_version, "2024-03-01-preview".to_string());
    }
}
