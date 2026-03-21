//! Provider（模型提供者）实现模块
//!
//! # 概述
//!
//! 本模块包含各种 LLM Provider 的具体实现，用于与不同的大语言模型服务交互。
//!
//! # 支持的 Provider
//!
//! | Provider | 说明 | 使用场景 |
//! |----------|------|----------|
//! | [`OpenAiProvider`] | OpenAI API 兼容服务 | GPT-4、GPT-3.5 等 |
//! | [`OllamaProvider`] | Ollama 本地模型 | 本地部署、隐私敏感 |
//! | [`AnthropicProvider`] | Anthropic Claude 模型 | Claude 3.5/3 系列 |
//! | [`GeminiProvider`] | Google Gemini 模型 | Gemini 1.5 Pro/Flash |
//! | [`AzureOpenAiProvider`] | Azure OpenAI 服务 | 企业级 GPT 部署 |
//! | [`OpenRouterProvider`] | 多模型聚合服务 | 70+ 模型一键访问 |
//! | [`DeepSeekProvider`] | DeepSeek 模型 | DeepSeek-V3/R1 |
//! | [`MoonshotProvider`] | 月之暗面 Kimi 模型 | Kimi 智能助手 |
//! | [`RouterProvider`] | 多 Provider 路由 | 负载均衡、故障转移 |
//! | [`ResilientProvider`] | 带重试机制的 Provider | 生产环境、高可用 |
//!
//! # 使用示例
//!
//! ## OpenAiProvider
//!
//! ```rust,no_run
//! use agentkit::provider::OpenAiProvider;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // 从环境变量加载配置
//! let provider = OpenAiProvider::from_env()?;
//!
//! // 或手动配置
//! let provider = OpenAiProvider::new(
//!     "https://api.openai.com/v1",
//!     "sk-...",
//! ).with_default_model("gpt-4o-mini");
//! # Ok(())
//! # }
//! ```
//!
//! ## OllamaProvider
//!
//! ```rust,no_run
//! use agentkit::provider::OllamaProvider;
//!
//! // 从环境变量加载
//! let provider = OllamaProvider::from_env();
//!
//! // 或手动配置
//! let provider = OllamaProvider::new("http://localhost:11434")
//!     .with_default_model("llama3");
//! ```
//!
//! ## AnthropicProvider
//!
//! ```rust,no_run
//! use agentkit::provider::AnthropicProvider;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let provider = AnthropicProvider::from_env()?;
//!
//! let provider = provider.with_default_model("claude-3-5-sonnet-20241022");
//! # Ok(())
//! # }
//! ```
//!
//! ## GeminiProvider
//!
//! ```rust,no_run
//! use agentkit::provider::GeminiProvider;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let provider = GeminiProvider::from_env()?;
//!
//! let provider = provider.with_default_model("gemini-1.5-pro");
//! # Ok(())
//! # }
//! ```
//!
//! ## AzureOpenAiProvider
//!
//! ```rust,no_run
//! use agentkit::provider::AzureOpenAiProvider;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let provider = AzureOpenAiProvider::from_env()?;
//! # Ok(())
//! # }
//! ```
//!
//! ## OpenRouterProvider
//!
//! ```rust,no_run
//! use agentkit::provider::OpenRouterProvider;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let provider = OpenRouterProvider::from_env()?;
//!
//! // 使用 Claude 模型
//! let provider = provider.with_default_model("anthropic/claude-3.5-sonnet");
//! # Ok(())
//! # }
//! ```
//!
//! ## DeepSeekProvider
//!
//! ```rust,no_run
//! use agentkit::provider::DeepSeekProvider;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let provider = DeepSeekProvider::from_env()?;
//!
//! let provider = provider.with_default_model("deepseek-chat");
//! # Ok(())
//! # }
//! ```
//!
//! ## MoonshotProvider
//!
//! ```rust,no_run
//! use agentkit::provider::MoonshotProvider;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let provider = MoonshotProvider::from_env()?;
//!
//! let provider = provider.with_default_model("moonshot-v1-32k");
//! # Ok(())
//! # }
//! ```
//!
//! ## RouterProvider
//!
//! ```rust,no_run
//! use agentkit::provider::{RouterProvider, OpenAiProvider, OllamaProvider};
//!
//! let openai = OpenAiProvider::new("https://api.openai.com/v1", "sk-...");
//! let ollama = OllamaProvider::new("http://localhost:11434");
//!
//! // 创建路由：默认使用 ollama，带 openai 前缀的使用 OpenAI
//! let router = RouterProvider::new("ollama")
//!     .register("openai", openai)
//!     .register("ollama", ollama);
//!
//! // 使用：
//! // - "llama3" -> 使用 ollama
//! // - "openai:gpt-4" -> 使用 openai
//! ```
//!
//! ## ResilientProvider
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use agentkit::provider::{ResilientProvider, RetryConfig, OpenAiProvider};
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let inner = OpenAiProvider::from_env()?;
//!
//! // 配置重试策略
//! let retry_config = RetryConfig::new()
//!     .with_max_retries(3)
//!     .with_base_delay_ms(100)
//!     .with_max_delay_ms(2000)
//!     .with_timeout_ms(30000);
//!
//! let resilient = ResilientProvider::new(Arc::new(inner))
//!     .with_config(retry_config);
//! # Ok(())
//! # }
//! ```
//!
//! # Provider 对比
//!
//! ## OpenAiProvider
//!
//! **优点**:
//! - 支持最新的 GPT 模型
//! - 支持 function calling
//! - 支持流式输出
//! - 支持结构化输出（JSON Schema）
//!
//! **缺点**:
//! - 需要网络连接
//! - 按 token 计费
//! - 数据隐私考虑
//!
//! ## OllamaProvider
//!
//! **优点**:
//! - 本地部署，无需网络
//! - 免费使用
//! - 数据隐私好
//! - 支持多种开源模型
//!
//! **缺点**:
//! - 需要本地资源（GPU/内存）
//! - 模型质量可能不如商业 API
//! - 部分版本不支持 function calling
//!
//! ## AnthropicProvider
//!
//! **优点**:
//! - Claude 模型强大的推理能力
//! - 支持 200K 上下文窗口
//! - 优秀的代码能力
//! - 支持 vision
//!
//! **缺点**:
//! - API 格式与 OpenAI 不同
//! - 需要单独配置
//!
//! ## GeminiProvider
//!
//! **优点**:
//! - Google 强大的多模态能力
//! - 支持 1M+ 上下文窗口
//! - 支持原生 vision
//! - 免费额度
//!
//! **缺点**:
//! - API 格式独特
//! - 国内访问可能受限
//!
//! ## AzureOpenAiProvider
//!
//! **优点**:
//! - 企业级 SLA 保障
//! - 数据隐私合规
//! - 专有网络支持
//! - 与 Azure 生态深度集成
//!
//! **缺点**:
//! - 配置相对复杂
//! - 需要 Azure 账户
//!
//! ## OpenRouterProvider
//!
//! **优点**:
//! - 一个 API 访问 70+ 模型
//! - 支持模型路由和 fallback
//! - 统一的计费接口
//! - 支持开源模型
//!
//! **缺点**:
//! - 额外的服务层
//! - 部分模型延迟略高
//!
//! ## DeepSeekProvider
//!
//! **优点**:
//! - 高性价比
//! - 强大的代码和推理能力
//! - 支持 128K 上下文
//!
//! **缺点**:
//! - 相对较新的服务
//!
//! ## MoonshotProvider
//!
//! **优点**:
//! - Kimi 长文本能力
//! - 支持 128K 上下文
//! - 中文优化
//!
//! **缺点**:
//! - 主要面向中文场景
//!
//! ## RouterProvider
//!
//! **优点**:
//! - 支持多 Provider 负载均衡
//! - 支持故障转移
//! - 支持冷却时间
//! - 统一接口
//!
//! **缺点**:
//! - 配置稍复杂
//!
//! ## ResilientProvider
//!
//! **优点**:
//! - 自动重试
//! - 指数退避
//! - 超时控制
//! - 可取消流式输出
//!
//! **缺点**:
//! - 增加延迟（重试时）
//!
//! # 环境变量
//!
//! | 变量名 | 说明 | 示例 |
//! |--------|------|------|
//! | `OPENAI_API_KEY` | OpenAI API Key | `sk-...` |
//! | `OPENAI_BASE_URL` | OpenAI Base URL | `https://api.openai.com/v1` |
//! | `OLLAMA_BASE_URL` | Ollama Base URL | `http://localhost:11434` |
//! | `ANTHROPIC_API_KEY` | Anthropic API Key | `sk-ant-...` |
//! | `GOOGLE_API_KEY` | Google API Key | `...` |
//! | `GEMINI_API_KEY` | Gemini API Key | `...` |
//! | `AZURE_OPENAI_API_KEY` | Azure OpenAI API Key | `...` |
//! | `AZURE_OPENAI_ENDPOINT` | Azure OpenAI Endpoint | `https://...azure.com` |
//! | `OPENROUTER_API_KEY` | OpenRouter API Key | `sk-or-...` |
//! | `DEEPSEEK_API_KEY` | DeepSeek API Key | `sk-...` |
//! | `MOONSHOT_API_KEY` | Moonshot API Key | `sk-...` |
//!
//! # 子模块
//!
//! - [`anthropic`]: Anthropic Provider 实现
//! - [`azure_openai`]: Azure OpenAI Provider 实现
//! - [`deepseek`]: DeepSeek Provider 实现
//! - [`gemini`]: Google Gemini Provider 实现
//! - [`helpers`]: Provider 辅助函数
//! - [`moonshot`]: Moonshot Provider 实现
//! - [`ollama`]: Ollama Provider 实现
//! - [`openai`]: OpenAI Provider 实现
//! - [`openrouter`]: OpenRouter Provider 实现
//! - [`resilient`]: 带重试机制的 Provider
//! - [`router`]: 多 Provider 路由

pub mod anthropic;
pub mod azure_openai;
pub mod deepseek;
pub mod gemini;
pub mod helpers;
pub mod moonshot;
pub mod ollama;
pub mod openai;
pub mod openrouter;
pub mod resilient;
pub mod router;

/// 重新导出常用 provider 实现
pub use anthropic::AnthropicProvider;
pub use azure_openai::AzureOpenAiProvider;
pub use deepseek::DeepSeekProvider;
pub use gemini::GeminiProvider;
pub use moonshot::MoonshotProvider;
pub use ollama::OllamaProvider;
pub use openai::OpenAiProvider;
pub use openrouter::OpenRouterProvider;
pub use resilient::{CancelHandle, ResilientProvider, RetryConfig};
pub use router::RouterProvider;
