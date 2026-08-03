//! 网页获取工具
//!
//! 获取网页的 HTML 内容

use async_trait::async_trait;
use futures_util::StreamExt;
use rucora_core::{
    error::ToolError,
    tool::{Tool, ToolCategory, types::ToolContext},
};
use serde_json::{Value, json};
use std::time::Duration;

use super::security::validate_public_http_url;

/// 最大响应体大小（字节）
const MAX_RESPONSE_SIZE: usize = 5 * 1024 * 1024;

/// 网页获取工具：获取网页内容。
///
/// 使用 HTTP 请求获取网页的 HTML 内容，支持超时设置。
/// 与 HttpRequestTool 不同，这个工具专门用于获取网页内容。
///
/// 输入格式：
/// ```json
/// {
///   "url": "https://example.com",
///   "timeout": 30
/// }
/// ```
pub struct WebFetchTool;

impl WebFetchTool {
    /// 创建一个新的 WebFetchTool 实例。
    pub fn new() -> Self {
        Self
    }
}

impl Default for WebFetchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WebFetchTool {
    /// 返回工具名称。
    fn name(&self) -> &str {
        "web_fetch"
    }

    /// 返回工具描述。
    fn description(&self) -> Option<&str> {
        Some("获取网页的 HTML 内容")
    }

    /// 返回工具分类。
    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::Network]
    }

    /// 返回输入参数的 JSON Schema。
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "网页 URL"
                },
                "timeout": {
                    "type": "integer",
                    "description": "超时时间（秒），默认 30 秒",
                    "default": 30
                }
            },
            "required": ["url"]
        })
    }

    /// 执行网页获取。
    async fn call(&self, input: Value, _context: &ToolContext) -> Result<Value, ToolError> {
        let url = input
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::message("缺少必需的 'url' 字段".to_string()))?;

        let timeout_secs = input.get("timeout").and_then(|v| v.as_u64()).unwrap_or(30);

        validate_public_http_url(url, None, None).await?;

        // 构建客户端
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .redirect(reqwest::redirect::Policy::none())
            .user_agent("Mozilla/5.0 (compatible; rucora/0.1)")
            .build()
            .map_err(|e| ToolError::Message { message: format!("HTTP 客户端创建失败: {e}"), source: None })?;

        // 发送 GET 请求
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| ToolError::Message { message: format!("获取网页失败: {e}"), source: None })?;

        let status = response.status().as_u16();

        // 流式读取响应体并限制大小，避免超大响应耗尽内存
        let mut body_bytes: Vec<u8> = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk
                .map_err(|e| ToolError::Message { message: format!("读取响应体失败: {e}"), source: None })?;
            body_bytes.extend_from_slice(&chunk);
            if body_bytes.len() > MAX_RESPONSE_SIZE {
                return Err(ToolError::message(format!(
                    "响应体过大（超过 {MAX_RESPONSE_SIZE} 字节）"
                )));
            }
        }
        let body = String::from_utf8_lossy(&body_bytes).to_string();

        Ok(json!({
            "url": url,
            "status": status,
            "html": body,
            "success": (200..300).contains(&status)
        }))
    }
}
