//! 内置 prompt 模块
//!
//! 直接通过模块访问静态变量，例如：
//! ```rust
//! use rucora_prompt::prompts;
//!
//! // Agent 类
//! println!("{}", prompts::agent::tool::SYSTEM);
//! println!("{}", prompts::agent::tool::TEMPLATE);
//!
//! // Tool 类
//! println!("{}", prompts::tool::search::SYSTEM);
//! println!("{}", prompts::tool::search::TEMPLATE);
//!
//! // Research 类
//! println!("{}", prompts::research::default::SYSTEM);
//!
//! // Filter 类
//! println!("{}", prompts::filter::classify::SYSTEM);
//! ```

pub mod agent;
pub mod filter;
pub mod research;
pub mod tool;
