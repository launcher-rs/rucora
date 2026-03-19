use agentkit::core::memory::Memory;
use agentkit::memory::{FileMemory, InMemoryMemory};
use agentkit::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mem = InMemoryMemory::new();
    mem.add(MemoryItem {
        id: "core:user_lang".to_string(),
        content: "Rust".to_string(),
        metadata: None,
    })
    .await?;

    let results = mem
        .query(MemoryQuery {
            text: "user_lang".to_string(),
            limit: 10,
        })
        .await?;

    println!("in_memory results: {:#?}", results);

    let file_mem = FileMemory::new("./target/memory_demo.json");
    file_mem
        .add(MemoryItem {
            id: "core:project_stack".to_string(),
            content: "agentkit".to_string(),
            metadata: None,
        })
        .await?;

    let file_results = file_mem
        .query(MemoryQuery {
            text: "project_stack".to_string(),
            limit: 10,
        })
        .await?;

    println!("file results: {:#?}", file_results);

    Ok(())
}
