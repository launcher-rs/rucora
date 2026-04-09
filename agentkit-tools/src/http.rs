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
use tracing::{info, warn};

/// 默认超时时间（秒）
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// 最大响应体大小（字节），默认 5MB
const MAX_RESPONSE_SIZE: usize = 5 * 1024 * 1024;

/// 禁止访问的内网 IP 段
const FORBIDDEN_IP_RANGES: &[&str] = &[
    "10.", "172.16.", "172.17.", "172.18.", "172.19.", "172.20.", "172.21.", "172.22.", "172.23.",
    "172.24.", "172.25.", "172.26.", "172.27.", "172.28.", "172.29.", "172.30.", "172.31.",
    "192.168.", "127.", "0.", "169.254.",
];

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

    /// 检查 URL 是否安全
    fn validate_url(&self, url: &str) -> Result<(), ToolError> {
        // 解析 URL
        let parsed =
            url::Url::parse(url).map_err(|e| ToolError::Message(format!("无效的 URL: {e}")))?;

        // 检查协议
        let scheme = parsed.scheme().to_lowercase();
        if scheme != "http" && scheme != "https" {
            return Err(ToolError::Message(format!(
                "不支持的协议：{scheme}（仅支持 http/https）"
            )));
        }

        // 获取主机名
        let host = parsed
            .host_str()
            .ok_or_else(|| ToolError::Message("URL 缺少主机名".to_string()))?;

        let host_lower = host.to_lowercase();

        // 检查是否在黑名单中
        if let Some(blocked) = &self.blocked_domains {
            for domain in blocked {
                if host_lower.ends_with(domain) || host_lower == *domain {
                    return Err(ToolError::Message(format!("域名 {host} 在黑名单中")));
                }
            }
        }

        // 检查是否在白名单中（如果配置了白名单）
        if let Some(allowed) = &self.allowed_domains {
            let is_allowed = allowed
                .iter()
                .any(|domain| host_lower.ends_with(domain) || host_lower == *domain);
            if !is_allowed {
                return Err(ToolError::Message(format!(
                    "域名 {host} 不在白名单中（允许的域名：{allowed:?}）"
                )));
            }
        }

        // 检查是否为内网 IP（简单的字符串前缀检查）
        for ip_prefix in FORBIDDEN_IP_RANGES {
            if host_lower.starts_with(ip_prefix) {
                return Err(ToolError::Message(format!("禁止访问内网资源：{host}")));
            }
        }

        // 检查 localhost
        if host_lower == "localhost" || host_lower.ends_with(".local") {
            return Err(ToolError::Message(format!("禁止访问本地资源：{host}")));
        }

        Ok(())
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
        Some("发送 HTTP 请求（有安全限制：禁止内网访问，支持域名白名单/黑名单）")
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

    /// 执行工具的核心逻辑。
    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let url = input
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Message("缺少必需的 'url' 字段".to_string()))?;

        // 验证 URL 安全性
        self.validate_url(url)?;

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
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .redirect(reqwest::redirect::Policy::limited(
                self.max_redirects as usize,
            ))
            .user_agent("Mozilla/5.0 (compatible; AgentKit/0.1)")
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
