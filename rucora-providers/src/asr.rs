//! OpenAI 兼容 ASR Provider 实现。
//!
//! 调用 OpenAI Audio API 兼容的 `/v1/audio/transcriptions` 端点，
//! 将音频/视频文件转写为带时间戳与说话人的文本。
//!
//! 适用于 [qwen3-asr](https://github.com/Quantatirsk/qwen3-asr) 等
//! 提供 OpenAI Audio API 兼容接口的本地语音识别服务。

use std::{env, path::Path};

use crate::{
    helpers::{map_http_error, map_reqwest_error},
    http_config::{
        build_client, build_client_with_timeout, DEFAULT_CONNECT_TIMEOUT_SECS,
        DEFAULT_REQUEST_TIMEOUT_SECS,
    },
};
use async_trait::async_trait;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use rucora_core::{
    asr::{AsrProvider, AsrRequest, AsrResponseFormat, AsrResult, AsrSegment},
    error::ProviderError,
};
use serde::Deserialize;
use tokio_util::io::ReaderStream;
use tracing::debug;

/// 默认模型（当未指定时使用）
const OPENAI_ASR_DEFAULT_MODEL: &str = "qwen3-asr-0.6b";

/// OpenAI 兼容语音转写 Provider。
///
/// 通过 multipart/form-data 上传音频文件到 `/v1/audio/transcriptions`，
/// 请求 `verbose_json` 格式响应，解析分段（含时间戳与说话人）。
///
/// # 使用示例
///
/// ```rust,no_run
/// use rucora_providers::asr::OpenAiAsrProvider;
/// use rucora_core::asr::{AsrProvider, AsrRequest};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let provider = OpenAiAsrProvider::new("http://127.0.0.1:8000/v1", "");
/// let result = provider
///     .transcribe(
///         &AsrRequest::builder("qwen3-asr-0.6b")
///             .file("audio.wav")
///             .enable_diarization(true)
///             .build()?,
///     )
///     .await?;
/// println!("{}", result.text);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct OpenAiAsrProvider {
    client: reqwest::Client,
    headers: HeaderMap,
    base_url: String,
    default_model: String,
    request_timeout_secs: Option<u64>,
    connect_timeout_secs: Option<u64>,
}

impl OpenAiAsrProvider {
    /// 从环境变量创建 Provider。
    ///
    /// - `OPENAI_BASE_URL`: API 基础 URL（默认 `http://127.0.0.1:8000/v1`）
    /// - `OPENAI_API_KEY`: API Key（本地服务通常为空）
    /// - `OPENAI_ASR_DEFAULT_MODEL`: 默认模型（默认 `qwen3-asr-0.6b`）
    pub fn from_env() -> Result<Self, ProviderError> {
        let base_url = env::var("OPENAI_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8000/v1".to_string());
        let api_key = env::var("OPENAI_API_KEY").unwrap_or_default();
        let default_model = env::var("OPENAI_ASR_DEFAULT_MODEL")
            .unwrap_or_else(|_| OPENAI_ASR_DEFAULT_MODEL.to_string());
        Ok(Self::with_model(base_url, api_key, default_model))
    }

