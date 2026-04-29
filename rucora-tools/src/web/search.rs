//! Web 搜索工具
//!
//! 提供多种搜索服务集成（SerpAPI、Tavily、GitHub Trending）

use async_trait::async_trait;
use backon::{ExponentialBuilder, Retryable};
use reqwest::Client;
use rucora_core::{
    error::ToolError,
    tool::{Tool, ToolCategory},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;

// ===== SerpAPI 搜索工具 =====

/// SerpAPI 搜索工具
///
/// 支持多个 API Key 轮询使用
pub struct SerpapiTool {
    /// API Keys 列表
    api_keys: Vec<String>,
}

impl SerpapiTool {
    /// 创建新的 SerpAPI 工具（单个 API Key）
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_keys: vec![api_key.into()],
        }
    }

    /// 创建带多个 API Keys 的 SerpAPI 工具。
    ///
    /// 当 `api_keys` 为空时返回错误，而不是 panic。
    pub fn with_keys(api_keys: Vec<String>) -> Result<Self, ToolError> {
        if api_keys.is_empty() {
            return Err(ToolError::Message("API Keys 不能为空".to_string()));
        }
        Ok(Self { api_keys })
    }

    /// 从环境变量加载 API Keys
    ///
    /// 环境变量：`SERPAPI_API_KEYS` - 逗号分隔的多个 API Keys
    pub fn from_env() -> Result<Self, ToolError> {
        let api_keys_str = std::env::var("SERPAPI_API_KEYS")
            .or_else(|_| std::env::var("SERPAPI_API_KEY"))
            .map_err(|_| {
                ToolError::Message("缺少环境变量 SERPAPI_API_KEYS 或 SERPAPI_API_KEY".to_string())
            })?;

        let api_keys: Vec<String> = api_keys_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if api_keys.is_empty() {
            return Err(ToolError::Message("API Keys 不能为空".to_string()));
        }

        Ok(Self { api_keys })
    }
}

/// SerpAPI 搜索参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerpapiArgs {
    /// 搜索关键词
    pub query: String,
    /// 时间范围：qdr:h（最近 1 小时）, qdr:d（最近 1 天）, qdr:w（最近 1 周）, qdr:m（最近 1 月）, qdr:y（最近 1 年）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tbs: Option<String>,
    /// 搜索国家：us（美国）, uk（英国）, cn（中国）, ru（俄罗斯）等
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gl: Option<String>,
    /// 搜索语言：en（英文）, zh-cn（简体中文）, zh-tw（繁体中文）, ru（俄文）等
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hl: Option<String>,
}

#[async_trait]
impl Tool for SerpapiTool {
    fn name(&self) -> &str {
        "serpapi_search"
    }

    fn description(&self) -> Option<&str> {
        Some("使用 SerpAPI 进行 Google 搜索，需要设置 SERPAPI_API_KEY 环境变量")
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
                "tbs": {
                    "type": "string",
                    "description": "时间范围：qdr:h（最近 1 小时）, qdr:d（最近 1 天）, qdr:w（最近 1 周）, qdr:m（最近 1 月）, qdr:y（最近 1 年）"
                },
                "gl": {
                    "type": "string",
                    "description": "搜索国家：us（美国）, uk（英国）, cn（中国）等"
                },
                "hl": {
                    "type": "string",
                    "description": "搜索语言：en（英文）, zh-cn（简体中文）等"
                }
            },
            "required": ["query"]
        })
    }

    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let args: SerpapiArgs = serde_json::from_value(input)
            .map_err(|e| ToolError::Message(format!("解析参数失败：{e}")))?;

        let config = ExponentialBuilder::default();
        let api_keys = self.api_keys.clone();

        let result = (move || {
            let args = args.clone();
            let api_keys = api_keys.clone();

            async move {
                // 构建搜索参数
                let mut params = HashMap::new();
                params.insert("engine".to_string(), "google".to_string());
                params.insert("q".to_string(), args.query.clone());

                if let Some(ref tbs) = args.tbs {
                    params.insert("tbs".to_string(), tbs.clone());
                }
                if let Some(ref gl) = args.gl {
                    params.insert("gl".to_string(), gl.clone());
                }
                if let Some(ref hl) = args.hl {
                    params.insert("hl".to_string(), hl.clone());
                }

                // 基于时间选择一个 API Key
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default();
                let idx = (now.subsec_nanos() as usize) % api_keys.len();
                let api_key = api_keys[idx].clone();
                params.insert("api_key".to_string(), api_key);

                // 执行搜索
                let client = Client::new();
                let response = client
                    .get("https://serpapi.com/search")
                    .query(&params)
                    .send()
                    .await
                    .map_err(|e| ToolError::Message(format!("请求失败：{e}")))?;

                let search_result: Value = response
                    .json()
                    .await
                    .map_err(|e| ToolError::Message(format!("解析 JSON 失败：{e}")))?;

                // 提取有机搜索结果
                let organic_results = search_result
                    .get("organic_results")
                    .ok_or_else(|| ToolError::Message("没有搜索结果".to_string()))?;

                Ok(organic_results.clone())
            }
        })
        .retry(config)
        .sleep(tokio::time::sleep)
        .notify(|err: &ToolError, _dur: std::time::Duration| {
            tracing::warn!("SerpAPI 重试：{:?}", err);
        })
        .await;

        match result {
            Ok(value) => Ok(json!({
                "success": true,
                "results": value
            })),
            Err(e) => Err(e),
        }
    }
}

