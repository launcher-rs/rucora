//! HTTP 工具模块。
//!
//! 提供 HTTP 请求功能，支持多种方法和安全限制。

use agentkit_core::{
    error::ToolError,
    tool::{Tool, ToolCategory},
};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::time::Duration;

/// HTTP 请求工具：发送 HTTP 请求。
///
/// 支持多种方法和安全限制，防止请求执行时间过长或输出过大。
///
/// 适用场景：
/// - 发送 HTTP 请求
/// - 获取网页内容
///
/// 输入格式：
/// ```json
/// {
///   "method": "GET",
///   "url": "https://example.com",
///   "headers": {
///     "Accept": "text/html"
///   },
///   "body": "请求体",
///   "timeout": 60 // 可选，超时时间（秒）
/// }
/// ```
pub struct HttpRequestTool;

impl HttpRequestTool {
    /// 创建一个新的 HttpRequestTool 实例。
    pub fn new() -> Self {
        Self
    }
}

impl Default for HttpRequestTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for HttpRequestTool {
    /// 返回工具名称。
    fn name(&self) -> &str {
        "http_request"
    }

    /// 返回工具描述。
    fn description(&self) -> Option<&str> {
        Some("发送 HTTP 请求")
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
                "method": {
                    "type": "string",
                    "description": "HTTP 方法"
                },
                "url": {
                    "type": "string",
                    "description": "请求 URL"
                },
                "headers": {
                    "type": "object",
                    "additionalProperties": {
                        "type": "string"
                    },
                    "description": "请求头"
                },
                "body": {
                    "type": "string",
                    "description": "请求体"
                },
                "timeout": {
                    "type": "integer",
                    "description": "超时时间（秒）"
                }
            },
            "required": ["method", "url", "headers", "body"]
        })
    }

    /// 执行工具的核心逻辑。
    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let url = input
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Message("缺少必需的 'url' 字段".to_string()))?;

        let method_str = input
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("GET")
            .to_uppercase();

        let timeout_secs = input.get("timeout").and_then(|v| v.as_u64()).unwrap_or(30);

        // 解析 HTTP 方法
        let method = match method_str.as_str() {
            "GET" => reqwest::Method::GET,
            "POST" => reqwest::Method::POST,
            "PUT" => reqwest::Method::PUT,
            "DELETE" => reqwest::Method::DELETE,
            "PATCH" => reqwest::Method::PATCH,
            "HEAD" => reqwest::Method::HEAD,
            _ => {
                return Err(ToolError::Message(format!(
                    "不支持的 HTTP 方法: {}",
                    method_str
                )));
            }
        };

        // 构建 HTTP 客户端
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .map_err(|e| ToolError::Message(format!("HTTP 客户端创建失败: {}", e)))?;

        // 构建请求
        let mut request = client.request(method, url);

        // 添加请求头
        if let Some(headers_map) = input.get("headers").and_then(|v| v.as_object()) {
            for (key, value) in headers_map {
                if let Some(val_str) = value.as_str() {
                    request = request.header(key, val_str);
                }
            }
        }

        // 添加请求体
        if let Some(body_str) = input.get("body").and_then(|v| v.as_str()) {
            request = request.body(body_str.to_string());
        }

        // 发送请求
        let response = request
            .send()
            .await
            .map_err(|e| ToolError::Message(format!("HTTP 请求失败: {}", e)))?;

        let status = response.status().as_u16();

        // 尝试获取响应体
        let body = response
            .text()
            .await
            .map_err(|e| ToolError::Message(format!("读取响应体失败: {}", e)))?;

        Ok(json!({
            "status": status,
            "body": body,
            "success": status >= 200 && status < 300
        }))
    }
}
