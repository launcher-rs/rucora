# Provider 指南

Provider 是 AgentKit 与 LLM API 之间的桥梁。AgentKit 支持 8 种 LLM Provider 实现。

## Provider 总览

| Provider | 适用模型 | 特点 |
|----------|---------|------|
| `OpenAiProvider` | OpenAI GPT 系列、API 兼容服务 | 最通用，支持 Ollama/LocalAI |
| `AnthropicProvider` | Claude 系列 | 原生 Claude API |
| `GeminiProvider` | Google Gemini 系列 | Google AI |
| `OllamaProvider` | Ollama 本地模型 | 本地部署 |
| `DeepSeekProvider` | DeepSeek 系列 | 国产大模型 |
| `MoonshotProvider` | Kimi (Moonshot) | 国产大模型 |
| `OpenRouterProvider` | 多模型聚合 | 一个接口访问多模型 |
| `AzureOpenAiProvider` | Azure OpenAI | 企业级部署 |
| `ResilientProvider` | 包装以上 Provider | 弹性重试/退避 |

## 通用接口

所有 Provider 都实现 `LlmProvider` trait：

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError>;
    async fn chat_stream(&self, request: ChatRequest) -> Result<BoxStream<'static, Result<ChatStreamChunk, ProviderError>>, ProviderError>;
}
```

## OpenAiProvider

最通用的 Provider，兼容所有 OpenAI API 格式的服务。

### 使用方式

```rust
use agentkit::provider::OpenAiProvider;

// 从环境变量加载（OPENAI_API_KEY）
let provider = OpenAiProvider::from_env()?;

// 兼容 Ollama
// export OPENAI_BASE_URL=http://localhost:11434
let provider = OpenAiProvider::from_env()?;
```

### 环境变量

| 变量 | 说明 | 示例 |
|------|------|------|
| `OPENAI_API_KEY` | API 密钥 | `sk-...` |
| `OPENAI_BASE_URL` | API 地址 | `https://api.openai.com` / `http://localhost:11434` |

### 适用模型

- OpenAI: gpt-4o, gpt-4o-mini, gpt-3.5-turbo
- Ollama: qwen3.5:9b, llama3, mistral 等
- 其他兼容 OpenAI 格式的服务

---

## AnthropicProvider

Anthropic Claude 系列模型的 Provider。

### 使用方式

```rust
use agentkit::provider::AnthropicProvider;

let provider = AnthropicProvider::from_env()?;
```

### 环境变量

| 变量 | 说明 |
|------|------|
| `ANTHROPIC_API_KEY` | API 密钥 |
| `ANTHROPIC_BASE_URL` | 自定义 API 地址（可选） |

### 适用模型

- claude-3-5-sonnet-20241022
- claude-3-opus-20240229
- claude-3-haiku-20240307

---

## GeminiProvider

Google Gemini 系列模型的 Provider。

### 使用方式

```rust
use agentkit::provider::GeminiProvider;

let provider = GeminiProvider::from_env()?;
```

### 环境变量

| 变量 | 说明 |
|------|------|
| `GOOGLE_API_KEY` | API 密钥 |

### 适用模型

- gemini-1.5-pro
- gemini-1.5-flash
- gemini-2.0-flash

---

## OllamaProvider

Ollama 本地模型的 Provider（也可以通过 `OpenAiProvider` + `OPENAI_BASE_URL` 使用）。

### 使用方式

```rust
use agentkit::provider::OllamaProvider;

let provider = OllamaProvider::from_env();
// 默认连接 http://localhost:11434
```

### 环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `OLLAMA_BASE_URL` | Ollama 地址 | `http://localhost:11434` |

---

## DeepSeekProvider / MoonshotProvider / OpenRouterProvider

国产和聚合 Provider。

```rust
use agentkit::provider::{DeepSeekProvider, MoonshotProvider, OpenRouterProvider};

let deepseek = DeepSeekProvider::from_env()?;
let moonshot = MoonshotProvider::from_env()?;
let openrouter = OpenRouterProvider::from_env()?;
```

### 环境变量

| Provider | 环境变量 |
|----------|---------|
| DeepSeek | `DEEPSEEK_API_KEY` |
| Moonshot | `MOONSHOT_API_KEY` |
| OpenRouter | `OPENROUTER_API_KEY` |

---

## AzureOpenAiProvider

Azure OpenAI 企业级部署的 Provider。

### 使用方式

```rust
use agentkit::provider::AzureOpenAiProvider;

let provider = AzureOpenAiProvider::from_env()?;
```

### 环境变量

| 变量 | 说明 |
|------|------|
| `AZURE_OPENAI_API_KEY` | API 密钥 |
| `AZURE_OPENAI_BASE_URL` | Azure 端点 URL |
| `AZURE_OPENAI_DEPLOYMENT` | 部署名称 |

---

## ResilientProvider（弹性 Provider）

包装任何 Provider，提供自动重试、退避和取消功能。

### 配置

```rust
use agentkit::provider::{ResilientProvider, RetryConfig, OpenAiProvider};
use std::time::Duration;

let inner = OpenAiProvider::from_env()?;

let provider = ResilientProvider::builder()
    .inner(inner)
    .retry_config(RetryConfig {
        max_retries: 3,
        initial_backoff: Duration::from_secs(1),
        max_backoff: Duration::from_secs(30),
        backoff_multiplier: 2.0,
        jitter: true,  // 启用抖动，避免重试风暴
    })
    .cancel_handle(cancel_handle)
    .build();
```

### 重试策略

| 参数 | 说明 | 推荐值 |
|------|------|--------|
| `max_retries` | 最大重试次数 | 3 |
| `initial_backoff` | 初始退避时间 | 1s |
| `max_backoff` | 最大退避时间 | 30s |
| `backoff_multiplier` | 退避倍数 | 2.0 |
| `jitter` | 启用抖动 | true |

---

## 自定义 Provider

实现 `LlmProvider` trait 即可创建自定义 Provider：

```rust
use agentkit::prelude::LlmProvider;
use agentkit::provider::types::{ChatRequest, ChatResponse, ChatStreamChunk};
use agentkit::error::ProviderError;
use futures_util::stream::BoxStream;

struct MyCustomProvider;

#[async_trait]
impl LlmProvider for MyCustomProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        // 实现你的逻辑
        todo!()
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<BoxStream<'static, Result<ChatStreamChunk, ProviderError>>, ProviderError> {
        // 实现流式逻辑
        todo!()
    }
}
```

---

## HTTP 超时配置

所有 Provider 默认：
- 请求超时：120 秒
- 连接超时：15 秒

---

## 选择指南

| 场景 | 推荐 Provider |
|------|--------------|
| OpenAI 官方 API | `OpenAiProvider` |
| Ollama 本地模型 | `OpenAiProvider` + `OPENAI_BASE_URL` |
| Claude 系列 | `AnthropicProvider` |
| Gemini 系列 | `GeminiProvider` |
| 国产模型（DeepSeek） | `DeepSeekProvider` |
| 国产模型（Kimi） | `MoonshotProvider` |
| 多模型聚合 | `OpenRouterProvider` |
| 企业级 Azure | `AzureOpenAiProvider` |
| 需要自动重试 | `ResilientProvider` 包装 |