// ===== Tavily 搜索工具 =====

/// Tavily 搜索工具
///
/// 支持多个 API Keys 轮询使用
pub struct TavilyTool {
    /// API Keys 列表
    api_keys: Vec<String>,
}

impl TavilyTool {
    /// 创建新的 Tavily 工具（单个 API Key）
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_keys: vec![api_key.into()],
        }
    }

    /// 创建带多个 API Keys 的 Tavily 工具。
    ///
    /// 当 `api_keys` 为空时返回错误，而不是 panic。
    pub fn with_keys(api_keys: Vec<String>) -> Result<Self, ToolError> {
        if api_keys.is_empty() {
            return Err(ToolError::Message("API Keys 不能为空".to_string()));
        }
        Ok(Self { api_keys })
    }

    /// 从环境变量加载 API Keys
    ///
    /// 环境变量：`TAVILY_API_KEYS` - 逗号分隔的多个 API Keys
    pub fn from_env() -> Result<Self, ToolError> {
        let api_keys_str = std::env::var("TAVILY_API_KEYS")
            .or_else(|_| std::env::var("TAVILY_API_KEY"))
            .map_err(|_| {
                ToolError::Message("缺少环境变量 TAVILY_API_KEYS 或 TAVILY_API_KEY".to_string())
            })?;

        let api_keys: Vec<String> = api_keys_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if api_keys.is_empty() {
            return Err(ToolError::Message("API Keys 不能为空".to_string()));
        }

        Ok(Self { api_keys })
    }
}

/// Tavily 搜索参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TavilyArgs {
    /// 搜索关键词
    pub query: String,
    /// 搜索结果数量（默认 5，最大 15）
    #[serde(default = "default_max_results")]
    pub max_results: usize,
    /// 包含答案（默认 true）
    #[serde(default = "default_true")]
    pub include_answer: bool,
    /// 包含原始内容（默认 false）
    #[serde(default)]
    pub include_raw_content: bool,
    /// 搜索深度：basic（基础）, advanced（高级）
    #[serde(default = "default_search_depth")]
    pub search_depth: String,
}

fn default_max_results() -> usize {
    5
}

fn default_true() -> bool {
    true
}

fn default_search_depth() -> String {
    "basic".to_string()
}

#[async_trait]
impl Tool for TavilyTool {
    fn name(&self) -> &str {
        "tavily_search"
    }

    fn description(&self) -> Option<&str> {
        Some("使用 Tavily AI 进行智能搜索，需要设置 TAVILY_API_KEY 环境变量")
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
                    "description": "搜索结果数量（默认 5，最大 15）"
                },
                "include_answer": {
                    "type": "boolean",
                    "description": "是否包含 AI 生成的答案（默认 true）"
                },
                "search_depth": {
                    "type": "string",
                    "description": "搜索深度：basic（基础）, advanced（高级）",
                    "enum": ["basic", "advanced"]
                }
            },
            "required": ["query"]
        })
    }

    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let args: TavilyArgs = serde_json::from_value(input)
            .map_err(|e| ToolError::Message(format!("解析参数失败：{e}")))?;

        // 选择一个 API Key
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let idx = (now.subsec_nanos() as usize) % self.api_keys.len();
        let api_key = self.api_keys[idx].clone();

        // 构建请求体
        let request_body = json!({
            "query": args.query,
            "api_key": api_key,
            "max_results": args.max_results.min(15),
            "include_answer": args.include_answer,
            "include_raw_content": args.include_raw_content,
            "search_depth": args.search_depth,
        });

        // 执行搜索
        let client = Client::new();
        let response = client
            .post("https://api.tavily.com/search")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ToolError::Message(format!("请求失败：{e}")))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "未知错误".to_string());
            return Err(ToolError::Message(format!("Tavily API 错误：{error_text}")));
        }

        let search_result: Value = response
            .json()
            .await
            .map_err(|e| ToolError::Message(format!("解析 JSON 失败：{e}")))?;

        Ok(json!({
            "success": true,
            "answer": search_result.get("answer").cloned(),
            "results": search_result.get("results").cloned().unwrap_or(json!([])),
            "query": search_result.get("query").cloned()
        }))
    }
}

