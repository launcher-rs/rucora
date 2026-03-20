use std::path::PathBuf;

use agentkit_core::channel::types::{ChannelEvent, TokenDeltaEvent};
use agentkit_core::provider::types::ChatMessage;
use agentkit_core::tool::types::{ToolCall, ToolResult};
use futures_util::StreamExt;
use serde_json::json;

use agentkit_runtime::trace::{read_trace_jsonl, replay_events, write_trace_jsonl};

fn unique_temp_file(prefix: &str) -> PathBuf {
    let mut base = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    base.push(format!("agentkit-{}-{}.jsonl", prefix, nanos));
    base
}

#[tokio::test]
async fn trace_jsonl_should_roundtrip_and_replay_in_order() {
    let path = unique_temp_file("trace");

    let events = vec![
        ChannelEvent::Message(ChatMessage::user("hi")),
        ChannelEvent::TokenDelta(TokenDeltaEvent {
            delta: "hello".to_string(),
        }),
        ChannelEvent::ToolCall(ToolCall {
            id: "call-1".to_string(),
            name: "echo".to_string(),
            input: json!({"x": 1}),
        }),
        ChannelEvent::ToolResult(ToolResult {
            tool_call_id: "call-1".to_string(),
            output: json!({"ok": true}),
        }),
    ];

    write_trace_jsonl(&path, &events).await.unwrap();
    let loaded = read_trace_jsonl(&path).await.unwrap();
    assert_eq!(loaded, events);

    let replayed: Vec<ChannelEvent> = replay_events(loaded).map(|r| r.unwrap()).collect().await;
    assert_eq!(replayed, events);

    // 尽量清理临时文件，但不让清理失败影响测试结果。
    let _ = tokio::fs::remove_file(path).await;
}