    /// 创建 Provider（使用内置默认模型 `qwen3-asr-0.6b`）。
    ///
    /// # 参数
    ///
    /// - `base_url`: API 基础 URL，如 `http://127.0.0.1:8000/v1`
    /// - `api_key`: API Key（本地服务通常为空字符串）
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self::with_model(base_url, api_key, OPENAI_ASR_DEFAULT_MODEL)
    }

    /// 创建 Provider（指定默认模型）。
    pub fn with_model(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        default_model: impl Into<String>,
    ) -> Self {
        let api_key = api_key.into();
        let mut headers = HeaderMap::new();
        if !api_key.is_empty()
            && let Ok(v) = HeaderValue::from_str(&format!("Bearer {api_key}"))
        {
            headers.insert(AUTHORIZATION, v);
        }
        let client = build_client(headers.clone());

        Self {
            client,
            headers,
            base_url: base_url.into(),
            default_model: default_model.into(),
            request_timeout_secs: None,
            connect_timeout_secs: None,
        }
    }

    fn build_http_client(
        headers: &HeaderMap,
        request_timeout_secs: Option<u64>,
        connect_timeout_secs: Option<u64>,
    ) -> reqwest::Client {
        match (request_timeout_secs, connect_timeout_secs) {
            (Some(rt), Some(ct)) => {
                build_client_with_timeout(headers.clone(), rt, ct)
            }
            (Some(rt), None) => {
                build_client_with_timeout(headers.clone(), rt, DEFAULT_CONNECT_TIMEOUT_SECS)
            }
            (None, Some(ct)) => {
                build_client_with_timeout(headers.clone(), DEFAULT_REQUEST_TIMEOUT_SECS, ct)
            }
            (None, None) => build_client(headers.clone()),
        }
    }

    /// 设置默认模型（覆盖内置默认值）。
    pub fn with_default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }

    /// 设置请求超时时间（秒）。
    pub fn with_request_timeout(mut self, secs: Option<u64>) -> Self {
        self.request_timeout_secs = secs;
        self.client =
            Self::build_http_client(&self.headers, self.request_timeout_secs, self.connect_timeout_secs);
        self
    }

    /// 设置连接超时时间（秒）。
    pub fn with_connect_timeout(mut self, secs: Option<u64>) -> Self {
        self.connect_timeout_secs = secs;
        self.client =
            Self::build_http_client(&self.headers, self.request_timeout_secs, self.connect_timeout_secs);
        self
    }

    /// 设置自定义 HTTP 客户端。
    pub fn with_client(mut self, client: reqwest::Client) -> Self {
        self.client = client;
        self
    }
}

fn guess_mime(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());
    match ext.as_deref() {
        Some("wav") => "audio/wav",
        Some("opus") | Some("ogg") => "audio/ogg",
        Some("flac") => "audio/flac",
        Some("mp3") => "audio/mpeg",
        Some("m4a") | Some("mp4") => "audio/mp4",
        Some("aac") => "audio/aac",
        Some("wma") => "audio/x-ms-wma",
        Some("amr") => "audio/amr",
        Some("webm") => "audio/webm",
        _ => "application/octet-stream",
    }
}

/// 解析实际使用的模型：请求中指定时优先，空串时回退到 Provider 默认模型。
fn resolve_model(request_model: &str, default_model: &str) -> String {
    if request_model.trim().is_empty() {
        default_model.to_string()
    } else {
        request_model.to_string()
    }
}

/// OpenAI `verbose_json` 转写响应的类型化结构。
#[derive(Debug, Deserialize)]
struct TranscriptionResponse {
    #[serde(default)]
    text: String,
    #[serde(default)]
    duration: f64,
    #[serde(default)]
    segments: Vec<TranscriptionSegment>,
}

/// 单条转写分段的类型化结构。
#[derive(Debug, Deserialize)]
struct TranscriptionSegment {
    #[serde(default)]
    text: String,
    #[serde(default)]
    start: f64,
    #[serde(default)]
    end: f64,
    #[serde(default)]
    speaker: Option<String>,
}

