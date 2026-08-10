//! 自动语音识别（ASR）功能抽象。
//!
//! 该模块定义音频转文字的 trait，类似于 `LlmProvider`，
//! 用于将音频/视频文件转换为带时间戳与说话人的文本。

use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::ProviderError;

/// 单段识别结果。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AsrSegment {
    /// 识别出的文本
    pub text: String,
    /// 起始时间（秒）
    pub start: f64,
    /// 结束时间（秒）
    pub end: f64,
    /// 说话人标识（启用说话人分离时存在）
    pub speaker: Option<String>,
}

/// 完整的转写结果。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AsrResult {
    /// 完整拼接的转写文本
    pub text: String,
    /// 音频总时长（秒）
    pub duration: f64,
    /// 分段识别结果
    pub segments: Vec<AsrSegment>,
}

/// 转写响应格式。
///
/// 与 OpenAI Audio API 的 `response_format` 参数对应。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AsrResponseFormat {
    /// 纯文本
    Text,
    /// JSON（仅含 `text` 字段）
    Json,
    /// SRT 字幕
    Srt,
    /// VTT 字幕
    Vtt,
    /// 详细 JSON（含分段、时间戳与说话人），默认格式
    #[default]
    VerboseJson,
}

impl AsrResponseFormat {
    /// 返回 OpenAI Audio API 的 `response_format` 参数值。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Json => "json",
            Self::Srt => "srt",
            Self::Vtt => "vtt",
            Self::VerboseJson => "verbose_json",
        }
    }
}

/// 转写请求。
///
/// # 兼容性说明
///
/// 可选字段默认为 `None` 时**不发送**对应参数，交由服务端默认行为决定
/// （自动选择），从而保持对 OpenAI Audio API 的兼容性：
///
/// - `enable_diarization = None`：不发送 `enable_speaker_diarization`，
///   qwen3-asr 默认开启说话人分离，OpenAI Whisper 则忽略该能力
/// - `word_timestamps = None`：不发送 `word_timestamps`
///
/// 需要显式控制时通过 [`AsrRequestBuilder`] 或直接设置字段即可（自定义）。
#[derive(Debug, Clone, Default)]
pub struct AsrRequest {
    /// 本地音频/视频文件路径（与 `audio_address` 二选一，优先文件）
    pub file: Option<PathBuf>,
    /// 模型名称（如 qwen3-asr-0.6b、whisper-1；空串回退到 Provider 默认模型）
    pub model: String,
    /// 语言代码（如 zh/en/ja），None 表示自动检测
    pub language: Option<String>,
    /// 是否启用说话人分离（None 时不发送参数，由服务端默认决定）
    pub enable_diarization: Option<bool>,
    /// 是否返回词级时间戳（None 时不发送参数，由服务端默认决定）
    pub word_timestamps: Option<bool>,
    /// 转写响应格式
    pub response_format: AsrResponseFormat,
    /// 音频/视频 URL（qwen3-asr 支持服务端下载；`file` 提供时被忽略）
    pub audio_address: Option<String>,
    /// 提示文本（保留参数，部分服务端忽略）
    pub prompt: Option<String>,
    /// 采样温度（保留参数，部分服务端忽略）
    pub temperature: Option<f32>,
}

impl AsrRequest {
    /// 创建请求构建器（`model` 为必填项）。
    pub fn builder(model: impl Into<String>) -> AsrRequestBuilder {
        AsrRequestBuilder::new(model)
    }
}

/// [`AsrRequest`] 构建器，用于快速构造带可选参数的转写请求。
#[derive(Debug, Clone, Default)]
pub struct AsrRequestBuilder {
    file: Option<PathBuf>,
    model: String,
    language: Option<String>,
    enable_diarization: Option<bool>,
    word_timestamps: Option<bool>,
    response_format: AsrResponseFormat,
    audio_address: Option<String>,
    prompt: Option<String>,
    temperature: Option<f32>,
}

impl AsrRequestBuilder {
    /// 创建新的构建器（`model` 为必填项）。
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            ..Self::default()
        }
    }

    /// 设置本地音频/视频文件路径。
    pub fn file(mut self, path: impl Into<PathBuf>) -> Self {
        self.file = Some(path.into());
        self
    }

    /// 设置音频/视频 URL（由服务端下载）。
    pub fn audio_address(mut self, url: impl Into<String>) -> Self {
        self.audio_address = Some(url.into());
        self
    }

    /// 设置语言代码（如 zh/en/ja）。
    pub fn language(mut self, lang: impl Into<String>) -> Self {
        self.language = Some(lang.into());
        self
    }

    /// 显式启用/禁用说话人分离（默认不发送参数，由服务端决定）。
    pub fn enable_diarization(mut self, enabled: bool) -> Self {
        self.enable_diarization = Some(enabled);
        self
    }

    /// 显式开启/关闭词级时间戳（默认不发送参数，由服务端决定）。
    pub fn word_timestamps(mut self, enabled: bool) -> Self {
        self.word_timestamps = Some(enabled);
        self
    }

    /// 设置转写响应格式（默认 `VerboseJson`）。
    pub fn response_format(mut self, format: AsrResponseFormat) -> Self {
        self.response_format = format;
        self
    }

    /// 设置提示文本。
    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    /// 设置采样温度。
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// 构建请求。
    ///
    /// # Errors
    ///
    /// 当 `file` 与 `audio_address` 均未提供时返回错误。
    pub fn build(self) -> Result<AsrRequest, ProviderError> {
        if self.file.is_none() && self.audio_address.is_none() {
            return Err(ProviderError::Message(
                "AsrRequest 必须提供 file 或 audio_address".to_string(),
            ));
        }
        Ok(AsrRequest {
            file: self.file,
            model: self.model,
            language: self.language,
            enable_diarization: self.enable_diarization,
            word_timestamps: self.word_timestamps,
            response_format: self.response_format,
            audio_address: self.audio_address,
            prompt: self.prompt,
            temperature: self.temperature,
        })
    }
}

/// 语音转写提供者抽象。
///
/// 该 trait 的目标：统一音频转文字接口，
/// 支持不同语音模型（qwen3-asr、Whisper 等）。
#[async_trait]
pub trait AsrProvider: Send + Sync {
    /// 将音频/视频文件转写为文本。
    ///
    /// # 参数
    /// * `request` - 转写请求（文件路径、模型、说话人分离开关等）
    ///
    /// # 返回
    /// * `Result<AsrResult, ProviderError>` - 转写结果（含分段）或错误
    async fn transcribe(&self, request: &AsrRequest) -> Result<AsrResult, ProviderError>;
}
