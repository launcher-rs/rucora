use agentkit::skills::testkit::{
    load_skills_with_mock_tools, unique_temp_dir, write_skill_rhai, MockToolInvoker,
};
use agentkit_runtime::ToolRegistry;
use serde_json::{json, Value};

#[tokio::test]
async fn skills_rhai_should_run_with_stdlib_call_tool() {
    let dir = unique_temp_dir("skills-stdlib");
    tokio::fs::create_dir_all(&dir).await.unwrap();

    let meta = r#"
name: demo
version: 0.1.0
capabilities: ["cmd_exec"]
"#;

    let script = r#"
let res = call_tool("cmd_exec", #{ command: "echo hi" });
#{ success: true, res: res }
"#;

    write_skill_rhai(&dir, "demo", meta, script, None)
        .await
        .expect("write skill");

    let mock = MockToolInvoker::new().register(
        "cmd_exec",
        std::sync::Arc::new(|input: Value| Ok(json!({"ok": true, "input": input}))),
    );

    let skills = load_skills_with_mock_tools(&dir, mock)
        .await
        .expect("load skills");

    let mut tools = ToolRegistry::new();
    for t in skills.as_tools() {
        tools = tools.register_arc(t);
    }

    let out = tools
        .get("demo")
        .expect("missing tool")
        .call(json!({"user_input": "x"}))
        .await
        .expect("tool call");

    assert!(out
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false));
    assert_eq!(out["res"]["ok"], json!(true));
}

#[tokio::test]
async fn skills_loader_should_validate_manifest_name() {
    let dir = unique_temp_dir("skills-manifest");
    tokio::fs::create_dir_all(&dir).await.unwrap();

    let meta = r#"
name: ""
version: 0.1.0
"#;

    let script = r#"#{ success: true }"#;

    write_skill_rhai(&dir, "bad_skill", meta, script, None)
        .await
        .expect("write skill");

    let mock = MockToolInvoker::new();
    let err = load_skills_with_mock_tools(&dir, mock)
        .await
        .err()
        .expect("should error");

    let msg = err.to_string();
    assert!(msg.contains("manifest"));
    assert!(msg.contains("name"));
}
