# AgentKit Providers

LLM Providers for AgentKit.

## Overview

This crate contains concrete implementations of various LLM providers for AgentKit, enabling interaction with different large language model services.

## Supported Providers

| Provider | Description |
|----------|-------------|
| OpenAiProvider | OpenAI GPT series models |
| AnthropicProvider | Anthropic Claude models |
| GeminiProvider | Google Gemini models |
| AzureOpenAiProvider | Azure OpenAI service |
| OllamaProvider | Ollama local models |
| OpenRouterProvider | OpenRouter multi-model service |
| DeepSeekProvider | DeepSeek models |
| MoonshotProvider | Moonshot (Kimi) models |
| ResilientProvider | Provider with retry logic |

## Installation

```toml
[dependencies]
agentkit-providers = "0.1"
```

Or via the main AgentKit crate:

```toml
[dependencies]
agentkit = { version = "0.1", features = ["providers"] }
```

## Usage

### OpenAI Provider

```rust
use agentkit_providers::OpenAiProvider;
use agentkit_core::provider::LlmProvider;

let provider = OpenAiProvider::from_env()?;

let request = ChatRequest::from_user_text("Hello");
let response = provider.chat(request).await?;
println!("{}", response.message.content);
```

### Anthropic Provider

```rust
use agentkit_providers::AnthropicProvider;

let provider = AnthropicProvider::from_env()?
    .with_default_model("claude-3-5-sonnet-20241022");
```

### Gemini Provider

```rust
use agentkit_providers::GeminiProvider;

let provider = GeminiProvider::from_env()?
    .with_default_model("gemini-1.5-pro");
```

### Resilient Provider (with retry)

```rust
use agentkit_providers::{OpenAiProvider, ResilientProvider, RetryConfig};
use std::sync::Arc;

let inner = Arc::new(OpenAiProvider::from_env()?);
let provider = ResilientProvider::new(inner)
    .with_config(RetryConfig::new()
        .with_max_retries(3)
        .with_base_delay_ms(200));
```

## Features

| Feature | Description |
|---------|-------------|
| `openai` | OpenAI Provider (default) |
| `anthropic` | Anthropic Provider |
| `gemini` | Google Gemini Provider |
| `azure-openai` | Azure OpenAI Provider |
| `ollama` | Ollama Provider |
| `openrouter` | OpenRouter Provider |
| `deepseek` | DeepSeek Provider |
| `moonshot` | Moonshot Provider |
| `resilient` | Resilient Provider wrapper |
| `all` | Enable all providers |

## Environment Variables

| Variable | Description |
|----------|-------------|
| `OPENAI_API_KEY` | OpenAI API Key |
| `OPENAI_BASE_URL` | OpenAI Base URL |
| `ANTHROPIC_API_KEY` | Anthropic API Key |
| `GOOGLE_API_KEY` / `GEMINI_API_KEY` | Google/Gemini API Key |
| `AZURE_OPENAI_API_KEY` | Azure OpenAI API Key |
| `AZURE_OPENAI_ENDPOINT` | Azure OpenAI Endpoint |
| `OLLAMA_BASE_URL` | Ollama Base URL |
| `OPENROUTER_API_KEY` | OpenRouter API Key |
| `DEEPSEEK_API_KEY` | DeepSeek API Key |
| `MOONSHOT_API_KEY` | Moonshot API Key |

## License

MIT