#[async_trait]
impl AsrProvider for OpenAiAsrProvider {
    async fn transcribe(&self, request: &AsrRequest) -> Result<AsrResult, ProviderError> {
        // 优先级：1) 请求中指定的 model 2) Provider 默认模型（model 为空串时）
        let model = resolve_model(&request.model, &self.default_model);

        let url = format!("{}/audio/transcriptions", self.base_url.trim_end_matches('/'));

        let mut form = reqwest::multipart::Form::new()
            .text("model", model.clone())
            .text("response_format", request.response_format.as_str().to_string());

        // 音频来源：本地文件优先，其次 audio_address URL
        if let Some(file_path) = &request.file {
            // 以流式方式上传文件，避免将大音频文件整体读入内存
            let file = tokio::fs::File::open(file_path).await.map_err(|e| {
                ProviderError::Message(format!(
                    "打开音频文件失败 {}: {e}",
                    file_path.display()
                ))
            })?;
            let file_name = file_path
                .file_name()
                .map_or_else(|| "audio".into(), |s| s.to_string_lossy().into_owned());
            let mime = guess_mime(file_path);

            let body = reqwest::Body::wrap_stream(ReaderStream::with_capacity(file, 64 * 1024));
            let part = reqwest::multipart::Part::stream(body)
                .file_name(file_name)
                .mime_str(mime)
                .map_err(|e| ProviderError::Message(format!("构造 multipart 失败: {e}")))?;
            form = form.part("file", part);
        } else if let Some(audio_address) = &request.audio_address {
            form = form.text("audio_address", audio_address.clone());
        } else {
            return Err(ProviderError::Message(
                "ASR 请求必须提供 file 或 audio_address".to_string(),
            ));
        }

        // 可选参数：None 时不发送，交由服务端默认（保持 OpenAI 兼容）
        if let Some(lang) = &request.language
            && !lang.trim().is_empty()
        {
            form = form.text("language", lang.clone());
        }
        if let Some(diarization) = request.enable_diarization {
            form = form.text("enable_speaker_diarization", diarization.to_string());
        }
        if let Some(word_ts) = request.word_timestamps {
            form = form.text("word_timestamps", word_ts.to_string());
        }
        if let Some(prompt) = &request.prompt
            && !prompt.trim().is_empty()
        {
            form = form.text("prompt", prompt.clone());
        }
        if let Some(temperature) = request.temperature {
            form = form.text("temperature", temperature.to_string());
        }

        debug!(
            provider = "openai_asr",
            url = %url,
            model = %model,
            file = ?request.file,
            diarization = request.enable_diarization,
            "asr.transcribe.start"
        );

        let start = std::time::Instant::now();
        let resp = self
            .client
            .post(url)
            .multipart(form)
            .send()
            .await
            .map_err(|e| map_reqwest_error(e, start.elapsed()))?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| ProviderError::Message(format!("读取响应失败：{e}")))?;

        let elapsed_ms = start.elapsed().as_millis() as u64;
        debug!(
            provider = "openai_asr",
            status = %status,
            elapsed_ms,
            "asr.transcribe.http.done"
        );

        if !status.is_success() {
            let error_msg = format!("ASR 请求失败：status={status} body={body}");
            return Err(map_http_error(status, error_msg));
        }

        // 按响应格式解析
        let result = match request.response_format {
            // text/srt/vtt 返回纯文本或字幕内容，直接作为 text
            AsrResponseFormat::Text | AsrResponseFormat::Srt | AsrResponseFormat::Vtt => {
                AsrResult {
                    text: body.trim().to_string(),
                    duration: 0.0,
                    segments: Vec::new(),
                }
            }
            // json/verbose_json 返回 JSON（verbose_json 额外含分段与说话人）
            AsrResponseFormat::Json | AsrResponseFormat::VerboseJson => {
                let data: TranscriptionResponse = serde_json::from_str(&body).map_err(|e| {
                    ProviderError::Message(format!(
                        "解析 ASR 响应 JSON 失败：{e}。响应内容：{}",
                        body.chars().take(500).collect::<String>()
                    ))
                })?;
                let segments = data
                    .segments
                    .into_iter()
                    .map(|s| AsrSegment {
                        text: s.text,
                        start: s.start,
                        end: s.end,
                        speaker: s.speaker,
                    })
                    .collect();
                AsrResult {
                    text: data.text,
                    duration: data.duration,
                    segments,
                }
            }
        };

        debug!(
            provider = "openai_asr",
            text_len = result.text.len(),
            segments_len = result.segments.len(),
            "asr.transcribe.parsed"
        );

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation() {
        let provider = OpenAiAsrProvider::new("http://127.0.0.1:8000/v1", "");
        assert_eq!(provider.default_model, "qwen3-asr-0.6b");
        assert!(!provider.headers.contains_key(AUTHORIZATION));
    }

