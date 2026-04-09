use agentkit_core::{
    error::ToolError,
    tool::{Tool, ToolCategory},
};
use async_trait::async_trait;
use dom_smoothie::{Config, Readability, TextMode};
use html2text::from_read;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::time::sleep;
use tracing::{debug, info, warn};

const DEFAULT_MAX_CONTENT_CHARS: usize = 15_000;

#[derive(Default, Clone)]
pub struct BrowseTool {
    sessions: Arc<Mutex<HashMap<String, BrowseSession>>>,
}

#[derive(Default, Clone)]
struct BrowseSession {
    url: Option<String>,
    content: Option<String>,
    raw_html: Option<String>,
}

impl BrowseTool {
    pub fn new() -> Self {
        Self::default()
    }

    fn readability_content(content: &str) -> Result<String, ToolError> {
        let cfg = Config {
            text_mode: TextMode::Formatted,
            ..Default::default()
        };

        let mut readability = Readability::new(content, None, Some(cfg))
            .map_err(|e| ToolError::Message(e.to_string()))?;
        let article = readability
            .parse()
            .map_err(|e| ToolError::Message(e.to_string()))?;

        let title = article.title.trim().to_string();
        let mut text = article.text_content.to_string();

        // 对“列表页/聚合页”，text_content 可能非常短；此时把 content(HTML片段) 转成纯文本作为兜底。
        if text.chars().count() < 500 {
            let html_fragment = article.content.to_string();
            let plain = from_read(html_fragment.as_bytes(), 120);
            if plain.chars().count() > text.chars().count() {
                text = plain;
            }
        }

        if title.is_empty() {
            Ok(text)
        } else {
            Ok(format!("{title}\n\n{text}"))
        }
    }

    async fn fetch_html(&self, url: &str, timeout_ms: u64) -> Result<String, ToolError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .user_agent("Mozilla/5.0 (compatible; AgentKit/0.1)")
            .build()
            .map_err(|e| ToolError::Message(format!("HTTP 客户端创建失败: {e}")))?;

        let resp = client
            .get(url)
            .send()
            .await
            .map_err(|e| ToolError::Message(format!("请求失败: {e}")))?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| ToolError::Message(format!("读取响应体失败: {e}")))?;

        if !status.is_success() {
            return Err(ToolError::Message(format!(
                "HTTP 状态码异常: {}",
                status.as_u16()
            )));
        }

        Ok(text)
    }
}

#[async_trait]
impl Tool for BrowseTool {
    fn name(&self) -> &str {
        "browse"
    }

    fn description(&self) -> Option<&str> {
        Some("最小浏览器兼容工具（非 CDP）：支持 navigate/get_content/wait/session_close")
    }

    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::Browser]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {"type": "string", "description": "navigate/get_content/wait/session_close"},
                "url": {"type": "string", "description": "navigate 时的 URL"},
                "session": {"type": "string", "description": "会话 id"},
                "timeout": {"type": "integer", "description": "wait 超时(ms) 或 navigate 请求超时(ms)", "default": 30000}
            },
            "required": ["action"]
        })
    }

    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let action = input
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Message("缺少必需的 'action' 字段".to_string()))?;

        let session = input
            .get("session")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();

        match action {
            "navigate" => {
                let url = input.get("url").and_then(|v| v.as_str()).ok_or_else(|| {
                    ToolError::Message("navigate 缺少必需的 'url' 字段".to_string())
                })?;

                let timeout_ms = input
                    .get("timeout")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(30_000);

                info!(tool.name = "browse", browse.action = "navigate", browse.session = %session, browse.url = %url, "browse.start");

                let html = match self.fetch_html(url, timeout_ms).await {
                    Ok(v) => v,
                    Err(e) => {
                        warn!(tool.name = "browse", error = %e, "browse.navigate.failed");
                        return Ok(json!({"success": false, "error": e.to_string()}));
                    }
                };

                let raw_html = html;

                let extracted = match Self::readability_content(&raw_html) {
                    Ok(v) => v,
                    Err(e) => {
                        debug!(tool.name = "browse", error = %e, "browse.readability.fallback_to_html");
                        raw_html.clone()
                    }
                };

                let mut sessions = self
                    .sessions
                    .lock()
                    .map_err(|_| ToolError::Message("browse session lock poisoned".to_string()))?;
                let ent = sessions.entry(session.clone()).or_default();
                ent.url = Some(url.to_string());
                ent.content = Some(extracted);
                ent.raw_html = Some(raw_html);

                Ok(json!({"success": true}))
            }
            "wait" => {
                let timeout_ms = input
                    .get("timeout")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1000);

                debug!(tool.name = "browse", browse.action = "wait", browse.session = %session, browse.timeout_ms = timeout_ms, "browse.wait");
                sleep(Duration::from_millis(timeout_ms)).await;
                Ok(json!({"success": true}))
            }
            "get_content" => {
                let max_chars = input
                    .get("max_chars")
                    .and_then(|v| v.as_u64())
                    .map_or(DEFAULT_MAX_CONTENT_CHARS, |v| v as usize);

                let sessions = self
                    .sessions
                    .lock()
                    .map_err(|_| ToolError::Message("browse session lock poisoned".to_string()))?;
                let ent = sessions.get(&session);
                let content = ent.and_then(|s| s.content.clone()).unwrap_or_default();
                let raw_html = ent.and_then(|s| s.raw_html.clone()).unwrap_or_default();

                let content_len = content.chars().count();
                let raw_html_len = raw_html.chars().count();

                let content_truncated = content_len > max_chars;
                let raw_html_truncated = raw_html_len > max_chars;

                let out_content = if content_truncated {
                    content.chars().take(max_chars).collect::<String>()
                } else {
                    content
                };

                let out_raw_html = if raw_html_truncated {
                    raw_html.chars().take(max_chars).collect::<String>()
                } else {
                    raw_html
                };

                let raw_text = from_read(out_raw_html.as_bytes(), 120);

                Ok(json!({
                    "success": true,
                    "content": out_content,
                    "content_len": content_len,
                    "content_truncated": content_truncated,
                    "raw_html": out_raw_html,
                    "raw_html_len": raw_html_len,
                    "raw_html_truncated": raw_html_truncated,
                    "raw_text": raw_text,
                    "max_chars": max_chars,
                    "truncated": content_truncated || raw_html_truncated
                }))
            }
            "session_close" => {
                let mut sessions = self
                    .sessions
                    .lock()
                    .map_err(|_| ToolError::Message("browse session lock poisoned".to_string()))?;
                sessions.remove(&session);
                Ok(json!({"success": true}))
            }
            other => Err(ToolError::Message(format!("未知 action: {other}"))),
        }
    }
}
