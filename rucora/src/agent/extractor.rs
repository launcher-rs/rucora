//! Extractor（结构化数据提取）模块
//!
//! # 概述
//!
//! Extractor 用于从非结构化文本中提取结构化数据。
//! 它利用 LLM 的理解能力，根据定义的 schema 提取信息并返回强类型的结构体。
//!
//! # 工作原理
//!
//! 1. 定义目标结构体（需实现 `serde::Deserialize`、`serde::Serialize` 和 `schemars::JsonSchema`）
//! 2. 创建 Extractor，它会自动生成一个 `submit` 工具，其参数 schema 为目标结构体
//! 3. LLM 分析输入文本，调用 `submit` 工具提交提取的数据
//! 4. 返回强类型的提取结果
//!
//! # 使用示例
//!
//! ## 基本使用
//!
//! ```rust,no_run
//! use rucora::agent::extractor::Extractor;
//! use rucora::provider::OpenAiProvider;
//! use serde::{Deserialize, Serialize};
//! use schemars::JsonSchema;
//!
//! #[derive(Debug, Deserialize, Serialize, JsonSchema)]
//! struct Person {
//!     /// 姓名
//!     name: Option<String>,
//!     /// 年龄
//!     age: Option<u8>,
//!     /// 职业
//!     profession: Option<String>,
//! }
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let provider = OpenAiProvider::from_env()?;
//!
//! // 创建 Extractor
//! let extractor = Extractor::<Person>::builder(provider, "gpt-4o-mini")
//!     .build();
//!
//! // 提取结构化数据
//! let person = extractor.extract("John Doe 是 30 岁的软件工程师")
//!     .await?;
//!
//! println!("姓名：{:?}", person.name);
//! println!("年龄：{:?}", person.age);
//! println!("职业：{:?}", person.profession);
//! # Ok(())
//! # }
//! ```
//!
//! ## 带 Usage 追踪
//!
//! ```rust,no_run
//! # use rucora::provider::OpenAiProvider;
//! # use serde::{Deserialize, Serialize};
//! # use schemars::JsonSchema;
//! #
//! # #[derive(Debug, Deserialize, Serialize, JsonSchema)]
//! # struct Person {
//! #     name: Option<String>,
//! #     age: Option<u8>,
//! #     profession: Option<String>,
//! # }
//! #
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let provider = OpenAiProvider::from_env()?;
//!
//! let extractor = Extractor::<Person>::builder(provider, "gpt-4o-mini")
//!     .build();
//!
//! let response = extractor.extract_with_usage("Jane Smith 是数据科学家")
//!     .await?;
//!
//! println!("提取的数据：{:?}", response.data);
//! println!("输入 token: {}", response.usage.input_tokens);
//! println!("输出 token: {}", response.usage.output_tokens);
//! # Ok(())
//! # }
//! ```
//!
//! ## 自定义提示词
//!
//! ```rust,no_run
//! # use rucora::provider::OpenAiProvider;
//! # use serde::{Deserialize, Serialize};
//! # use schemars::JsonSchema;
//! #
//! # #[derive(Debug, Deserialize, Serialize, JsonSchema)]
//! # struct Person {
//! #     name: Option<String>,
//! #     age: Option<u8>,
//! #     profession: Option<String>,
//! # }
//! #
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let provider = OpenAiProvider::from_env()?;
//!
//! let extractor = Extractor::<Person>::builder(provider, "gpt-4o-mini")
//!     .preamble("只提取明确提到的信息，不要推测。")
//!     .retries(3)
//!     .build();
//!
//! let person = extractor.extract("John Doe 是 30 岁的软件工程师")
//!     .await?;
//! # Ok(())
//! # }
//! ```

use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::marker::PhantomData;

use rucora_core::provider::LlmProvider;
use rucora_core::provider::types::{ChatMessage, ChatRequest, LlmParams};
use rucora_core::tool::Tool;

use crate::agent::ToolAgent;

/// 提取响应，包含提取的数据和 token 使用情况
#[derive(Debug, Clone)]
pub struct ExtractionResponse<T> {
    /// 提取的结构化数据
    pub data: T,
    /// Token 使用情况（如果有）
    pub usage: Option<TokenUsage>,
}

/// Token 使用统计
#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    /// 输入 token 数
    pub input_tokens: u32,
    /// 输出 token 数
    pub output_tokens: u32,
    /// 总 token 数
    pub total_tokens: u32,
}

/// 提取错误
#[derive(Debug, thiserror::Error)]
pub enum ExtractionError {
    #[error("未提取到数据")]
    NoData,

