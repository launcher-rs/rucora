//! Coding Agent 闭环演示（无需 API Key）。
//!
//! 展示 rucora-coding 的核心闭环：
//!
//! ```text
//! 任务 → 流式事件 → 工具调用 → 审批（人工裁决）→ 文件变更 → 会话持久化
//! ```
//!
//! 使用脚本化 Mock Provider 模拟模型行为（先读文件、再发起编辑），
//! "人工"由事件消费端自动放行。真实场景下把 Mock 换成任意 Provider、
//! 把自动放行换成 GUI 审批即可。
//!
//! 运行：
//!
//! ```bash
//! cargo run -p rucora-coding --example closed_loop
//! ```

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures_util::stream::{self, BoxStream};
use rucora::agent::ToolAgent;
use rucora::tools::{FileEditTool, FileReadTool};
use rucora_core::error::ProviderError;
use rucora_core::provider::LlmProvider;
use rucora_core::provider::types::{ChatRequest, ChatResponse, ChatStreamChunk, FinishReason};
use rucora_core::tool::types::ToolCall;
use rucora_coding::events::{AgentEvent, EventBus};
use rucora_coding::permissions::{ApprovalDecision, ApprovalToolPolicy, PermissionConfig};
use rucora_coding::runner::CodingSession;
use rucora_coding::session::FileSessionStore;

/// 脚本化 Mock Provider。
struct ScriptedProvider {
    scripts: Mutex<VecDeque<Vec<ChatStreamChunk>>>,
}

impl ScriptedProvider {
    fn new(scripts: Vec<Vec<ChatStreamChunk>>) -> Self {
        Self {
            scripts: Mutex::new(VecDeque::from(scripts)),
        }
    }
}

#[async_trait]
impl LlmProvider for ScriptedProvider {
    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        Err(ProviderError::Message("演示仅走流式路径".to_string()))
    }

    fn stream_chat(
        &self,
        _request: ChatRequest,
    ) -> Result<BoxStream<'static, Result<ChatStreamChunk, ProviderError>>, ProviderError> {
        let script = self
            .scripts
            .lock()
            .expect("脚本队列锁中毒")
            .pop_front()
            .ok_or_else(|| ProviderError::Message("脚本已耗尽".to_string()))?;
        Ok(Box::pin(stream::iter(script.into_iter().map(Ok))))
    }
}

fn text(text: &str) -> ChatStreamChunk {
    ChatStreamChunk {
        delta: Some(text.to_string()),
        tool_calls: vec![],
        usage: None,
        finish_reason: None,
    }
}

fn tool_call(call: ToolCall) -> ChatStreamChunk {
    ChatStreamChunk {
        delta: None,
        tool_calls: vec![call],
        usage: None,
        finish_reason: Some(FinishReason::ToolCall),
    }
}

