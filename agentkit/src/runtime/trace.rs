use std::path::Path;

use agentkit_core::channel::ChannelEvent;
use agentkit_core::error::AgentError;
use futures_util::stream::{self, BoxStream};
use serde_json::json;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// 轨迹（trace）持久化与回放。
///
/// 设计目标：
/// - 将 runtime 中产生的 `ChannelEvent` 以 JSONL（每行一个 JSON）写入文件。
/// - 便于调试/评估：一次运行可导出最小可复现的事件序列。
/// - 支持从 JSONL 文件读取并“回放”（replay）事件流。
///
/// 说明：
/// - 这里采用 JSONL 而非单个大 JSON 数组，便于流式写入与增量分析。
/// - `ChannelEvent` 在 core 层实现了 `Serialize/Deserialize`，因此可直接序列化。

/// 将一组事件写入 JSONL 文件。
pub async fn write_trace_jsonl(
    path: impl AsRef<Path>,
    events: &[ChannelEvent],
) -> Result<(), AgentError> {
    let mut f = fs::File::create(path.as_ref())
        .await
        .map_err(|e| AgentError::Message(format!("create trace file failed: {}", e)))?;

    for ev in events {
        let line = serde_json::to_string(ev)
            .map_err(|e| AgentError::Message(format!("serialize trace event failed: {}", e)))?;
        f.write_all(line.as_bytes())
            .await
            .map_err(|e| AgentError::Message(format!("write trace event failed: {}", e)))?;
        f.write_all(b"\n")
            .await
            .map_err(|e| AgentError::Message(format!("write trace newline failed: {}", e)))?;
    }

    f.flush()
        .await
        .map_err(|e| AgentError::Message(format!("flush trace file failed: {}", e)))?;
    Ok(())
}

/// 从 JSONL 文件读取事件序列。
pub async fn read_trace_jsonl(path: impl AsRef<Path>) -> Result<Vec<ChannelEvent>, AgentError> {
    let f = fs::File::open(path.as_ref())
        .await
        .map_err(|e| AgentError::Message(format!("open trace file failed: {}", e)))?;
    let mut reader = BufReader::new(f);

    let mut events = Vec::new();
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader
            .read_line(&mut line)
            .await
            .map_err(|e| AgentError::Message(format!("read trace line failed: {}", e)))?;
        if n == 0 {
            break;
        }
        let raw = line.trim_end_matches(['\n', '\r']);
        if raw.is_empty() {
            continue;
        }
        let ev: ChannelEvent = serde_json::from_str(raw).map_err(|e| {
            AgentError::Message(format!(
                "parse trace event failed: {} (line={})",
                e,
                json!({"line": raw})
            ))
        })?;
        events.push(ev);
    }

    Ok(events)
}

/// 将事件序列回放为 stream。
///
/// 说明：
/// - 该函数只是把事件按原顺序输出为 stream。
/// - 上层可以把这个 stream 接到任意 channel/subscriber，用于复现 UI 展示。
pub fn replay_events(
    events: Vec<ChannelEvent>,
) -> BoxStream<'static, Result<ChannelEvent, AgentError>> {
    Box::pin(stream::iter(events.into_iter().map(Ok)))
}