    #[error("反序列化提取的数据失败：{0}")]
    DeserializationError(#[from] serde_json::Error),

    #[error("LLM 调用失败：{0}")]
    LlmError(String),

    /// 当配置了重试次数且所有尝试都失败时返回。
    #[error("达到最大重试次数")]
    MaxRetriesExceeded,
}

/// Extractor 用于从非结构化文本中提取结构化数据
///
/// # 类型参数
///
/// - `T`: 目标数据结构类型，必须实现 `JsonSchema`、`Deserialize`、`Serialize`
pub struct Extractor<T>
where
    T: JsonSchema + for<'a> Deserialize<'a> + Serialize + Send + Sync + 'static,
{
    agent: ToolAgent<Box<dyn LlmProvider>>,
    _t: PhantomData<T>,
    retries: u32,
    /// LLM 请求参数
    llm_params: LlmParams,
}

impl<T> Extractor<T>
where
    T: JsonSchema + for<'a> Deserialize<'a> + Serialize + Send + Sync + 'static,
{
    /// 创建 Extractor 构建器
    ///
    /// # 参数
    ///
    /// - `provider`: LLM Provider
    /// - `model`: 模型名称
    pub fn builder<P>(provider: P, model: impl Into<String>) -> ExtractorBuilder<T>
    where
        P: LlmProvider + Send + Sync + 'static,
    {
        ExtractorBuilder::new(provider, model)
    }

    /// 从文本中提取结构化数据
    ///
    /// # 参数
    ///
    /// - `text`: 输入文本
    ///
    /// # 返回
    ///
    /// 返回提取的结构化数据
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// # use rucora::provider::OpenAiProvider;
    /// # use serde::{Deserialize, Serialize};
    /// # use schemars::JsonSchema;
    /// #
    /// # #[derive(Debug, Deserialize, Serialize, JsonSchema)]
    /// # struct Person {
    /// #     name: Option<String>,
    /// #     age: Option<u8>,
    /// # }
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let provider = OpenAiProvider::from_env()?;
    /// let extractor = Extractor::<Person>::builder(provider, "gpt-4o-mini").build();
    ///
    /// let person = extractor.extract("John Doe 是 30 岁").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn extract(&self, text: impl Into<String>) -> Result<T, ExtractionError> {
        self._extract_with_chat_history(text.into(), vec![]).await
    }

    /// 从文本中提取结构化数据，带 Usage 追踪
    ///
    /// # 参数
    ///
    /// - `text`: 输入文本
    ///
    /// # 返回
    ///
    /// 返回 [`ExtractionResponse`]，包含提取的数据和 usage 信息
    pub async fn extract_with_usage(
        &self,
        text: impl Into<String>,
    ) -> Result<ExtractionResponse<T>, ExtractionError> {
        self._extract_with_usage_and_chat_history(text.into(), vec![])
            .await
    }

    /// 从文本中提取结构化数据，带对话历史
    ///
    /// # 参数
    ///
    /// - `text`: 输入文本
    /// - `chat_history`: 对话历史
    ///
    /// # 返回
    ///
    /// 返回提取的结构化数据
    pub async fn extract_with_chat_history(
        &self,
        text: impl Into<String>,
        chat_history: Vec<String>,
    ) -> Result<T, ExtractionError> {
        self._extract_with_chat_history(text.into(), chat_history)
            .await
    }

    /// 内部实现：带对话历史的提取
    async fn _extract_with_chat_history(
        &self,
        text: String,
        chat_history: Vec<String>,
    ) -> Result<T, ExtractionError> {
        let mut last_error = None;

        for i in 0..=self.retries {
            tracing::debug!("提取 JSON，剩余重试次数：{}", self.retries - i);

            match self._extract_json(text.clone(), chat_history.clone()).await {
                Ok(data) => return Ok(data),
                Err(e) => {
                    tracing::warn!("第 {} 次提取失败：{:?}，重试中...", i, e);
                    last_error = Some(e);
                }
            }
        }

        match last_error {
            Some(_) if self.retries > 0 => Err(ExtractionError::MaxRetriesExceeded),
            Some(error) => Err(error),
            None => Err(ExtractionError::NoData),
        }
    }

    /// 内部实现：带 Usage 和对话历史的提取
    async fn _extract_with_usage_and_chat_history(
        &self,
        text: String,
        chat_history: Vec<String>,
    ) -> Result<ExtractionResponse<T>, ExtractionError> {
        let mut last_error = None;

        for i in 0..=self.retries {
            tracing::debug!("提取 JSON，剩余重试次数：{}", self.retries - i);

            match self
                ._extract_json_with_usage(text.clone(), chat_history.clone())
                .await
            {
                Ok((data, usage)) => {
                    return Ok(ExtractionResponse {
                        data,
                        usage: Some(usage),
                    });
                }
                Err(e) => {
                    tracing::warn!("第 {} 次提取失败：{:?}，重试中...", i, e);
                    last_error = Some(e);
                }
            }
        }

        match last_error {
            Some(_) if self.retries > 0 => Err(ExtractionError::MaxRetriesExceeded),
            Some(error) => Err(error),
            None => Err(ExtractionError::NoData),
        }
    }