    #[test]
    fn test_provider_with_model() {
        let provider = OpenAiAsrProvider::with_model(
            "http://127.0.0.1:8000/v1",
            "sk-test",
            "whisper-1",
        );
        assert_eq!(provider.default_model, "whisper-1");
        assert!(provider.headers.contains_key(AUTHORIZATION));
    }

    #[test]
    fn test_provider_with_default_model_chain() {
        let provider = OpenAiAsrProvider::new("http://127.0.0.1:8000/v1", "")
            .with_default_model("whisper-large-v3");
        assert_eq!(provider.default_model, "whisper-large-v3");
    }

    #[test]
    fn test_parse_transcription_response() {
        let body = r#"{
            "text": "你好世界",
            "duration": 2.5,
            "segments": [
                {"text": "你好", "start": 0.0, "end": 1.0, "speaker": "SPEAKER_00"},
                {"text": "世界", "start": 1.0, "end": 2.5}
            ]
        }"#;
        let data: TranscriptionResponse = serde_json::from_str(body).unwrap();
        assert_eq!(data.text, "你好世界");
        assert_eq!(data.duration, 2.5);
        assert_eq!(data.segments.len(), 2);
        assert_eq!(data.segments[0].speaker.as_deref(), Some("SPEAKER_00"));
        assert_eq!(data.segments[1].speaker, None);
        assert_eq!(data.segments[1].start, 1.0);
    }

    #[test]
    fn test_parse_empty_response() {
        let data: TranscriptionResponse = serde_json::from_str("{}").unwrap();
        assert_eq!(data.text, "");
        assert_eq!(data.duration, 0.0);
        assert!(data.segments.is_empty());
    }

    #[test]
    fn test_resolve_model() {
        let default = "qwen3-asr-0.6b";
        assert_eq!(resolve_model("whisper-1", default), "whisper-1");
        assert_eq!(resolve_model("", default), default);
        assert_eq!(resolve_model("  ", default), default);
        assert_eq!(resolve_model("qwen3-asr-0.6b", default), "qwen3-asr-0.6b");
    }

    #[test]
    fn test_builder_defaults() {
        let req = AsrRequest::builder("qwen3-asr-0.6b")
            .file("audio.wav")
            .build()
            .unwrap();
        assert_eq!(req.model, "qwen3-asr-0.6b");
        assert_eq!(req.file.as_deref(), Some(std::path::Path::new("audio.wav")));
        assert_eq!(req.language, None);
        assert_eq!(req.enable_diarization, None);
        assert_eq!(req.word_timestamps, None);
        assert_eq!(req.audio_address, None);
        assert_eq!(req.response_format, AsrResponseFormat::VerboseJson);
    }

    #[test]
    fn test_builder_requires_source() {
        assert!(AsrRequest::builder("qwen3-asr-0.6b").build().is_err());
        let req = AsrRequest::builder("qwen3-asr-0.6b")
            .audio_address("https://example.com/a.mp3")
            .build()
            .unwrap();
        assert_eq!(
            req.audio_address.as_deref(),
            Some("https://example.com/a.mp3")
        );
        assert!(req.file.is_none());
    }

    #[test]
    fn test_response_format_str() {
        assert_eq!(AsrResponseFormat::Text.as_str(), "text");
        assert_eq!(AsrResponseFormat::Json.as_str(), "json");
        assert_eq!(AsrResponseFormat::Srt.as_str(), "srt");
        assert_eq!(AsrResponseFormat::Vtt.as_str(), "vtt");
        assert_eq!(AsrResponseFormat::VerboseJson.as_str(), "verbose_json");
    }

    #[test]
    fn test_guess_mime() {
        assert_eq!(guess_mime(Path::new("a.wav")), "audio/wav");
        assert_eq!(guess_mime(Path::new("a.OPUS")), "audio/ogg");
        assert_eq!(guess_mime(Path::new("a.mp3")), "audio/mpeg");
        assert_eq!(guess_mime(Path::new("a.m4a")), "audio/mp4");
        assert_eq!(guess_mime(Path::new("a.xyz")), "application/octet-stream");
    }
}
