//! 嵌入向量 Provider 模块
//!
//! 提供嵌入向量生成能力，支持多种后端：
//! - `cache`: 带缓存的嵌入向量 Provider
//! - `ollama`: Ollama 嵌入服务
//! - `openai`: OpenAI 嵌入服务

pub mod cache;
/// Ollama 嵌入 Provider
pub mod ollama;
/// OpenAI 嵌入 Provider
pub mod openai;