    /// 内部实现：执行 JSON 提取
    async fn _extract_json(
        &self,
        text: String,
        chat_history: Vec<String>,
    ) -> Result<T, ExtractionError> {
        let (data, _usage) = self._extract_json_with_usage(text, chat_history).await?;
        Ok(data)
    }

    /// 内部实现：执行 JSON 提取并返回 Usage
    async fn _extract_json_with_usage(
        &self,
        text: String,
        chat_history: Vec<String>,
    ) -> Result<(T, TokenUsage), ExtractionError> {
        // 构建消息历史
        let mut messages = Vec::new();

        // 添加对话历史
        for msg in chat_history {
            messages.push(ChatMessage::user(msg));
        }

        // 添加当前输入
        messages.push(ChatMessage::user(text));

        // 构建请求
        let mut request = ChatRequest {
            messages,
            model: Some(self.agent.model().to_string()),
            tools: Some(self.agent.tool_registry().definitions()),
            temperature: None,
            max_tokens: None,
            response_format: None,
            metadata: None,
            top_p: None,
            top_k: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            extra: None,
        };

        // 应用 llm_params
        self.llm_params.apply_to(&mut request);

        // 调用 LLM
        let response = self
            .agent
            .provider()
            .chat(request)
            .await
            .map_err(|e| ExtractionError::LlmError(e.to_string()))?;

        // 解析工具调用
        let submit_call = response
            .tool_calls
            .iter()
            .find(|call| call.name == SUBMIT_TOOL_NAME);

        let arguments = if let Some(call) = submit_call {
            call.input.clone()
        } else {
            tracing::warn!("未找到 submit 工具调用");
            return Err(ExtractionError::NoData);
        };

        // 反序列化为目标类型
        let data: T = serde_json::from_value(arguments)?;

        // 构建 usage（如果 provider 支持）
        let usage = response
            .usage
            .map(|u| TokenUsage {
                input_tokens: u.prompt_tokens,
                output_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            })
            .unwrap_or_default();

        Ok((data, usage))
    }

    /// 获取内部的 Agent 引用
    pub fn agent(&self) -> &ToolAgent<Box<dyn LlmProvider>> {
        &self.agent
    }
}

/// Extractor 构建器
pub struct ExtractorBuilder<T>
where
    T: JsonSchema + for<'a> Deserialize<'a> + Serialize + Send + Sync + 'static,
{
    provider: Box<dyn LlmProvider>,
    model: String,
    _t: PhantomData<T>,
    retries: u32,
    preamble: Option<String>,
    /// LLM 请求参数
    llm_params: LlmParams,
}

