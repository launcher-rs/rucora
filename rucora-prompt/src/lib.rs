//! rucora-prompt
//!
//! Prompt 模板库，提供内置 prompt 和自定义加载能力。
//!
//! # 快速开始
//!
//! ```rust
//! use rucora_prompt::prompts;
//!
//! // 直接使用静态变量
//! let system = prompts::agent::tool::SYSTEM;
//! let template = prompts::agent::tool::TEMPLATE;
//!
//! // 或通过函数获取
//! let tmpl = prompts::agent::tool::template();
//! ```

pub mod prompts;
pub mod template;

pub use template::PromptTemplate;
