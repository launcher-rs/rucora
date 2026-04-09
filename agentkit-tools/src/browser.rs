//! 浏览器工具模块。
//!
//! 提供系统浏览器操作功能。

use agentkit_core::{
    error::ToolError,
    tool::{Tool, ToolCategory},
};
use async_trait::async_trait;
use serde_json::{Value, json};

/// 浏览器打开工具：在系统默认浏览器中打开 URL。
///
/// 仅在系统默认浏览器中打开批准的 HTTPS URL，不进行网页抓取或 DOM 操作。
/// 用于需要用户查看网页内容的场景。
///
/// 输入格式：
/// ```json
/// {
///   "url": "https://example.com"
/// }
/// ```
pub struct BrowserOpenTool;

impl BrowserOpenTool {
    /// 创建一个新的 BrowserOpenTool 实例。
    pub fn new() -> Self {
        Self
    }

    /// 验证 URL 是否安全
    fn validate_url(&self, url: &str) -> Result<String, ToolError> {
        let url = url.trim();

        if url.is_empty() {
            return Err(ToolError::Message("URL 不能为空".to_string()));
        }

        if url.chars().any(char::is_whitespace) {
            return Err(ToolError::Message("URL 不能包含空白字符".to_string()));
        }

        if !url.starts_with("https://") {
            return Err(ToolError::Message("只允许 https:// URL".to_string()));
        }

        // 检查是否为本地或私有地址
        let host = url
            .trim_start_matches("https://")
            .split('/')
            .next()
            .unwrap_or("");

        if host.starts_with("localhost")
            || host.starts_with("127.")
            || host.starts_with("192.168.")
            || host.starts_with("10.")
            || host.starts_with("172.")
        {
            return Err(ToolError::Message(format!(
                "阻止访问本地/私有主机: {host}"
            )));
        }

        Ok(url.to_string())
    }
}

impl Default for BrowserOpenTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for BrowserOpenTool {
    /// 返回工具名称。
    fn name(&self) -> &str {
        "browser_open"
    }

    /// 返回工具描述。
    fn description(&self) -> Option<&str> {
        Some("在系统默认浏览器中打开 HTTPS URL")
    }

    /// 返回工具分类。
    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::Browser]
    }

    /// 返回输入参数的 JSON Schema。
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "要打开的 HTTPS URL"
                }
            },
            "required": ["url"]
        })
    }

    /// 执行打开浏览器操作。
    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let url = input
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Message("缺少必需的 'url' 字段".to_string()))?;

        // 验证 URL
        let validated_url = self.validate_url(url)?;

        // 根据操作系统打开浏览器
        #[cfg(target_os = "windows")]
        {
            tokio::process::Command::new("cmd")
                .args(["/C", "start", "", &validated_url])
                .spawn()
                .map_err(|e| ToolError::Message(format!("打开浏览器失败: {e}")))?;
        }

        #[cfg(target_os = "macos")]
        {
            tokio::process::Command::new("open")
                .arg(&validated_url)
                .spawn()
                .map_err(|e| ToolError::Message(format!("打开浏览器失败: {}", e)))?;
        }

        #[cfg(target_os = "linux")]
        {
            tokio::process::Command::new("xdg-open")
                .arg(&validated_url)
                .spawn()
                .map_err(|e| ToolError::Message(format!("打开浏览器失败: {}", e)))?;
        }

        Ok(json!({
            "success": true,
            "url": validated_url,
            "message": "已在系统浏览器中打开 URL"
        }))
    }
}