fn event_label(event: &AgentEvent) -> String {
    match event {
        AgentEvent::SessionStarted { session_id } => format!("会话开始 {session_id}"),
        AgentEvent::SessionResumed { session_id } => format!("会话恢复 {session_id}"),
        AgentEvent::TaskStarted { task } => format!("任务：{task}"),
        AgentEvent::MessageDelta { delta } => format!("增量 +{delta}"),
        AgentEvent::AssistantMessage { text, has_tool_calls } => format!(
            "助手段（{}）",
            if *has_tool_calls { "伴随工具调用" } else { "纯文本" }
        )
        .replace("助手段", text),
        AgentEvent::ToolStarted { name, input, .. } => format!("工具开始 {name} {input}"),
        AgentEvent::ToolCompleted { name, success, output, .. } => format!(
            "工具完成 {name}（{}）",
            if *success {
                "成功".to_string()
            } else {
                format!("失败 {output}")
            }
        ),
        AgentEvent::ApprovalRequired { tool_name, reason, .. } => {
            format!("⏸ 待审批 {tool_name}：{reason}")
        }
        AgentEvent::ApprovalResolved { decision, .. } => format!("✓ 审批裁决 {decision}"),
        AgentEvent::TaskCompleted { tool_calls, duration_ms, .. } => {
            format!("✅ 任务完成（工具 {tool_calls} 次，{duration_ms}ms）")
        }
        AgentEvent::Failed { message } => format!("❌ 失败 {message}"),
        AgentEvent::Cancelled => "⛔ 已取消".to_string(),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 准备临时工作区
    let workspace = std::env::temp_dir().join(format!(
        "rucora-coding-demo-{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&workspace)?;
    let src = workspace.join("demo.txt");
    std::fs::write(&src, "hello")?;
    println!("工作区：{}", workspace.display());
    println!("初始内容：{}\n", std::fs::read_to_string(&src)?);

    // 2. 事件总线与审批策略
    let bus = EventBus::new();
    let policy = Arc::new(ApprovalToolPolicy::new(
        PermissionConfig {
            ask_timeout_secs: 30,
            ..PermissionConfig::default()
        },
        "demo-session".to_string(),
        bus.clone(),
    ));

    // 3. 模拟模型：读文件 → 发起编辑 → 总结
    let edit_call = ToolCall {
        id: "call-1".to_string(),
        name: "file_edit".to_string(),
        input: serde_json::json!({
            "path": src.display().to_string(),
            "old_string": "hello",
            "new_string": "hello, rucora!"
        }),
    };
    let provider = ScriptedProvider::new(vec![
        vec![
            text("先确认当前内容。"),
            tool_call(ToolCall {
                id: "call-0".to_string(),
                name: "file_read".to_string(),
                input: serde_json::json!({ "path": src.display().to_string() }),
            }),
        ],
        vec![
            text("内容确认，现在发起编辑。"),
            tool_call(edit_call),
        ],
        vec![text("编辑已完成，demo.txt 已更新为「hello, rucora!」。")],
    ]);

    // 4. 组装 Agent（审批策略接入执行引擎）
    let agent = ToolAgent::builder(provider)
        .model("scripted-mock")
        .system_prompt("演示用编程助手")
        .tool(FileReadTool::new().with_allowed_dirs(vec![workspace.clone()]))
        .tool(FileEditTool::new().with_allowed_dirs(vec![workspace.clone()]))
        .with_policy(policy.clone())
        .build();

    // 5. 启动会话
    let store = FileSessionStore::new(workspace.join("sessions"))?;
    let session = CodingSession::start(
        Arc::new(agent),
        &workspace,
        store,
        bus.clone(),
        policy.clone(),
        None,
        None,
    )?;

    // 6. "人工"审批：消费事件流，收到 ApprovalRequired 自动放行
    let approver = {
        let policy = policy.clone();
        let mut sub = session.events().subscribe();
        tokio::spawn(async move {
            loop {
                match sub.receiver.recv().await {
                    Ok((_, AgentEvent::ApprovalRequired { request_id, tool_name, .. })) => {
                        println!("[审批] 收到请求 {tool_name}，1 秒后放行（模拟人工确认）...");
                        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                        policy.resolve(&request_id, ApprovalDecision::Allow);
                    }
                    Ok((_, AgentEvent::TaskCompleted { .. })) => return,
                    Ok(_) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => return,
                }
            }
        })
    };

    // 7. 运行任务（主线程同时打印事件流，模拟 GUI 渲染）
    let mut render = session.events().subscribe();
    let renderer = tokio::spawn(async move {
        let mut last_seq = 0u64;
        loop {
            match render.receiver.recv().await {
                Ok((seq, event)) => {
                    if seq <= last_seq {
                        continue; // 历史回放与实时事件的去重
                    }
                    last_seq = seq;
                    println!("[事件 {seq:>2}] {}", event_label(&event));
                    if matches!(event, AgentEvent::TaskCompleted { .. } | AgentEvent::Failed { .. }) {
                        return; // 任务终态：渲染循环退出
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => return,
            }
        }
    });

    let outcome = session.run_task("把 demo.txt 的问候语改成「hello, rucora!」").await?;

    approver.await?;
    renderer.await?;
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    println!("\n──────────────────────────────");
    println!("最终回复：{}", outcome.text);
    println!("文件内容：{}", std::fs::read_to_string(&src)?);
    println!(
        "会话记录：{} 条转录，状态 {:?}",
        session.record_snapshot().transcript.len(),
        session.record_snapshot().status
    );
    println!("存储目录：{}", workspace.join("sessions").display());

    std::fs::remove_dir_all(&workspace).ok();
    Ok(())
}
