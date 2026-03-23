use agentkit_core::error::ToolError;
use agentkit_core::tool::Tool;
use async_trait::async_trait;
use serde_json::{Value, json};

struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "text": {"type": "string"}
            },
            "required": ["text"]
        })
    }

    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let text = input
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Message("missing field: text".to_string()))?;
        Ok(json!({"success": true, "output": text}))
    }
}

#[tokio::test]
async fn tool_contract_should_have_non_empty_name() {
    let t = EchoTool;
    assert!(!t.name().trim().is_empty());
}

#[tokio::test]
async fn tool_contract_call_should_return_json_value_or_tool_error() {
    let t = EchoTool;

    let ok = t.call(json!({"text": "hi"})).await.unwrap();
    assert_eq!(ok.get("success").and_then(|v| v.as_bool()), Some(true));

    let err = t.call(json!({})).await.unwrap_err();
    match err {
        ToolError::Message(msg) => assert!(msg.contains("text")),
        _ => panic!("unexpected ToolError variant"),
    }
}
