//! 子 Agent 编排集成测试：结果回传、共享变更追踪、超时保护。

use std::sync::Arc;

use rucora::agent::ToolAgent;
use rucora::tools::FileEditTool;
use rucora_core::tool::types::ToolCall;
use rucora_coding::events::{AgentEvent, EventBus};
use rucora_coding::permissions::{ApprovalToolPolicy, PermissionConfig};
use rucora_coding::runner::CodingSession;
use rucora_coding::session::{FileSessionStore, TranscriptEntry};

mod common;
use common::{temp_workspace, text_chunk, tool_call_chunk, ScriptedProvider};

fn allow_subagent(config: PermissionConfig) -> PermissionConfig {
    config.with_rule("subagent", rucora_coding::permissions::PermissionAction::Allow)
}

/// 子 Agent 编排：主循环派生子任务，结果以结构化摘要回传，嵌套事件实时上报。
#[tokio::test]
async fn subagent_result_returns_to_main_loop() {
    let workspace = temp_workspace("subagent");
    let bus = EventBus::new();
    let policy = Arc::new(ApprovalToolPolicy::new(
        allow_subagent(PermissionConfig::default()),
        "sub-session".to_string(),
        bus.clone(),
    ));

    let sub_provider = Arc::new(ScriptedProvider::new(vec![
        vec![text_chunk("子任务调查结果：发现 3 个问题。")],
    ]));
    let sub_provider_in_tool = sub_provider.clone();
    let policy_in_tool = policy.clone();
    let bus_in_tool = bus.clone();

    let agent = ToolAgent::builder(Arc::new(ScriptedProvider::new(vec![
        vec![
            text_chunk("这个任务需要调研，我派子任务。"),
            tool_call_chunk(ToolCall {
                id: "call-1".to_string(),
                name: "subagent".to_string(),
                input: serde_json::json!({"task": "调研仓库结构并汇报问题"}),
            }),
        ],
        vec![text_chunk("子任务完成，我已汇总。")],
    ])))
    .model("main-mock")
    .tool(rucora_coding::subagent::SubAgentTool::new(
        move |_task| {
            let agent = ToolAgent::builder(sub_provider_in_tool.clone())
                .model("sub-mock")
                .system_prompt("你是子任务执行者")
                .with_policy(policy_in_tool.clone())
                .with_observer(rucora_coding::subagent::SubAgentObserver::new(
                    bus_in_tool.clone(),
                ))
                .build();
            Ok(Arc::new(agent))
        },
        workspace.clone(),
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
        None,
        None,
    )
    .expect("会话启动");

    let outcome = session.run_task("完成整体目标").await.expect("任务应成功");
    assert!(outcome.success);
    assert_eq!(outcome.tool_calls, 1);
    assert!(outcome.text.contains("已汇总"));

    // 主循环收到的工具结果含子任务摘要
    let record = session.record_snapshot();
    let sub_entry = record.transcript.iter().find_map(|entry| match entry {
        TranscriptEntry::ToolCall { name, output, success, .. } if name == "subagent" => {
            Some((output.clone(), *success))
        }
        _ => None,
    });
    let (output, success) = sub_entry.expect("应有 subagent 工具记录");
    assert!(success);
    // 执行器会把工具返回值包装为 {"ok": ..., "output": <工具JSON>}
    let inner = output
        .get("output")
        .cloned()
        .unwrap_or_else(|| output.clone());
    assert_eq!(inner["ok"], true);
    assert!(
        inner["summary"].as_str().expect("摘要").contains("发现 3 个问题"),
        "摘要应来自子 Agent：{output}"
    );

    // 子 Agent 收到独立任务输入（上下文隔离：不含主会话文本）
    let requests = sub_provider.captured_requests();
    assert_eq!(requests.len(), 1);
    let sub_input = requests[0]
        .messages
        .last()
        .expect("应有子任务消息")
        .content_text();
    assert!(sub_input.contains("调研仓库结构"), "子任务描述应传入：{sub_input}");
    assert!(!sub_input.contains("这个任务需要调研"), "不应继承主会话文本");

    // 嵌套事件已上报
    let events: Vec<AgentEvent> = session
        .event_history()
        .into_iter()
        .map(|(_, ev)| ev)
        .collect();
    assert!(
        events.iter().any(|ev| matches!(
            ev,
            AgentEvent::SubAgentDelta { delta } if delta.contains("子任务调查结果")
        )),
        "应有子任务文本增量事件"
    );
    // 子任务未调用工具：不应有嵌套工具事件
    assert!(
        !events
            .iter()
            .any(|ev| matches!(ev, AgentEvent::SubAgentToolStarted { .. })),
        "无工具子任务不应有嵌套工具开始事件"
    );

    std::fs::remove_dir_all(&workspace).ok();
}

/// 子 Agent 编辑文件 + 共享 ChangeTracker：父会话可回滚子任务的修改。
#[tokio::test]
async fn subagent_file_changes_shared_with_parent_tracker() {
    let workspace = temp_workspace("sub-tracked");
    let src = workspace.join("shared.txt");
    std::fs::write(&src, "原始内容").expect("写初始文件");

    let bus = EventBus::new();
    let policy = Arc::new(ApprovalToolPolicy::new(
        allow_subagent(PermissionConfig::default())
            .with_rule("file_edit", rucora_coding::permissions::PermissionAction::Allow),
        "sub-tracked-session".to_string(),
        bus.clone(),
    ));
    let tracker = Arc::new(rucora_coding::diff::ChangeTracker::default());
    let tracker_in_tool = tracker.clone();
    let policy_in_tool = policy.clone();
    let workspace_in_tool = workspace.clone();

    let edit_call = ToolCall {
        id: "sub-call-1".to_string(),
        name: "file_edit".to_string(),
        input: serde_json::json!({
            "path": src.display().to_string(),
            "old_string": "原始内容",
            "new_string": "被子任务修改了"
        }),
    };
    let sub_provider = Arc::new(ScriptedProvider::new(vec![
        vec![tool_call_chunk(edit_call)],
        vec![text_chunk("子任务编辑完成。")],
    ]));
    let sub_provider_in_tool = sub_provider.clone();

    let agent = ToolAgent::builder(Arc::new(ScriptedProvider::new(vec![
        vec![
            tool_call_chunk(ToolCall {
                id: "call-1".to_string(),
                name: "subagent".to_string(),
                input: serde_json::json!({"task": "编辑 shared.txt"}),
            }),
        ],
        vec![text_chunk("子任务已编辑文件。")],
    ])))
    .model("main-mock")
    .tool(rucora_coding::subagent::SubAgentTool::new(
        move |_task| {
            let agent = ToolAgent::builder(sub_provider_in_tool.clone())
                .model("sub-mock")
                .tool(rucora_coding::diff::ChangeRecorder::wrap(
                    FileEditTool::new().with_allowed_dirs(vec![workspace_in_tool.clone()]),
                    tracker_in_tool.clone(),
                    &workspace_in_tool,
                ))
                .with_policy(policy_in_tool.clone())
                .build();
            Ok(Arc::new(agent))
        },
        workspace.clone(),
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
        None,
    )
    .expect("会话启动");

    session.run_task("让子任务编辑文件").await.expect("任务应成功");

    // 文件被子任务修改
    assert_eq!(std::fs::read_to_string(&src).expect("读取"), "被子任务修改了");

    // 父会话的追踪器捕获了子任务的修改（共享 tracker）
    let diffs = session.change_diffs();
    assert_eq!(diffs.len(), 1, "子任务的编辑应进入父追踪器");
    assert!(diffs[0].diff.contains("+被子任务修改了"));

    // 回滚还原
    assert_eq!(session.rollback_all().expect("回滚"), 1);
    assert_eq!(std::fs::read_to_string(&src).expect("读取"), "原始内容");

    std::fs::remove_dir_all(&workspace).ok();
}

/// 超时保护：子任务挂起 → 超时以 ok:false 返回 → 主循环继续。
#[tokio::test]
async fn subagent_timeout_returns_structured_error() {
    let workspace = temp_workspace("sub-timeout");
    let bus = EventBus::new();
    let policy = Arc::new(ApprovalToolPolicy::new(
        allow_subagent(PermissionConfig::default()),
        "timeout-session".to_string(),
        bus.clone(),
    ));

    let policy_in_tool = policy.clone();
    let agent = ToolAgent::builder(Arc::new(ScriptedProvider::new(vec![
        vec![
            tool_call_chunk(ToolCall {
                id: "call-1".to_string(),
                name: "subagent".to_string(),
                input: serde_json::json!({"task": "永远不会完成的任务"}),
            }),
        ],
        vec![text_chunk("子任务超时了，我调整策略。")],
    ])))
    .model("main-mock")
    .tool(
        rucora_coding::subagent::SubAgentTool::new(
            move |_task| {
                let provider = ScriptedProvider::new(vec![]);
                provider.push_script(ScriptedProvider::hang_script());
                let agent = ToolAgent::builder(Arc::new(provider))
                    .model("sub-mock")
                    .build();
                Ok(Arc::new(agent))
            },
            workspace.clone(),
        )
        .with_timeout(std::time::Duration::from_secs(1)),
    )
    .with_policy(policy_in_tool.clone())
    .build();

    let store = FileSessionStore::new(workspace.join("sessions")).expect("创建会话存储");
    let session = CodingSession::start(
        Arc::new(agent),
        &workspace,
        store,
        bus.clone(),
        policy.clone(),
        None,
        None,
    )
    .expect("会话启动");

    let outcome = session.run_task("触发超时").await.expect("任务应正常结束");
    assert!(outcome.success, "超时不应中断主任务");

    // 超时以结构化错误回传主循环
    let record = session.record_snapshot();
    let sub_entry = record.transcript.iter().find_map(|entry| match entry {
        TranscriptEntry::ToolCall { name, output, .. } if name == "subagent" => Some(output.clone()),
        _ => None,
    });
    let output = sub_entry.expect("应有 subagent 工具记录");
    let inner = output
        .get("output")
        .cloned()
        .unwrap_or_else(|| output.clone());
    assert_eq!(inner["ok"], false);
    assert_eq!(inner["error"]["kind"], "subagent_timeout");

    std::fs::remove_dir_all(&workspace).ok();
}
