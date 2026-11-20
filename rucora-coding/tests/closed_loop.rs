//! 闭环集成测试：任务 → 流式事件 → 审批 → 工具执行 → 文件变更 → 会话持久化。

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures_util::stream::{self, BoxStream};
use rucora::agent::ToolAgent;
use rucora::tools::FileEditTool;
use rucora_core::error::ProviderError;
use rucora_core::provider::LlmProvider;
use rucora_core::provider::types::{ChatRequest, ChatResponse, ChatStreamChunk, FinishReason};
use rucora_core::tool::types::{ToolCall, ToolDefinition};
use rucora_coding::events::{AgentEvent, EventBus};
use rucora_coding::permissions::{ApprovalDecision, ApprovalToolPolicy, PermissionConfig};
use rucora_coding::runner::CodingSession;
use rucora_coding::session::{FileSessionStore, SessionStatus, TranscriptEntry};

/// 脚本化 Mock Provider：按调用次序回放预设的流式分块。
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

fn text_chunk(text: &str) -> ChatStreamChunk {
    ChatStreamChunk {
        delta: Some(text.to_string()),
        tool_calls: vec![],
        usage: None,
        finish_reason: None,
    }
}

fn tool_call_chunk(call: ToolCall) -> ChatStreamChunk {
    ChatStreamChunk {
        delta: None,
        tool_calls: vec![call],
        usage: None,
        finish_reason: Some(FinishReason::ToolCall),
    }
}

#[async_trait]
impl LlmProvider for ScriptedProvider {
    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        Err(ProviderError::Message(
            "ScriptedProvider 仅支持流式路径".to_string(),
        ))
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
        Ok(Box::pin(stream::iter(
            script.into_iter().map(Ok),
        )))
    }
}

fn temp_workspace(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "rucora-coding-it-{tag}-{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&dir).expect("创建临时工作区");
    dir
}

