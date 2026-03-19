use std::sync::Arc;

use agentkit::{memory::InMemoryMemory, tools::{MemoryRecallTool, MemoryStoreTool}};
use agentkit_core::tool::Tool;
use serde_json::json;

#[tokio::test]
async fn memory_store_and_recall_share_backend() {
    let backend = Arc::new(InMemoryMemory::new());
    let store = MemoryStoreTool::from_memory(backend.clone());
    let recall = MemoryRecallTool::from_memory(backend);

    let store_out = store
        .call(json!({"key":"user_lang","content":"Rust","category":"core"}))
        .await
        .unwrap();

    assert_eq!(store_out["success"], true);

    let recall_out = recall
        .call(json!({"key":"user_lang","category":"core"}))
        .await
        .unwrap();

    assert_eq!(recall_out["found"], true);
    assert_eq!(recall_out["content"], "Rust");
}
