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
//!
//! # 子模块
//!
//! - [`ollama`]: Ollama Provider 实现
//! - [`openai`]: OpenAI Provider 实现
//! - [`resilient`]: 带重试机制的 Provider
//! - [`router`]: 多 Provider 路由

pub mod ollama;
pub mod openai;
pub mod resilient;
pub mod router;

/// 重新导出常用 provider 实现
pub use ollama::OllamaProvider;
pub use openai::OpenAiProvider;
pub use resilient::{CancelHandle, ResilientProvider, RetryConfig};
pub use router::RouterProvider;