/// 完整闭环：编辑文件需审批 → 放行 → 文件真实变更 → 会话持久化。
#[tokio::test]
async fn closed_loop_with_approval() {
    let workspace = temp_workspace("approval");
    let src = workspace.join("src.txt");
    std::fs::write(&src, "hello").expect("写入初始文件");

    let bus = EventBus::new();
    let policy = Arc::new(ApprovalToolPolicy::new(
        PermissionConfig {
            ask_timeout_secs: 10,
            ..PermissionConfig::default()
        },
        "it-session".to_string(),
        bus.clone(),
    ));

    let edit_call = ToolCall {
        id: "call-1".to_string(),
        name: "file_edit".to_string(),
        input: serde_json::json!({
            "path": src.display().to_string(),
            "old_string": "hello",
            "new_string": "hello world"
        }),
    };
    let provider = ScriptedProvider::new(vec![
        // 第一轮：说明意图并发起编辑
        vec![
            text_chunk("我来编辑文件。"),
            tool_call_chunk(edit_call),
        ],
        // 第二轮：确认完成
        vec![text_chunk("编辑完成，文件已更新。")],
    ]);

    let agent = ToolAgent::builder(provider)
        .model("scripted-mock")
        .system_prompt("测试用编程助手")
        .tool(FileEditTool::new().with_allowed_dirs(vec![workspace.clone()]))
        .with_policy(policy.clone())
        .build();

    let store = FileSessionStore::new(workspace.join("sessions")).expect("创建会话存储");
    let session = CodingSession::start(
        Arc::new(agent),
        &workspace,
        store,
        bus.clone(),
        policy.clone(),
    )
    .expect("会话启动");

    // 订阅事件（历史 + 实时），并作为"人工"在收到审批请求时放行
    let resolver = {
        let policy = policy.clone();
        let mut sub = session.events().subscribe();
        tokio::spawn(async move {
            let mut approved = false;
            loop {
                match sub.receiver.recv().await {
                    Ok((_, AgentEvent::ApprovalRequired { request_id, tool_name, .. })) => {
                        assert_eq!(tool_name, "file_edit");
                        assert!(policy.resolve(&request_id, ApprovalDecision::Allow));
                        approved = true;
                    }
                    Ok((_, AgentEvent::TaskCompleted { .. })) => {
                        return approved;
                    }
                    Ok(_) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => return approved,
                }
            }
        })
    };

    let outcome = session
        .run_task("把 src.txt 的 hello 改成 hello world")
        .await
        .expect("任务应成功");

    assert!(outcome.success, "任务应成功");
    assert_eq!(outcome.tool_calls, 1);
    assert!(outcome.text.contains("编辑完成"));

    // 1) 文件真实变更
    let content = std::fs::read_to_string(&src).expect("读取结果文件");
    assert_eq!(content, "hello world");

    // 2) 审批确实发生了
    let approved = resolver.await.expect("裁决任务不应 panic");
    assert!(approved, "应收到并处理审批请求");

    // 3) 事件流完整（含审批事件与完成事件）
    let events: Vec<AgentEvent> = session
        .event_history()
        .into_iter()
        .map(|(_, ev)| ev)
        .collect();
    assert!(
        events
            .iter()
            .any(|ev| matches!(ev, AgentEvent::ApprovalRequired { .. })),
        "应有 ApprovalRequired 事件"
    );
    assert!(
        events
            .iter()
            .any(|ev| matches!(ev, AgentEvent::ApprovalResolved { decision, request_id: _ } if decision == "allow")),
        "应有 ApprovalResolved(allow) 事件"
    );
    assert!(
        events
            .iter()
            .any(|ev| matches!(ev, AgentEvent::TaskCompleted { tool_calls: 1, .. })),
        "应有 TaskCompleted 事件"
    );

    // 4) 会话持久化：转录含任务、工具调用与最终文本
    let record = session.record_snapshot();
    assert_eq!(record.status, SessionStatus::Completed);
    assert_eq!(record.tasks.len(), 1);
    let tool_entries = record
        .transcript
        .iter()
        .filter_map(|entry| match entry {
            TranscriptEntry::ToolCall { name, success, .. } => Some((name.as_str(), *success)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(tool_entries, vec![("file_edit", true)]);
    assert!(record.transcript.iter().any(|entry| matches!(
        entry,
        TranscriptEntry::AssistantText { text } if text.contains("编辑完成")
    )));

    // 5) 存储可重新加载同一会话
    let store = FileSessionStore::new(workspace.join("sessions")).expect("重建存储");
    let reloaded = store.load(&record.id).expect("重新加载");
    assert_eq!(reloaded.id, record.id);

    std::fs::remove_dir_all(&workspace).ok();
}

/// 拒绝路径：审批被拒 → 工具以 policy_denied 结果返回 → 模型继续工作并收尾。
#[tokio::test]
async fn denied_approval_keeps_agent_working() {
    let workspace = temp_workspace("deny");
    let src = workspace.join("src.txt");
    std::fs::write(&src, "hello").expect("写入初始文件");

    let bus = EventBus::new();
    let policy = Arc::new(ApprovalToolPolicy::new(
        PermissionConfig {
            ask_timeout_secs: 10,
            ..PermissionConfig::default()
        },
        "deny-session".to_string(),
        bus.clone(),
    ));

    let edit_call = ToolCall {
        id: "call-1".to_string(),
        name: "file_edit".to_string(),
        input: serde_json::json!({
            "path": src.display().to_string(),
            "old_string": "hello",
            "new_string": "nope"
        }),
    };
    let provider = ScriptedProvider::new(vec![
        vec![tool_call_chunk(edit_call)],
        // 被拒后模型改口收尾（不再调用工具）
        vec![text_chunk("用户拒绝了编辑，我不再修改文件。")],
    ]);

    let agent = ToolAgent::builder(provider)
        .model("scripted-mock")
        .tool(FileEditTool::new().with_allowed_dirs(vec![workspace.clone()]))
        .with_policy(policy.clone())
        .build();

    let store = FileSessionStore::new(workspace.join("sessions")).expect("创建会话存储");
    let session = CodingSession::start(
        Arc::new(agent),
        &workspace,
        store,
        bus.clone(),
        policy.clone(),
    )
    .expect("会话启动");

    // 人工裁决：拒绝
    let resolver = {
        let policy = policy.clone();
        let mut sub = session.events().subscribe();
        tokio::spawn(async move {
            loop {
                match sub.receiver.recv().await {
                    Ok((_, AgentEvent::ApprovalRequired { request_id, .. })) => {
                        assert!(policy.resolve(&request_id, ApprovalDecision::Deny));
                    }
                    Ok((_, AgentEvent::TaskCompleted { .. })) => return,
                    Ok(_) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => return,
                }
            }
        })
    };

    let outcome = session.run_task("修改 src.txt").await.expect("任务应正常结束");

    // 任务本身成功结束（拒绝不是失败——模型收到了结构化拒绝并调整）
    assert!(outcome.success);
    assert!(outcome.text.contains("不再修改"));

    // 文件未被修改
    let content = std::fs::read_to_string(&src).expect("读取文件");
    assert_eq!(content, "hello");

    // 转录中的工具调用为"失败"（policy_denied → ok:false → success=false）
    let record = session.record_snapshot();
    assert!(
        record.transcript.iter().any(|entry| matches!(
            entry,
            TranscriptEntry::ToolCall { success: false, .. }
        )),
        "被拒的工具调用应记为失败"
    );

    resolver.await.expect("裁决任务不应 panic");
    std::fs::remove_dir_all(&workspace).ok();
}

/// 确认 ToolDefinition 可用（工具定义的编译期烟测）。
#[test]
fn tool_definition_shape() {
    let def = ToolDefinition {
        name: "demo".to_string(),
        description: Some("演示".to_string()),
        input_schema: serde_json::json!({"type": "object"}),
        version: 1,
    };
    assert_eq!(def.name, "demo");
}
