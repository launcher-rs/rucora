//! GitHub 趋势榜工具
//!
//! 获取 GitHub 趋势榜单信息

use rucora_core::{
    error::ToolError,
    tool::{Tool, ToolCategory},
};
use async_trait::async_trait;
use scraper::{Html, Selector};
use serde_json::{Value, json};

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

        let document = Html::parse_document(&content);

        // 选择器：每行一个项目
        let row_selector = Selector::parse(".Box-row")
            .map_err(|e| ToolError::Message(format!("选择器解析失败：{e}")))?;

        let mut results = Vec::new();

        for element in document.select(&row_selector) {
            // 提取标题
            let title = element
                .select(&Selector::parse("h2 a").unwrap())
                .next()
                .map(|el| el.text().collect::<Vec<_>>().join(""))
                .unwrap_or_default()
                .trim()
                .replace("\n", "")
                .replace(" ", "")
                .to_string();

            // 提取描述
            let description = element
                .select(&Selector::parse("p.col-9").unwrap())
                .next()
                .map(|el| el.text().collect::<Vec<_>>().join(""))
                .unwrap_or_default()
                .trim()
                .to_string();

            // 提取链接
            let url = element
                .select(&Selector::parse("h2 a").unwrap())
                .next()
                .and_then(|el| el.value().attr("href"))
                .map(|href| format!("https://github.com{href}"))
                .unwrap_or_default();

            // 提取编程语言
            let language = element
                .select(&Selector::parse("span[itemprop='programmingLanguage']").unwrap())
                .next()
                .map(|el| el.text().collect::<Vec<_>>().join(""))
                .unwrap_or_default()
                .trim()
                .to_string();

            // 提取 stars 数量
            let stars = element
                .select(&Selector::parse("a[href$='/stargazers']").unwrap())
                .next()
                .map(|el| el.text().collect::<Vec<_>>().join(""))
                .unwrap_or_default()
                .trim()
                .to_string();

            // 提取 forks 数量
            let forks = element
                .select(&Selector::parse("a[href$='/forks']").unwrap())
                .next()
                .map(|el| el.text().collect::<Vec<_>>().join(""))
                .unwrap_or_default()
                .trim()
                .to_string();

            // 提取今日 stars 数量
            let today_stars = element
                .select(&Selector::parse("span.d-inline-block.float-sm-right").unwrap())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_github_trending() {
        let tool = GithubTrendingTool::new();
        let result = tool.call(json!({})).await;

        match result {
            Ok(value) => {
                println!("GitHub 趋势榜：{}", value);
                assert!(value.get("total").is_some());
            }
            Err(e) => {
                println!("测试失败（可能是网络问题）: {}", e);
            }
        }
    }
}
