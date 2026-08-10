//! 自动语音识别（ASR）抽象模块
//!
//! # 概述
//!
//! ASR 用于将音频/视频文件转换为带时间戳与说话人的文本，支持：
//! - 单文件转写（transcribe）
//! - 分段结果（segments）
//! - 说话人分离（可选）
//!
//! 在 core 层，我们只定义抽象接口，不绑定具体实现。
//!
//! # 核心类型
//!
//! ## AsrProvider trait
//!
//! [`crate::asr::AsrProvider`] trait 定义了语音识别的接口：
//!
//! ```rust,no_run
//! use rucora_core::asr::{AsrProvider, AsrRequest};
//! use rucora_core::error::ProviderError;
//! use async_trait::async_trait;
//!
//! # async fn example(provider: &dyn AsrProvider) -> Result<(), ProviderError> {
//! let request = AsrRequest::builder("qwen3-asr-0.6b")
//!     .file("audio.wav")
//!     .enable_diarization(true)
//!     .build()?;
//!
//! let result = provider.transcribe(&request).await?;
//! println!("转写结果：{}", result.text);
//!
//! for seg in &result.segments {
//!     println!("[{:.1}s-{:.1}s] {}: {}", seg.start, seg.end, seg.speaker.as_deref().unwrap_or_default(), seg.text);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # 常见 ASR Provider
//!
//! - OpenAI 兼容语音转写接口（`/v1/audio/transcriptions`）
//! - 其他第三方服务

pub mod r#trait;

/// 重新导出 asr 相关 trait
pub use r#trait::*;
