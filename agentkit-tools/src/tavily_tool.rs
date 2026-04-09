//! Tavily 搜索工具
//!
//! 使用 Tavily AI 进行搜索
//!
//! Tavily 免费版：每月 1,000 API 积分
//! 注册：https://www.tavily.com/

use agentkit_core::{
    error::ToolError,
    tool::{Tool, ToolCategory},
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

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

    /// 创建带多个 API Keys 的 Tavily 工具
    pub fn with_keys(api_keys: Vec<String>) -> Self {
        if api_keys.is_empty() {
            panic!("API Keys 不能为空");
        }
        Self { api_keys }
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

/// Tavily 搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TavilySearchResult {
    /// 答案（如果请求了）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answer: Option<String>,
    /// 搜索结果
    pub results: Vec<TavilyResult>,
    /// 查询
    pub query: String,
}

/// 单个搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TavilyResult {
    /// 标题
    pub title: String,
    /// URL
    pub url: String,
    /// 内容摘要
    pub content: String,
    /// 相关度分数
    pub score: f32,
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
            .map_err(|e| ToolError::Message(format!("解析参数失败：{}", e)))?;

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
        let client = reqwest::Client::new();
        let response = client
            .post("https://api.tavily.com/search")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ToolError::Message(format!("请求失败：{}", e)))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "未知错误".to_string());
            return Err(ToolError::Message(format!(
                "Tavily API 错误：{}",
                error_text
            )));
        }

        let search_result: Value = response
            .json()
            .await
            .map_err(|e| ToolError::Message(format!("解析 JSON 失败：{}", e)))?;

        Ok(json!({
            "success": true,
            "answer": search_result.get("answer").cloned(),
            "results": search_result.get("results").cloned().unwrap_or(json!([])),
            "query": search_result.get("query").cloned().unwrap_or(json!(args.query))
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tavily_from_env() {
        // 设置测试环境变量
        unsafe {
            std::env::set_var("TAVILY_API_KEY", "test_key_1,test_key_2");
        }

        let result = TavilyTool::from_env();
        assert!(result.is_ok());
        let tool = result.unwrap();
        assert_eq!(tool.api_keys.len(), 2);
    }

    #[test]
    fn test_tavily_args_default() {
        let args = TavilyArgs {
            query: "test".to_string(),
            max_results: default_max_results(),
            include_answer: default_true(),
            include_raw_content: false,
            search_depth: default_search_depth(),
        };

        assert_eq!(args.max_results, 5);
        assert!(args.include_answer);
        assert_eq!(args.search_depth, "basic");
    }
}


