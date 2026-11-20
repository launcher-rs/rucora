//! SessionManager 集成测试：多会话注册、审批聚合、批量取消、记录删除保护。

use std::sync::Arc;

use rucora::agent::ToolAgent;
use rucora_core::tool::types::ToolCall;
use rucora_coding::events::AgentEvent;
use rucora_coding::manager::SessionManager;
use rucora_coding::permissions::{ApprovalDecision, ApprovalToolPolicy, PermissionConfig};
use rucora_coding::session::{FileSessionStore, SessionStatus};

mod common;
use common::{temp_workspace, text_chunk, tool_call_chunk, ScriptedProvider};

/// 启动两个会话：注册、ID 唯一、审批聚合空态与 resolve miss。
#[tokio::test]
async fn launch_registers_two_sessions() {
    let workspace = temp_workspace("launch");
    let manager =
        SessionManager::new(FileSessionStore::new(workspace.join("sessions")).expect("存储"));

    let session_a = manager
        .launch(&workspace, |blueprint| {
            let policy = Arc::new(ApprovalToolPolicy::new(
                PermissionConfig::default(),
                blueprint.session_id.clone(),
                blueprint.bus.clone(),
            ));
            let agent = ToolAgent::builder(Arc::new(ScriptedProvider::new(vec![vec![text_chunk(
                "A 完成",
            )]])))
            .model("mock")
            .with_policy(policy.clone())
            .build();
            Ok((Arc::new(agent), policy, None))
        })
        .expect("会话 A 启动");

    let session_b = manager
        .launch(&workspace, |blueprint| {
            let policy = Arc::new(ApprovalToolPolicy::new(
                PermissionConfig::default(),
                blueprint.session_id.clone(),
                blueprint.bus.clone(),
            ));
            let agent = ToolAgent::builder(Arc::new(ScriptedProvider::new(vec![vec![text_chunk(
                "B 完成",
            )]])))
            .model("mock")
            .with_policy(policy.clone())
            .build();
            Ok((Arc::new(agent), policy, None))
        })
        .expect("会话 B 启动");

    assert_eq!(manager.active_count(), 2);
    assert_ne!(session_a.id(), session_b.id(), "会话 ID 应唯一");
    assert!(manager.get(&session_a.id()).is_some());
    assert!(manager.get(&session_b.id()).is_some());

    // 无审批时聚合为空；对未知会话裁决返回 false
    assert!(manager.pending_approvals().is_empty());
    assert!(!manager.resolve_approval("no-such-session", "req-1", ApprovalDecision::Allow));

    // 会话正常运行任务
    let outcome = session_a.run_task("任务").await.expect("任务应成功");
    assert!(outcome.success);

    std::fs::remove_dir_all(&workspace).ok();
}

/// 审批聚合与路由：会话 A 的待审批请求出现在 manager 视角，
/// 通过 manager.resolve_approval 送达并放行。
#[tokio::test]
async fn pending_approval_aggregation_and_routing() {
    let workspace = temp_workspace("route");
    let manager =
        SessionManager::new(FileSessionStore::new(workspace.join("sessions")).expect("存储"));

    let edit_call = ToolCall {
        id: "call-1".to_string(),
        name: "file_edit".to_string(),
        input: serde_json::json!({
            "path": workspace.join("src.txt").display().to_string(),
            "old_string": "hello",
            "new_string": "world"
        }),
    };
    std::fs::write(workspace.join("src.txt"), "hello").expect("写初始文件");

    let session = manager
        .launch(&workspace, |blueprint| {
            let policy = Arc::new(ApprovalToolPolicy::new(
                PermissionConfig {
                    ask_timeout_secs: 10,
                    ..PermissionConfig::default()
                },
                blueprint.session_id.clone(),
                blueprint.bus.clone(),
            ));
            let agent = ToolAgent::builder(Arc::new(ScriptedProvider::new(vec![
                vec![tool_call_chunk(edit_call)],
                vec![text_chunk("完成。")],
            ])))
            .model("mock")
            .tool(rucora::tools::FileEditTool::new().with_allowed_dirs(vec![workspace.clone()]))
            .with_policy(policy.clone())
            .build();
            Ok((Arc::new(agent), policy, None))
        })
        .expect("会话启动");
    let session_id = session.id();

    // 运行中：manager 视角聚合到该会话的审批请求，并放行
    let router = {
        let manager = manager.clone();
        let mut sub = session.events().subscribe();
        tokio::spawn(async move {
            loop {
                match sub.receiver.recv().await {
                    Ok((_, AgentEvent::ApprovalRequired { .. })) => {
                        let routes = manager.pending_approvals();
                        assert_eq!(routes.len(), 1, "manager 应聚合到 1 条审批");
                        assert_eq!(routes[0].session_id, session_id);
                        assert_eq!(routes[0].request.tool_name, "file_edit");
                        manager.resolve_approval(
                            &routes[0].session_id,
                            &routes[0].request.request_id,
                            ApprovalDecision::Allow,
                        );
                        return;
                    }
                    Ok((_, AgentEvent::TaskCompleted { .. })) => return,
                    Ok(_) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => return,
                }
            }
        })
    };

    let outcome = session.run_task("修改 src.txt").await.expect("任务应成功");
    assert!(outcome.success);
    router.await.expect("路由任务不应 panic");
}

