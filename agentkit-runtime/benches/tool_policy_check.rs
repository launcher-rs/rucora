use agentkit_core::tool::types::ToolCall;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use serde_json::json;

use agentkit_runtime::{DefaultToolPolicy, ToolCallContext, ToolPolicy};

fn bench_default_tool_policy_check(c: &mut Criterion) {
    let policy = DefaultToolPolicy::new();

    // 用一个专用 runtime 来跑 async bench。
    let rt = tokio::runtime::Runtime::new().unwrap();

    let call = ToolCall {
        id: "call-1".to_string(),
        name: "cmd_exec".to_string(),
        input: json!({"command": "curl", "args": ["https://example.com"]}),
    };

    c.bench_function("default_tool_policy/check cmd_exec", |b| {
        b.iter(|| {
            rt.block_on(async {
                let ctx = ToolCallContext {
                    tool_call: black_box(call.clone()),
                };
                let r = policy.check(&ctx).await;
                black_box(r)
            })
        })
    });
}

criterion_group!(benches, bench_default_tool_policy_check);
criterion_main!(benches);
