//! 联网搜索工具
//!
//! 提供网络搜索功能，用于深度研究时收集信息

use agentkit_core::{
    error::ToolError,
    tool::{Tool, ToolCategory},
};
use async_trait::async_trait;
use serde_json::{Value, json};

/// 联网搜索工具
///
/// 使用搜索引擎搜索相关信息
pub struct WebSearchTool {
    /// 搜索结果数量限制
    pub max_results: usize,
    /// API Key（可选，用于某些搜索 API）
    pub api_key: Option<String>,
}

impl WebSearchTool {
    /// 创建新的搜索工具
    pub fn new() -> Self {
        Self {
            max_results: 10,
            api_key: None,
        }
    }

    /// 设置最大结果数
    pub fn with_max_results(mut self, max: usize) -> Self {
        self.max_results = max;
        self
    }

    /// 设置 API Key
    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// 执行搜索（模拟实现）
    async fn search(&self, query: &str) -> Result<Vec<SearchResult>, ToolError> {
        // 模拟搜索结果
        // 真实实现中可以调用 Google Custom Search API、Bing Search API 等
        let mock_results = vec![
            SearchResult {
                title: format!("{query} - 官方文档"),
                url: "https://example.com/official".to_string(),
                snippet: format!("{query}的官方文档，提供详细的技术说明和 API 参考。"),
            },
            SearchResult {
                title: format!("{query} 入门教程"),
                url: "https://example.com/tutorial".to_string(),
                snippet: format!("{query}的完整入门教程，适合初学者系统学习。"),
            },
            SearchResult {
                title: format!("{query} 最佳实践"),
                url: "https://example.com/best-practices".to_string(),
                snippet: format!("{query}的最佳实践指南，包含实际项目中的经验总结。"),
            },
            SearchResult {
                title: format!("{query} 技术博客"),
                url: "https://example.com/blog".to_string(),
                snippet: format!("多位专家分享的{query}实践经验和技术心得。"),
            },
            SearchResult {
                title: format!("{query} GitHub 项目"),
                url: "https://github.com/topics/{}".to_string(),
                snippet: format!("GitHub 上相关的{}开源项目集合。", query.to_lowercase()),
            },
        ];

        Ok(mock_results.into_iter().take(self.max_results).collect())
    }
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

/// 搜索结果
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> Option<&str> {
        Some("联网搜索工具，用于搜索网络上的相关信息和资源")
    }

    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::Network]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "搜索关键词"
                },
                "max_results": {
                    "type": "integer",
                    "description": "最大结果数量（可选，默认 10）"
                }
            },
            "required": ["query"]
        })
    }

    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let query = input
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Message("缺少必需的 'query' 字段".to_string()))?;

        let max_results = input
            .get("max_results")
            .and_then(|v| v.as_u64())
            .map_or(self.max_results, |v| v as usize);

        // 执行搜索
        let results = self.search(query).await?;

        // 限制结果数量
        let limited_results: Vec<SearchResult> = results.into_iter().take(max_results).collect();

        // 格式化输出
        let formatted_results: Vec<Value> = limited_results
            .iter()
            .map(|r| {
                json!({
                    "title": r.title,
                    "url": r.url,
                    "snippet": r.snippet
                })
            })
            .collect();

        Ok(json!({
            "query": query,
            "total_results": formatted_results.len(),
            "results": formatted_results
        }))
    }
}

/// 网页抓取工具
///
/// 从指定 URL 抓取网页内容
pub struct WebScraperTool {
    /// 请求超时时间（秒）
    pub timeout_secs: u64,
}

impl WebScraperTool {
    /// 创建新的抓取工具
    pub fn new() -> Self {
        Self { timeout_secs: 30 }
    }

    /// 设置超时时间
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }
}

impl Default for WebScraperTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WebScraperTool {
    fn name(&self) -> &str {
        "web_scraper"
    }

    fn description(&self) -> Option<&str> {
        Some("网页抓取工具，从指定 URL 抓取网页内容")
    }

    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::Network]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "要抓取的网页 URL"
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

        // 模拟抓取结果
        // 真实实现中可以使用 reqwest 等库抓取网页
        let mock_content = format!(
            r#"网页内容摘要：
URL: {url}
标题：示例网页
内容：这是一个模拟的网页内容。在实际实现中，这里会显示从网络抓取的真实网页内容。
可以使用 reqwest 库发送 HTTP 请求，然后解析 HTML 获取所需信息。"#
        );

        Ok(json!({
            "url": url,
            "content": mock_content,
            "success": true
        }))
    }
}
