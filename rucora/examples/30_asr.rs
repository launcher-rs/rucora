//! rucora 语音转写（ASR）示例
//!
//! 展示如何使用 `OpenAiAsrProvider` 将音频/视频文件转写为文本，
//! 支持说话人分离、词级时间戳、自定义响应格式等参数。
//!
//! ## 环境准备
//!
//! 需要一个 OpenAI Audio API 兼容的本地语音识别服务，如
//! [qwen3-asr](https://github.com/Quantatirsk/qwen3-asr)：
//!
//! ```bash
//! # 启动 qwen3-asr 服务（默认端口 8000）
//! docker-compose up -d
//! ```
//!
//! ## 配置环境变量（均有默认值）
//!
//! ```bash
//! export ASR_BASE_URL=http://127.0.0.1:8000/v1
//! export ASR_API_KEY=your_api_key      # 可选，本地服务通常为空
//! export ASR_MODEL=qwen3-asr-0.6b
//! export ASR_FILE=path/to/audio.wav    # 也可传 http(s):// 音频 URL
//! ```
//!
//! ## 运行方法
//! ```bash
//! cargo run --example 30_asr
//! ```

use rucora::core::asr::{AsrRequest, AsrResponseFormat, AsrSegment};
use rucora::prelude::AsrProvider;
use rucora::provider::OpenAiAsrProvider;
use std::path::Path;
use tracing::{Level, info, warn};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // 读取配置（提供合理默认值）
    let base_url = std::env::var("ASR_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:8000/v1".into());
    let api_key = std::env::var("ASR_API_KEY").unwrap_or_default();
    let model = std::env::var("ASR_MODEL").unwrap_or_else(|_| "qwen3-asr-0.6b".into());
    let source = std::env::var("ASR_FILE").unwrap_or_else(|_| "audio.wav".into());

    info!("╔════════════════════════════════════════╗");
    info!("║   rucora ASR 语音转写示例              ║");
    info!("╚════════════════════════════════════════╝\n");

    // 1. 创建 Provider
    info!("1. 创建 OpenAiAsrProvider...");
    let provider = OpenAiAsrProvider::with_model(&base_url, &api_key, &model);
    info!("✓ Provider: {} （模型: {}）\n", base_url, model);

    // 2. 构建转写请求
    info!("2. 构建转写请求...");
    let mut builder = AsrRequest::builder(model).enable_diarization(true);

    // 音频来源：本地文件 或 URL
    if source.starts_with("http://") || source.starts_with("https://") {
        builder = builder.audio_address(&source);
        info!("   音频来源: URL {}", source);
    } else {
        if !Path::new(&source).exists() {
            warn!(
                "音频文件不存在：{}（可通过 ASR_FILE 环境变量指定）",
                source
            );
            warn!("示例提前退出。");
            return Ok(());
        }
        builder = builder.file(&source);
        info!("   音频来源: 文件 {}", source);
    }

    let request = builder
        .word_timestamps(true)
        .response_format(AsrResponseFormat::VerboseJson)
        .build()?;
    info!("✓ 请求构建完成（说话人分离 + 词级时间戳 + verbose_json）\n");

    // 3. 执行转写
    info!("3. 调用转写接口（长音频可能需要等待）...");
    let result = provider.transcribe(&request).await?;
    info!("✓ 转写完成：{} 字符，{} 个分段，时长 {:.1}s\n", result.text.len(), result.segments.len(), result.duration);

    // 4. 输出结果
    info!("4. 转写结果：\n");
    info!("完整文本：\n{}\n", result.text);

    if !result.segments.is_empty() {
        info!("分段明细（{} 段）：", result.segments.len());
        for (i, seg) in result.segments.iter().enumerate() {
            print_segment(i, seg);
        }
    }

    info!("示例完成！");

    Ok(())
}

/// 打印单个分段（含时间戳与说话人）。
fn print_segment(index: usize, seg: &AsrSegment) {
    let speaker = seg.speaker.as_deref().unwrap_or("未知");
    info!(
        "[{}] {:.1}s-{:.1}s ({}): {}",
        index + 1,
        seg.start,
        seg.end,
        speaker,
        seg.text
    );
}