impl<T> ExtractorBuilder<T>
where
    T: JsonSchema + for<'a> Deserialize<'a> + Serialize + Send + Sync + 'static,
{
    /// 创建新的构建器
    fn new<P>(provider: P, model: impl Into<String>) -> Self
    where
        P: LlmProvider + Send + Sync + 'static,
    {
        Self {
            provider: Box::new(provider),
            model: model.into(),
            preamble: None,
            retries: 0,
            llm_params: LlmParams::new().temperature(0.0), // 默认 temperature=0.0 以保证输出稳定
            _t: PhantomData,
        }
    }

    /// 添加额外的系统提示词
    ///
    /// # 参数
    ///
    /// - `preamble`: 额外的指令或上下文
    pub fn preamble(mut self, preamble: impl Into<String>) -> Self {
        self.preamble = Some(preamble.into());
        self
    }

    /// 设置最大重试次数
    ///
    /// # 参数
    ///
    /// - `retries`: 重试次数
    pub fn retries(mut self, retries: u32) -> Self {
        self.retries = retries;
        self
    }

    /// 设置 LLM 请求参数
    ///
    /// # 参数
    ///
    /// - `params`: LLM 参数集合
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use rucora::LlmParams;
    /// use rucora::agent::extractor::Extractor;
    /// # use rucora::provider::OpenAiProvider;
    /// # use serde::{Deserialize, Serialize};
    /// # use schemars::JsonSchema;
    /// # #[derive(Debug, Deserialize, Serialize, JsonSchema)]
    /// # struct Person { name: Option<String> }
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let provider = OpenAiProvider::from_env()?;
    /// let extractor = Extractor::<Person>::builder(provider, "gpt-4o-mini")
    ///     .llm_params(LlmParams::new().temperature(0.1).max_tokens(1024))
    ///     .build();
    /// # Ok(())
    /// # }
    /// ```
    pub fn llm_params(mut self, params: LlmParams) -> Self {
        self.llm_params = params;
        self
    }

    /// 设置 temperature
    ///
    /// # 参数
    ///
    /// - `value`: 温度值（0.0 - 2.0）
    pub fn temperature(mut self, value: f32) -> Self {
        self.llm_params.temperature = Some(value);
        self
    }

    /// 设置 top_p
    pub fn top_p(mut self, value: f32) -> Self {
        self.llm_params.top_p = Some(value);
        self
    }

    /// 设置 max_tokens
    pub fn max_tokens(mut self, value: u32) -> Self {
        self.llm_params.max_tokens = Some(value);
        self
    }

    /// 设置 frequency_penalty
    pub fn frequency_penalty(mut self, value: f32) -> Self {
        self.llm_params.frequency_penalty = Some(value);
        self
    }

    /// 设置 presence_penalty
    pub fn presence_penalty(mut self, value: f32) -> Self {
        self.llm_params.presence_penalty = Some(value);
        self
    }

    /// 设置 stop 序列
    pub fn stop(mut self, value: Vec<String>) -> Self {
        self.llm_params.stop = Some(value);
        self
    }

    /// 构建 Extractor
    ///
    /// # 返回
    ///
    /// 返回配置好的 [`Extractor`] 实例
    pub fn build(self) -> Extractor<T> {
        // 构建系统提示词
        let mut system_prompt = String::from(
            "你是一个 AI 助手，用于从文本中提取结构化数据。\n\
             你可以使用 `submit` 工具来提交提取的数据。\n\
             务必调用 `submit` 工具，即使使用默认值！\n",
        );

        if let Some(preamble) = self.preamble {
            system_prompt.push_str("\n=============== 额外指令 ===============\n");
            system_prompt.push_str(&preamble);
        }

        // 创建 Agent
        let agent = ToolAgent::builder()
            .provider(self.provider)
            .model(self.model)
            .system_prompt(system_prompt)
            .tool(SubmitTool::<T>::new())
            .max_steps(3)
            .build();

        Extractor {
            agent,
            _t: PhantomData,
            retries: self.retries,
            llm_params: self.llm_params,
        }
    }
}

/// submit 工具的名称
const SUBMIT_TOOL_NAME: &str = "submit";

/// SubmitTool - 用于提交提取的结构化数据
///
/// 这是一个特殊的工具，其参数 schema 由目标类型 `T` 的 JSON Schema 生成。
struct SubmitTool<T>
where
    T: JsonSchema + for<'a> Deserialize<'a> + Serialize + Send + Sync + 'static,
{
    _t: PhantomData<T>,
}

impl<T> SubmitTool<T>
where
    T: JsonSchema + for<'a> Deserialize<'a> + Serialize + Send + Sync + 'static,
{
    /// 创建新的 SubmitTool
    fn new() -> Self {
        Self { _t: PhantomData }
    }
}

impl<T> Default for SubmitTool<T>
where
    T: JsonSchema + for<'a> Deserialize<'a> + Serialize + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl<T> Tool for SubmitTool<T>
where
    T: JsonSchema + for<'a> Deserialize<'a> + Serialize + Send + Sync + 'static,
{
    fn name(&self) -> &str {
        SUBMIT_TOOL_NAME
    }

    fn description(&self) -> Option<&str> {
        Some("提交从文本中提取的结构化数据")
    }

    fn categories(&self) -> &'static [rucora_core::tool::ToolCategory] {
        &[rucora_core::tool::ToolCategory::Basic]
    }

    fn input_schema(&self) -> serde_json::Value {
        // 生成 T 的 JSON Schema (schemars 1.x 的 Schema 直接实现了 Serialize)
        let schema = schema_for!(T);
        serde_json::to_value(&schema).unwrap_or_else(|_| json!({}))
    }

    async fn call(
        &self,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, rucora_core::error::ToolError> {
        // SubmitTool 只是返回输入的数据
        Ok(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
    struct TestPerson {
        name: Option<String>,
        age: Option<u8>,
        profession: Option<String>,
    }

    #[test]
    fn test_submit_tool_schema() {
        let tool = SubmitTool::<TestPerson>::new();
        let schema = tool.input_schema();

        // 验证 schema 包含预期的字段
        assert!(schema.get("type").is_some());
        assert!(schema.get("properties").is_some());
    }
}
