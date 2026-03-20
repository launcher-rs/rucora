//! Provider（模型提供者）相关实现。
//!
//! 该模块参考 `zeroclaw/src/providers` 的组织方式进行拆分：
//! - `router`：路由 provider（按模型名/前缀分发到不同 provider）
//! - `mock`：用于测试/示例的 mock provider
//! - `openai`：OpenAI provider（真实 HTTP 调用）
//! - `ollama`：Ollama provider（本地模型）

pub mod ollama;
pub mod openai;
pub mod resilient;
pub mod router;

/// 重新导出常用 provider 实现。
pub use ollama::OllamaProvider;
pub use openai::OpenAiProvider;
pub use resilient::{CancelHandle, ResilientProvider, RetryConfig};
pub use router::RouterProvider;
