//! Router Provider。
//!
//! 参考 `zeroclaw/src/providers/router.rs`：
//! - 将请求根据 `model` 字符串解析后分发到不同的底层 provider
//!
//! 在本项目的简化约定中：
//! - `ChatRequest.model` 允许使用 `"provider:model"` 形式
//!   - 例如：`"openai:gpt-4o-mini"`、`"ollama:llama3"`
//! - 如果不带前缀，则使用默认 provider。

use std::collections::HashMap;
use std::sync::Arc;

use agentkit_core::{
    error::ProviderError,
    provider::LlmProvider,
    provider::types::{ChatRequest, ChatResponse},
};
use async_trait::async_trait;

/// 用于在多个 provider 之间做路由的 Provider。
pub struct RouterProvider {
    providers: HashMap<String, Arc<dyn LlmProvider>>,
    default_provider: String,
}

impl RouterProvider {
    /// 创建 RouterProvider。
    ///
    /// - `default_provider`：当 model 不带前缀或前缀未命中时使用。
    pub fn new(default_provider: impl Into<String>) -> Self {
        Self {
            providers: HashMap::new(),
            default_provider: default_provider.into(),
        }
    }

    /// 注册一个 provider。
    pub fn register<P>(mut self, name: impl Into<String>, provider: P) -> Self
    where
        P: LlmProvider + 'static,
    {
        self.providers.insert(name.into(), Arc::new(provider));
        self
    }

    /// 解析 `model`：
    /// - 输入：`provider:model` 或 `model`
    /// - 输出：`(provider_name, resolved_model)`
    fn resolve_model<'a>(&self, model: &'a str) -> (String, &'a str) {
        if let Some((provider, rest)) = model.split_once(':') {
            if !provider.is_empty() && !rest.is_empty() {
                return (provider.to_string(), rest);
            }
        }
        (self.default_provider.clone(), model)
    }

    /// 选择具体 provider。
    fn select_provider(&self, provider_name: &str) -> Result<Arc<dyn LlmProvider>, ProviderError> {
        if let Some(p) = self.providers.get(provider_name) {
            return Ok(p.clone());
        }

        // 找不到指定 provider 时，回退到默认 provider。
        self.providers
            .get(&self.default_provider)
            .cloned()
            .ok_or_else(|| {
                ProviderError::Message(format!(
                    "router provider 未找到 provider：{}（default={}）",
                    provider_name, self.default_provider
                ))
            })
    }
}

#[async_trait]
impl LlmProvider for RouterProvider {
    async fn chat(&self, mut request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let model = request.model.clone().unwrap_or_else(|| "".to_string());
        let (provider_name, resolved_model) = self.resolve_model(&model);

        let provider = self.select_provider(&provider_name)?;

        // 将解析后的 model 回写，避免底层 provider 再次处理前缀。
        if !resolved_model.is_empty() {
            request.model = Some(resolved_model.to_string());
        }

        provider.chat(request).await
    }
}
