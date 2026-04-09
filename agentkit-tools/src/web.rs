//! 网页工具模块。
//!
//! 提供网页内容获取功能。

use agentkit_core::{
    error::ToolError,
    tool::{Tool, ToolCategory},
};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::time::Duration;

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
    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let url = input
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Message("缺少必需的 'url' 字段".to_string()))?;

        let timeout_secs = input.get("timeout").and_then(|v| v.as_u64()).unwrap_or(30);

        // 验证 URL
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(ToolError::Message(
                "URL 必须以 http:// 或 https:// 开头".to_string(),
            ));
        }

        // 构建客户端
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .user_agent("Mozilla/5.0 (compatible; AgentKit/0.1)")
            .build()
            .map_err(|e| ToolError::Message(format!("HTTP 客户端创建失败: {}", e)))?;

        // 发送 GET 请求
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| ToolError::Message(format!("获取网页失败: {}", e)))?;

        let status = response.status().as_u16();

        // 获取响应体
        let body = response
            .text()
            .await
            .map_err(|e| ToolError::Message(format!("读取响应体失败: {}", e)))?;

        Ok(json!({
            "url": url,
            "status": status,
            "html": body,
            "success": (200..300).contains(&status)
        }))
    }
}
