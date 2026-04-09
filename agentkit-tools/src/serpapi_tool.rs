//! SerpAPI 搜索工具
//!
//! 使用 SerpAPI 进行 Google 搜索
//!
//! SerpAPI 免费版：每月 250 次免费搜索
//! 注册：https://serpapi.com/

use agentkit_core::{
    error::ToolError,
    tool::{Tool, ToolCategory},
};
use async_trait::async_trait;
use backon::{ExponentialBuilder, Retryable};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;

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

    /// 创建带多个 API Keys 的 SerpAPI 工具
    pub fn with_keys(api_keys: Vec<String>) -> Self {
        if api_keys.is_empty() {
            panic!("API Keys 不能为空");
        }
        Self { api_keys }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serpapi_from_env() {
        // 设置测试环境变量
        unsafe {
            std::env::set_var("SERPAPI_API_KEY", "test_key_1,test_key_2");
        }

        let result = SerpapiTool::from_env();
        assert!(result.is_ok());
        let tool = result.unwrap();
        assert_eq!(tool.api_keys.len(), 2);
    }
}
