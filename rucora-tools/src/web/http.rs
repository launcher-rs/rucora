//! HTTP 请求工具
//!
//! 提供 HTTP 请求功能，支持多种方法和安全限制

use async_trait::async_trait;
use rucora_core::{
    error::ToolError,
    tool::{Tool, ToolCategory},
};
use serde_json::{Value, json};
use std::time::Duration;
use tracing::{info, warn};

use super::security::validate_public_http_url;

/// 默认超时时间（秒）
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// 最大响应体大小（字节），默认 5MB
const MAX_RESPONSE_SIZE: usize = 5 * 1024 * 1024;

/// HTTP 请求工具：发送 HTTP 请求。
///
/// 安全限制：
/// - 禁止访问内网资源（防止 SSRF 攻击）
/// - 支持域名白名单/黑名单
/// - 限制重定向次数
/// - 限制响应体大小
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
pub struct HttpRequestTool {
    /// 允许的域名白名单（可选）
    allowed_domains: Option<Vec<String>>,
    /// 禁止的域名黑名单（可选）
    blocked_domains: Option<Vec<String>>,
    /// 最大重定向次数
    max_redirects: u32,
}

impl HttpRequestTool {
    /// 创建一个新的 HttpRequestTool 实例。
    pub fn new() -> Self {
        Self {
            allowed_domains: None,
            blocked_domains: None,
            max_redirects: 3,
        }
    }

    /// 设置允许的域名白名单
    pub fn with_allowed_domains(mut self, domains: Vec<String>) -> Self {
        self.allowed_domains = Some(domains);
        self
    }

    /// 设置禁止的域名黑名单
    pub fn with_blocked_domains(mut self, domains: Vec<String>) -> Self {
        self.blocked_domains = Some(domains);
        self
    }

    /// 设置最大重定向次数
    pub fn with_max_redirects(mut self, max: u32) -> Self {
        self.max_redirects = max;
        self
    }
}

impl Default for HttpRequestTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for HttpRequestTool {
    fn name(&self) -> &str {
        "http_request"
    }

    fn description(&self) -> Option<&str> {
        Some("发送 HTTP 请求（有安全限制：禁止内网访问，支持域名白名单/黑名单）")
    }

    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::Network]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "method": {
                    "type": "string",
                    "description": "HTTP 方法",
                    "enum": ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"]
                },
                "url": {
                    "type": "string",
                    "description": "请求 URL（必须是 http/https 协议）"
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
                    "description": "超时时间（秒），默认 30 秒",
                    "default": 30
                }
            },
            "required": ["url"]
        })
    }

    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let url = input
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Message("缺少必需的 'url' 字段".to_string()))?;

        // 验证 URL 安全性
        validate_public_http_url(
            url,
            self.allowed_domains.as_deref(),
            self.blocked_domains.as_deref(),
        )
        .await?;

        let method_str = input
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("GET")
            .to_uppercase();

        let timeout_secs = input
            .get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_TIMEOUT_SECS);

        info!(
            tool.name = "http_request",
            http.method = %method_str,
            http.url = %url,
            http.timeout_secs = timeout_secs,
            "http_request.start"
        );

        let start = std::time::Instant::now();

        // 解析 HTTP 方法
        let method = match method_str.as_str() {
            "GET" => reqwest::Method::GET,
            "POST" => reqwest::Method::POST,
            "PUT" => reqwest::Method::PUT,
            "DELETE" => reqwest::Method::DELETE,
            "PATCH" => reqwest::Method::PATCH,
            "HEAD" => reqwest::Method::HEAD,
            "OPTIONS" => reqwest::Method::OPTIONS,
            _ => {
                return Err(ToolError::Message(format!(
                    "不支持的 HTTP 方法：{method_str}"
                )));
            }
        };

        // 构建 HTTP 客户端
        let _configured_redirect_limit = self.max_redirects;
        // 自动重定向会让跳转目标绕过调用前 URL 校验，因此这里保守禁用。
        let redirect_policy = reqwest::redirect::Policy::none();
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .redirect(redirect_policy)
            .user_agent("Mozilla/5.0 (compatible; rucora/0.1)")
            .build()
            .map_err(|e| ToolError::Message(format!("HTTP 客户端创建失败：{e}")))?;

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
        let response = request.send().await.map_err(|e| {
            warn!(
                tool.name = "http_request",
                http.method = %method_str,
                http.url = %url,
                error = %e,
                "http_request.error"
            );
            ToolError::Message(format!("HTTP 请求失败：{e}"))
        })?;

        let status = response.status().as_u16();

        // 获取响应体，限制大小
        let body_bytes = response
            .bytes()
            .await
            .map_err(|e| ToolError::Message(format!("读取响应体失败：{e}")))?;

        if body_bytes.len() > MAX_RESPONSE_SIZE {
            return Err(ToolError::Message(format!(
                "响应体过大（{} 字节），超过限制（{} 字节）",
                body_bytes.len(),
                MAX_RESPONSE_SIZE
            )));
        }

        let body = String::from_utf8_lossy(&body_bytes).to_string();

        let elapsed_ms = start.elapsed().as_millis() as u64;
        let body_len = body.len();

        info!(
            tool.name = "http_request",
            http.method = %method_str,
            http.url = %url,
            http.status = status,
            http.success = (200..300).contains(&status),
            http.body_len = body_len,
            http.elapsed_ms = elapsed_ms,
            "http_request.done"
        );

        Ok(json!({
            "url": url,
            "status": status,
            "body": body,
            "body_len": body_len,
            "success": (200..300).contains(&status),
            "elapsed_ms": elapsed_ms
        }))
    }
}
