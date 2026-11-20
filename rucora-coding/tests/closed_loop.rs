//! 闭环集成测试：任务 → 流式事件 → 审批 → 工具执行 → 文件变更 → 会话持久化。

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures_util::stream::{self, BoxStream, StreamExt};
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
    scripts: Mutex<VecDeque<Script>>,
    /// 捕获每次 stream_chat 收到的请求（供断言上下文注入）。
    captured: Arc<Mutex<Vec<ChatRequest>>>,
}

/// 单次流式调用的脚本。
enum Script {
    /// 按序回放的固定分块。
    Chunks(Vec<ChatStreamChunk>),
    /// 先发一个增量然后永久挂起（用于取消测试）。
    HangAfterFirstDelta,
}

impl ScriptedProvider {
    fn new(scripts: Vec<Vec<ChatStreamChunk>>) -> Self {
        Self {
            scripts: Mutex::new(VecDeque::from(
                scripts.into_iter().map(Script::Chunks).collect::<Vec<_>>(),
            )),
            captured: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn with_captured(mut self, captured: Arc<Mutex<Vec<ChatRequest>>>) -> Self {
        self.captured = captured;
        self
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
        request: ChatRequest,
    ) -> Result<BoxStream<'static, Result<ChatStreamChunk, ProviderError>>, ProviderError> {
        self.captured
            .lock()
            .expect("捕获锁中毒")
            .push(request);
        let script = self
            .scripts
            .lock()
            .expect("脚本队列锁中毒")
            .pop_front()
            .ok_or_else(|| ProviderError::Message("脚本已耗尽".to_string()))?;
        let s: BoxStream<'static, Result<ChatStreamChunk, ProviderError>> = match script {
            Script::Chunks(chunks) => Box::pin(stream::iter(chunks.into_iter().map(Ok))),
            Script::HangAfterFirstDelta => {
                // 先发一个增量，然后永久挂起（用于取消测试）
                let chunk = ChatStreamChunk {
                    delta: Some("开始工作…".to_string()),
                    tool_calls: vec![],
                    usage: None,
                    finish_reason: None,
                };
                Box::pin(stream::iter([Ok(chunk)]).chain(stream::pending()))
            }
        };
        Ok(s)
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
        None,
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
        None,
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

/// 上下文注入：run_task_with_context 把项目指令与仓库结构带进模型输入。
#[tokio::test]
async fn context_injection_reaches_provider() {
    let workspace = temp_workspace("context");
    std::fs::write(workspace.join("AGENTS.md"), "# 项目约定：所有注释使用中文").expect("写指令");
    std::fs::create_dir_all(workspace.join("src")).expect("建 src");
    std::fs::write(workspace.join("src/app.rs"), "fn app() {}\n").expect("写源码");

    let bus = EventBus::new();
    let policy = Arc::new(ApprovalToolPolicy::new(
        PermissionConfig {
            ask_timeout_secs: 10,
            ..PermissionConfig::default()
        },
        "ctx-session".to_string(),
        bus.clone(),
    ));

    let captured: Arc<Mutex<Vec<ChatRequest>>> = Arc::new(Mutex::new(Vec::new()));
    let provider = ScriptedProvider::new(vec![vec![text_chunk("收到。")]])
        .with_captured(captured.clone());

    let agent = ToolAgent::builder(provider)
        .model("scripted-mock")
        .with_policy(policy.clone())
        .build();

    let store = FileSessionStore::new(workspace.join("sessions")).expect("创建会话存储");
    let session =
        CodingSession::start(Arc::new(agent), &workspace, store, bus.clone(), policy.clone(), None)
            .expect("会话启动");

    let assembler = rucora_coding::context::ContextAssembler::new(
        &workspace,
        rucora_coding::context::ContextConfig::default(),
    );
    let outcome = session
        .run_task_with_context("修复编译错误", &assembler)
        .await
        .expect("任务应成功");
    assert!(outcome.success);

    // 模型收到的输入应包含上下文块与任务
    let requests = captured.lock().expect("捕获锁中毒");
    assert_eq!(requests.len(), 1);
    let input_text = requests[0].messages.last().expect("应有用户消息").content_text();
    assert!(input_text.contains("<project-context>"), "应有上下文标签：{input_text}");
    assert!(input_text.contains("项目约定"), "应含 AGENTS.md 指令");
    assert!(input_text.contains("app.rs"), "应含仓库结构：{input_text}");
    assert!(input_text.contains("<task>修复编译错误</task>"));

    std::fs::remove_dir_all(&workspace).ok();
}

/// 取消：挂起流 + cancel() → 任务以 Cancelled 结束，事件与状态一致。
#[tokio::test]
async fn cancel_stops_running_task() {
    let workspace = temp_workspace("cancel");
    let bus = EventBus::new();
    let policy = Arc::new(ApprovalToolPolicy::new(
        PermissionConfig::default(),
        "cancel-session".to_string(),
        bus.clone(),
    ));

    let provider = ScriptedProvider::new(vec![vec![]])
        .with_captured(Arc::new(Mutex::new(Vec::new())));
    // 将脚本替换为"挂起流"：先发一个增量，然后永久等待
    provider
        .scripts
        .lock()
        .expect("脚本队列锁中毒")
        .clear();
    provider
        .scripts
        .lock()
        .expect("脚本队列锁中毒")
        .push_back(Script::HangAfterFirstDelta);

    let agent = ToolAgent::builder(provider)
        .model("scripted-mock")
        .with_policy(policy.clone())
        .build();

    let store = FileSessionStore::new(workspace.join("sessions")).expect("创建会话存储");
    let session =
        CodingSession::start(Arc::new(agent), &workspace, store, bus.clone(), policy.clone(), None)
            .expect("会话启动");

    // 后台任务：收到首个 MessageDelta 后取消
    let canceller = {
        let session = session.clone();
        let mut sub = session.events().subscribe();
        tokio::spawn(async move {
            loop {
                match sub.receiver.recv().await {
                    Ok((_, AgentEvent::MessageDelta { .. })) => {
                        session.cancel();
                        return;
                    }
                    Ok(_) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => return,
                }
            }
        })
    };

    let outcome = session.run_task("长时间任务").await.expect("取消不是错误");
    canceller.await.expect("取消任务不应 panic");

    assert!(outcome.cancelled, "应标记取消");
    assert!(!outcome.success);

    // 状态与事件一致
    let record = session.record_snapshot();
    assert_eq!(record.status, SessionStatus::Cancelled);
    let events: Vec<AgentEvent> = session
        .event_history()
        .into_iter()
        .map(|(_, ev)| ev)
        .collect();
    assert!(
        events.iter().any(|ev| matches!(ev, AgentEvent::Cancelled)),
        "应有 Cancelled 事件"
    );
    assert!(
        !events.iter().any(|ev| matches!(ev, AgentEvent::TaskCompleted { .. })),
        "取消后不应有 TaskCompleted"
    );

    std::fs::remove_dir_all(&workspace).ok();
}

/// 闭环 + 变更追踪：编辑 → tracker 捕获 → diff/回滚 → 文件还原。
#[tokio::test]
async fn tracked_changes_diff_and_rollback() {
    let workspace = temp_workspace("tracked");
    let src = workspace.join("src.txt");
    std::fs::write(&src, "第一行\n第二行\n").expect("写入初始文件");

    let bus = EventBus::new();
    let tracker = Arc::new(rucora_coding::diff::ChangeTracker::default());
    let policy = Arc::new(ApprovalToolPolicy::new(
        PermissionConfig {
            ask_timeout_secs: 10,
            ..PermissionConfig::default()
        }
        .with_rule("file_edit", rucora_coding::permissions::PermissionAction::Allow),
        "tracked-session".to_string(),
        bus.clone(),
    ));

    let edit_call = ToolCall {
        id: "call-1".to_string(),
        name: "file_edit".to_string(),
        input: serde_json::json!({
            "path": src.display().to_string(),
            "old_string": "第一行",
            "new_string": "第一行已修改\n新增行"
        }),
    };
    let provider = ScriptedProvider::new(vec![
        vec![tool_call_chunk(edit_call)],
        vec![text_chunk("编辑完成。")],
    ]);

    let agent = ToolAgent::builder(provider)
        .model("scripted-mock")
        // 关键：用 ChangeRecorder 包装文件工具，变更才会被捕获
        .tool(rucora_coding::diff::ChangeRecorder::wrap(
            FileEditTool::new().with_allowed_dirs(vec![workspace.clone()]),
            tracker.clone(),
            &workspace,
        ))
        .with_policy(policy.clone())
        .build();

    let store = FileSessionStore::new(workspace.join("sessions")).expect("创建会话存储");
    let session = CodingSession::start(
        Arc::new(agent),
        &workspace,
        store,
        bus.clone(),
        policy.clone(),
        Some(tracker.clone()),
    )
    .expect("会话启动");

    let outcome = session.run_task("修改 src.txt").await.expect("任务应成功");
    assert!(outcome.success);
    assert_eq!(outcome.changes, 1, "应捕获 1 个文件变更");

    // diff 内容正确（统一 diff + 统计）
    let diffs = session.change_diffs();
    assert_eq!(diffs.len(), 1);
    let diff = &diffs[0];
    assert!(diff.diff.contains("---"), "应有统一 diff 头：{}", diff.diff);
    assert!(diff.diff.contains("+++"));
    assert!(diff.diff.contains("+第一行已修改"));
    assert!(diff.diff.contains("-第一行"));
    assert!(diff.diff.contains("+新增行"));
    assert_eq!(diff.added_lines, 2);
    assert_eq!(diff.removed_lines, 1);
    assert!(!diff.created);

    // 回滚：文件恢复原内容
    let restored = session.rollback_all().expect("回滚");
    assert_eq!(restored, 1);
    let content = std::fs::read_to_string(&src).expect("读取回滚后的文件");
    assert_eq!(content, "第一行\n第二行\n");
    assert!(tracker.diffs().is_empty(), "回滚后不再有未回滚变更");

    // 会话记录持久化了变更
    let record = session.record_snapshot();
    assert_eq!(record.changes.len(), 1);
    assert!(record.changes[0].rolled_back);

    std::fs::remove_dir_all(&workspace).ok();
}

/// 新建文件的回滚：删除文件本身。
#[tokio::test]
async fn rollback_deletes_created_file() {
    let workspace = temp_workspace("created");
    let new_file = workspace.join("new.txt");

    let bus = EventBus::new();
    let policy = Arc::new(ApprovalToolPolicy::new(
        PermissionConfig::default().with_rule("file_write", rucora_coding::permissions::PermissionAction::Allow),
        "created-session".to_string(),
        bus.clone(),
    ));
    let tracker = Arc::new(rucora_coding::diff::ChangeTracker::default());


    let write_call = ToolCall {
        id: "call-1".to_string(),
        name: "file_write".to_string(),
        input: serde_json::json!({
            "path": new_file.display().to_string(),
            "content": "新建内容"
        }),
    };
    let provider = ScriptedProvider::new(vec![
        vec![tool_call_chunk(write_call)],
        vec![text_chunk("已写入。")],
    ]);

    let agent = ToolAgent::builder(provider)
        .model("scripted-mock")
        .tool(rucora_coding::diff::ChangeRecorder::wrap(
            rucora::tools::FileWriteTool::new().with_allowed_dirs(vec![workspace.clone()]),
            tracker.clone(),
            &workspace,
        ))
        .with_policy(policy.clone())
        .build();

    let store = FileSessionStore::new(workspace.join("sessions")).expect("创建会话存储");
    let session = CodingSession::start(
        Arc::new(agent),
        &workspace,
        store,
        bus.clone(),
        policy.clone(),
        Some(tracker.clone()),
    )
    .expect("会话启动");

    session.run_task("新建 new.txt").await.expect("任务应成功");

    // 新建文件出现在 diff 中且 created=true
    let diffs = session.change_diffs();
    assert_eq!(diffs.len(), 1);
    assert!(diffs[0].created, "应为新建文件");

    // 回滚 → 文件被删除
    assert_eq!(session.rollback_all().expect("回滚"), 1);
    assert!(!new_file.exists(), "回滚后新建文件应被删除");

    std::fs::remove_dir_all(&workspace).ok();
}
