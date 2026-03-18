use agentkit_core::tool::Tool;
use serde_json::json;

#[tokio::test]
async fn http_request_tool_should_fetch_rustcc_article() {
    let tool = agentkit::tools::HttpRequestTool::new();

    let out = tool
        .call(json!({
            "method": "GET",
            "url": "https://rustcc.cn/article?id=a122f1ed-44bd-4e72-9dd5-ca901331370b",
            "headers": {
                "Accept": "text/html",
                "User-Agent": "agentkit-http-tool-smoke-test"
            },
            "timeout": 30
        }))
        .await
        .expect("http_request tool call failed");

    let status = out
        .get("status")
        .and_then(|v| v.as_u64())
        .expect("missing status") as u16;

    let success = out
        .get("success")
        .and_then(|v| v.as_bool())
        .expect("missing success");

    let body = out
        .get("body")
        .and_then(|v| v.as_str())
        .expect("missing body");

    assert!(status >= 200 && status < 400, "unexpected status: {}", status);
    assert!(success || (status >= 300 && status < 400));
    assert!(!body.is_empty(), "body should not be empty");
    assert!(
        body.contains("rustcc") || body.contains("Rust") || body.contains("<html"),
        "body does not look like html"
    );
}
