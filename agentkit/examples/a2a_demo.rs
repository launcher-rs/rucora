#[cfg(not(feature = "a2a"))]
fn main() {
    eprintln!("This example requires feature 'a2a'. Try: cargo run -p agentkit --example a2a_demo --features a2a");
}

#[cfg(feature = "a2a")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use agentkit::a2a::{
        protocol::{A2aMessage, A2aTask, AgentId, TaskId},
        transport::{A2aTransport, InProcessA2aTransport},
    };
    use serde_json::json;

    let t = InProcessA2aTransport::new();

    let mut worker_rx = t.register(AgentId("worker".to_string())).await?;

    t.send(
        &AgentId("worker".to_string()),
        A2aMessage::Task(A2aTask {
            id: TaskId("t1".to_string()),
            from: AgentId("leader".to_string()),
            to: AgentId("worker".to_string()),
            payload: json!({"task":"say_hi"}),
        }),
    )
    .await?;

    if let Some(msg) = worker_rx.recv().await {
        match msg {
            A2aMessage::Task(task) => {
                println!("worker received task: {} from {}", task.id.0, task.from.0);
                println!("payload: {}", task.payload);
            }
            other => {
                println!("worker received other message: {other:?}");
            }
        }
    }

    Ok(())
}