/// cancel_all 批量取消挂起会话。
#[tokio::test]
async fn cancel_all_stops_hanging_sessions() {
    let workspace = temp_workspace("cancel-all");
    let manager =
        SessionManager::new(FileSessionStore::new(workspace.join("sessions")).expect("存储"));

    let session = manager
        .launch(&workspace, |blueprint| {
            let policy = Arc::new(ApprovalToolPolicy::new(
                PermissionConfig::default(),
                blueprint.session_id.clone(),
                blueprint.bus.clone(),
            ));
            let provider = ScriptedProvider::new(vec![]);
            provider.push_script(ScriptedProvider::hang_script());
            let agent = ToolAgent::builder(Arc::new(provider))
                .model("mock")
                .with_policy(policy.clone())
                .build();
            Ok((Arc::new(agent), policy, None))
        })
        .expect("会话启动");

    let run = tokio::spawn({
        let session = session.clone();
        async move { session.run_task("长任务").await }
    });
    // 等任务开始（挂起流已发出首个增量）
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    assert_eq!(manager.cancel_all(), 1);
    let outcome = run.await.expect("任务不应 panic").expect("取消不是错误");
    assert!(outcome.cancelled);
    assert_eq!(session.record_snapshot().status, SessionStatus::Cancelled);
}

/// resume 经管理器注册；活动会话记录删除保护；detach 后可删。
#[tokio::test]
async fn resume_register_and_delete_protection() {
    let workspace = temp_workspace("resume-del");
    let manager =
        SessionManager::new(FileSessionStore::new(workspace.join("sessions")).expect("存储"));

    let session = manager
        .launch(&workspace, |blueprint| {
            let policy = Arc::new(ApprovalToolPolicy::new(
                PermissionConfig::default(),
                blueprint.session_id.clone(),
                blueprint.bus.clone(),
            ));
            let agent = ToolAgent::builder(Arc::new(ScriptedProvider::new(vec![vec![text_chunk(
                "第一轮完成",
            )]])))
            .model("mock")
            .with_policy(policy.clone())
            .build();
            Ok((Arc::new(agent), policy, None))
        })
        .expect("会话启动");
    let record_id = session.id();
    session.run_task("任务一").await.expect("任务应成功");
    manager.detach(&record_id);
    assert_eq!(manager.active_count(), 0);

    // 经管理器恢复
    let resumed = manager
        .resume_session(&workspace, &record_id, |blueprint| {
            let policy = Arc::new(ApprovalToolPolicy::new(
                PermissionConfig::default(),
                blueprint.session_id.clone(),
                blueprint.bus.clone(),
            ));
            let agent = ToolAgent::builder(Arc::new(ScriptedProvider::new(vec![vec![text_chunk(
                "恢复后继续",
            )]])))
            .model("mock")
            .with_policy(policy.clone())
            .build();
            Ok((Arc::new(agent), policy, None))
        })
        .expect("恢复会话");
    assert_eq!(resumed.id(), record_id);
    assert!(manager.get(&record_id).is_some());

    // 活动会话的记录不可删除；detach 后可删
    assert!(manager.delete_record(&record_id).is_err());
    assert!(manager.detach(&record_id));
    assert!(manager.delete_record(&record_id).expect("删除"));
    assert!(!manager.delete_record(&record_id).expect("重复删除返回 false"));

    // 恢复事件已发布
    let events: Vec<AgentEvent> = resumed
        .event_history()
        .into_iter()
        .map(|(_, ev)| ev)
        .collect();
    assert!(
        events
            .iter()
            .any(|ev| matches!(ev, AgentEvent::SessionResumed { .. })),
        "应有 SessionResumed 事件"
    );

    std::fs::remove_dir_all(&workspace).ok();
}
