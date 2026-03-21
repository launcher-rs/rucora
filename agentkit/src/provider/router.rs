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
use std::sync::Mutex;
use std::time::{Duration, Instant};

use agentkit_core::{
    error::ProviderError,
    provider::types::{ChatRequest, ChatResponse, ChatStreamChunk},
    provider::LlmProvider,
};
use async_trait::async_trait;
use futures_util::stream::BoxStream;

/// 用于在多个 provider 之间做路由的 Provider。
pub struct RouterProvider {
    providers: HashMap<String, Vec<Arc<dyn LlmProvider>>>,
    default_provider: String,

    // 轮询计数器与健康状态。
    rr: Mutex<HashMap<String, usize>>,
    cooldown_until: Mutex<HashMap<String, Instant>>,
    cooldown: Duration,
}

impl RouterProvider {
    /// 创建 RouterProvider。
    ///
    /// - `default_provider`：当 model 不带前缀或前缀未命中时使用。
    pub fn new(default_provider: impl Into<String>) -> Self {
        Self {
            providers: HashMap::new(),
            default_provider: default_provider.into(),
            rr: Mutex::new(HashMap::new()),
            cooldown_until: Mutex::new(HashMap::new()),
            cooldown: Duration::from_secs(10),
        }
    }

    /// 设置 provider 失败后的冷却时间（默认 10 秒）。
    pub fn with_cooldown(mut self, cooldown: Duration) -> Self {
        self.cooldown = cooldown;
        self
    }

    /// 注册一个 provider。
    pub fn register<P>(mut self, name: impl Into<String>, provider: P) -> Self
    where
        P: LlmProvider + 'static,
    {
        let name = name.into();
        self.providers
            .entry(name)
            .or_default()
            .push(Arc::new(provider));
        self
    }

    /// 注册一个 provider（Arc 版本）。
    pub fn register_arc(mut self, name: impl Into<String>, provider: Arc<dyn LlmProvider>) -> Self {
        let name = name.into();
        self.providers.entry(name).or_default().push(provider);
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

    fn is_cooled_down(&self, provider_name: &str) -> bool {
        let map = self.cooldown_until.lock().unwrap();
        if let Some(until) = map.get(provider_name) {
            return Instant::now() < *until;
        }
        false
    }

    fn mark_failure(&self, provider_name: &str) {
        let mut map = self.cooldown_until.lock().unwrap();
        map.insert(provider_name.to_string(), Instant::now() + self.cooldown);
    }

    fn next_index(&self, provider_name: &str, len: usize) -> usize {
        let mut rr = self.rr.lock().unwrap();
        let entry = rr.entry(provider_name.to_string()).or_insert(0);
        let idx = if len == 0 { 0 } else { *entry % len };
        *entry = entry.wrapping_add(1);
        idx
    }

    fn providers_for<'a>(&'a self, provider_name: &str) -> Option<&'a [Arc<dyn LlmProvider>]> {
        self.providers.get(provider_name).map(|v| v.as_slice())
    }

    async fn select_provider(
        &self,
        provider_name: &str,
    ) -> Result<(String, Arc<dyn LlmProvider>), ProviderError> {
        let requested = if self.providers.contains_key(provider_name) {
            provider_name
        } else {
            &self.default_provider
        };

        if self.providers_for(requested).is_none() {
            return Err(ProviderError::Message(format!(
                "router provider 未找到 provider：{}（default={}）",
                provider_name, self.default_provider
            )));
        }

        // 如果请求的 provider 正在冷却，尝试直接回退默认 provider。
        if requested != self.default_provider.as_str() && self.is_cooled_down(requested) {
            if self.providers_for(&self.default_provider).is_some()
                && !self.is_cooled_down(&self.default_provider)
            {
                let list = self.providers_for(&self.default_provider).unwrap();
                let idx = self.next_index(&self.default_provider, list.len());
                return Ok((self.default_provider.clone(), list[idx].clone()));
            }
        }

        let list = self.providers_for(requested).unwrap();
        let idx = self.next_index(requested, list.len());
        Ok((requested.to_string(), list[idx].clone()))
    }

    async fn chat_with_fallback(
        &self,
        provider_name: &str,
        request: ChatRequest,
    ) -> Result<ChatResponse, ProviderError> {
        // 尝试：指定 provider -> 默认 provider。
        let mut order: Vec<String> = Vec::new();
        order.push(provider_name.to_string());
        if provider_name != self.default_provider {
            order.push(self.default_provider.clone());
        }

        let mut last_err: Option<ProviderError> = None;
        for name in order {
            if self.is_cooled_down(&name) {
                continue;
            }
            let (picked_name, provider) = self.select_provider(&name).await?;
            match provider.chat(request.clone()).await {
                Ok(v) => return Ok(v),
                Err(e) => {
                    self.mark_failure(&picked_name);
                    last_err = Some(e);
                }
            }
        }
        Err(last_err.unwrap_or_else(|| {
            ProviderError::Message("router provider: no available provider".to_string())
        }))
    }

    fn stream_with_fallback(
        &self,
        provider_name: &str,
        request: ChatRequest,
    ) -> Result<BoxStream<'static, Result<ChatStreamChunk, ProviderError>>, ProviderError> {
        // stream_chat 是同步返回 stream 的接口；这里做“最佳努力”：
        // - 先尝试指定 provider
        // - 如果 stream_chat 直接返回 Err，再回退默认 provider
        let order = if provider_name == self.default_provider {
            vec![provider_name.to_string()]
        } else {
            vec![provider_name.to_string(), self.default_provider.clone()]
        };

        let mut last_err: Option<ProviderError> = None;
        for name in order {
            let Some(list) = self.providers.get(&name) else {
                continue;
            };
            if list.is_empty() {
                continue;
            }
            let idx = self.next_index(&name, list.len());
            match list[idx].stream_chat(request.clone()) {
                Ok(s) => return Ok(s),
                Err(e) => last_err = Some(e),
            }
        }
        Err(last_err.unwrap_or_else(|| {
            ProviderError::Message("router provider: stream_chat not available".to_string())
        }))
    }
}

#[async_trait]
impl LlmProvider for RouterProvider {
    async fn chat(&self, mut request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let model = request.model.clone().unwrap_or_else(|| "".to_string());
        let (provider_name, resolved_model) = self.resolve_model(&model);

        // 将解析后的 model 回写，避免底层 provider 再次处理前缀。
        if !resolved_model.is_empty() {
            request.model = Some(resolved_model.to_string());
        }

        self.chat_with_fallback(&provider_name, request).await
    }

    fn stream_chat(
        &self,
        mut request: ChatRequest,
    ) -> Result<BoxStream<'static, Result<ChatStreamChunk, ProviderError>>, ProviderError> {
        let model = request.model.clone().unwrap_or_else(|| "".to_string());
        let (provider_name, resolved_model) = self.resolve_model(&model);

        if !resolved_model.is_empty() {
            request.model = Some(resolved_model.to_string());
        }

        self.stream_with_fallback(&provider_name, request)
    }
}
