//! Integration tests for rucora macros

use rucora::rucora_guard;
use rucora::rucora_tool;
use rucora::tool_params;
use rucora_core::tool::Tool;
use rucora_core::tool::types::ToolContext;
use rucora_core::{InjectionGuard, ScanResult};
use serde_json::json;

#[rucora_tool(name = "test_add", description = "Test addition")]
async fn test_add(a: i32, b: i32) -> Result<serde_json::Value, rucora_core::error::ToolError> {
    Ok(json!({ "result": a + b }))
}

#[rucora_tool(name = "test_greet", description = "Greet user")]
async fn test_greet(name: String) -> Result<serde_json::Value, rucora_core::error::ToolError> {
    Ok(json!({ "message": format!("Hello, {}!", name) }))
}

#[tokio::test]
async fn test_rucora_tool_macro_expansion() {
    let tool = TestAddTool;
    assert_eq!(tool.name(), "test_add");
    assert_eq!(tool.description(), Some("Test addition"));

    let schema = tool.input_schema();
    assert!(schema["properties"]["a"].is_object());
    assert!(schema["properties"]["b"].is_object());

    let result = tool
        .call(json!({ "a": 1, "b": 2 }), &ToolContext::new())
        .await
        .unwrap();
    assert_eq!(result["result"], 3);
}

#[tokio::test]
async fn test_rucora_tool_macro_string_param() {
    let tool = TestGreetTool;
    assert_eq!(tool.name(), "test_greet");

    let result = tool
        .call(json!({ "name": "Rust" }), &ToolContext::new())
        .await
        .unwrap();
    assert_eq!(result["message"], "Hello, Rust!");
}

#[tokio::test]
async fn test_rucora_tool_macro_invalid_input() {
    let tool = TestAddTool;
    let result = tool
        .call(json!({ "a": "not_a_number", "b": 2 }), &ToolContext::new())
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    // Should be a deserialization error wrapped in ToolError
    assert!(format!("{err}").contains("Invalid parameters"));
}

#[test]
fn test_tool_params_macro_in_test() {
    let schema = tool_params! {
        "id" => (string, required, "User ID"),
        "age" => (number, "User age"),
    };

    assert_eq!(schema["type"], "object");
    let required = schema["required"].as_array().unwrap();
    assert_eq!(required.len(), 1);
    assert_eq!(required[0], "id");
}

/// 简单的长度限制守卫
#[rucora_guard(name = "length-limit")]
fn check_length(content: &str, source: &str) -> ScanResult {
    let _ = source;
    let too_long = content.len() > 5;
    ScanResult {
        is_safe: !too_long,
        threats: vec![],
        cleaned_content: None,
        original_length: content.len(),
    }
}

#[test]
fn test_rucora_guard_macro_generates_working_impl() {
    let guard = LengthLimitGuard;

    let safe = guard.scan("abc", "test");
    assert!(safe.is_safe);
    assert_eq!(safe.original_length, 3);

    let blocked = guard.scan("this is too long", "test");
    assert!(!blocked.is_safe);
    assert_eq!(blocked.original_length, 16);
}