// ===== GitHub Trending 工具 =====

/// GitHub 趋势榜工具
///
/// 获取 GitHub 热门项目趋势信息
pub struct GithubTrendingTool;

/// GitHub 趋势数据
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GithubTrendingData {
    /// 代码仓库标题
    pub title: String,
    /// 代码仓库描述
    pub description: String,
    /// 代码仓库链接
    pub url: String,
    /// 编程语言
    pub language: String,
    /// 代码仓库 star 数量
    pub stars: String,
    /// 代码仓库 fork 数量
    pub forks: String,
    /// 代码仓库今天 star 数量
    pub today_stars: String,
}

impl GithubTrendingTool {
    /// 创建新的 GitHub 趋势工具
    pub fn new() -> Self {
        Self
    }

    /// 获取 GitHub 趋势榜
    async fn get_github_trending(&self) -> Result<Vec<GithubTrendingData>, ToolError> {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (compatible; rucora/0.1)")
            .build()
            .map_err(|e| ToolError::Message(format!("创建 HTTP 客户端失败：{e}")))?;

        let resp = client
            .get("https://github.com/trending")
            .send()
            .await
            .map_err(|e| ToolError::Message(format!("网络请求失败：{e}")))?;

        let content = resp
            .text()
            .await
            .map_err(|e| ToolError::Message(format!("读取响应失败：{e}")))?;

        let document = scraper::Html::parse_document(&content);

        // 选择器：每行一个项目
        let row_selector = scraper::Selector::parse(".Box-row")
            .map_err(|e| ToolError::Message(format!("选择器解析失败：{e}")))?;

        let mut results = Vec::new();

        for element in document.select(&row_selector) {
            // 提取标题
            let title = element
                .select(&scraper::Selector::parse("h2 a").unwrap())
                .next()
                .map(|el| el.text().collect::<Vec<_>>().join(""))
                .unwrap_or_default()
                .trim()
                .replace("\n", "")
                .replace(" ", "")
                .to_string();

            // 提取描述
            let description = element
                .select(&scraper::Selector::parse("p.col-9").unwrap())
                .next()
                .map(|el| el.text().collect::<Vec<_>>().join(""))
                .unwrap_or_default()
                .trim()
                .to_string();

            // 提取链接
            let url = element
                .select(&scraper::Selector::parse("h2 a").unwrap())
                .next()
                .and_then(|el| el.value().attr("href"))
                .map(|href| format!("https://github.com{href}"))
                .unwrap_or_default();

            // 提取编程语言
            let language = element
                .select(&scraper::Selector::parse("span[itemprop='programmingLanguage']").unwrap())
                .next()
                .map(|el| el.text().collect::<Vec<_>>().join(""))
                .unwrap_or_default()
                .trim()
                .to_string();

            // 提取 stars 数量
            let stars = element
                .select(&scraper::Selector::parse("a[href$='/stargazers']").unwrap())
                .next()
                .map(|el| el.text().collect::<Vec<_>>().join(""))
                .unwrap_or_default()
                .trim()
                .to_string();

            // 提取 forks 数量
            let forks = element
                .select(&scraper::Selector::parse("a[href$='/forks']").unwrap())
                .next()
                .map(|el| el.text().collect::<Vec<_>>().join(""))
                .unwrap_or_default()
                .trim()
                .to_string();

            // 提取今日 stars 数量
            let today_stars = element
                .select(&scraper::Selector::parse("span.d-inline-block.float-sm-right").unwrap())
                .next()
                .map(|el| el.text().collect::<Vec<_>>().join(""))
                .unwrap_or_default()
                .trim()
                .to_string();

            results.push(GithubTrendingData {
                title,
                description,
                url,
                language,
                stars,
                forks,
                today_stars,
            });
        }

        Ok(results)
    }
}

impl Default for GithubTrendingTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GithubTrendingTool {
    fn name(&self) -> &str {
        "github_trending"
    }

    fn description(&self) -> Option<&str> {
        Some("获取 GitHub 趋势榜单，展示热门开源项目")
    }

    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::Network]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "description": "不需要参数，直接获取 GitHub 趋势榜"
        })
    }

    async fn call(&self, _input: Value) -> Result<Value, ToolError> {
        let data = self.get_github_trending().await?;
        Ok(json!({
            "total": data.len(),
            "trending_projects": data
        }))
    }
}
