use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rucora_core::channel::types::{ChannelEvent, TokenDeltaEvent};
use rucora_core::provider::types::{ChatMessage, Role};
use rucora_core::tool::types::{ToolCall, ToolResult};
use serde_json::json;

fn bench_channel_event_serialize(c: &mut Criterion) {
    let ev = ChannelEvent::Message(ChatMessage {
        role: Role::Assistant,
        content: "hello world".to_string(),
        name: None,
    });

    c.bench_function("channel_event/message serialize", |b| {
        b.iter(|| {
            let s = serde_json::to_string(black_box(&ev)).unwrap();
            black_box(s)
        })
    });

    let ev2 = ChannelEvent::TokenDelta(TokenDeltaEvent {
        delta: "delta".to_string(),
    });

    c.bench_function("channel_event/token_delta serialize", |b| {
        b.iter(|| {
            let s = serde_json::to_string(black_box(&ev2)).unwrap();
            black_box(s)
        })
    });
}

fn bench_tool_result_serialize(c: &mut Criterion) {
    let call = ToolCall {
        id: "call-1".to_string(),
        name: "echo".to_string(),
        input: json!({"text": "hello"}),
    };
    let result = ToolResult {
        tool_call_id: call.id,
        output: json!({"success": true, "output": "hello"}),
    };

    c.bench_function("tool_result serialize", |b| {
        b.iter(|| {
            let s = serde_json::to_string(black_box(&result)).unwrap();
            black_box(s)
        })
    });
}

criterion_group!(
    benches,
    bench_channel_event_serialize,
    bench_tool_result_serialize
);
criterion_main!(benches);
