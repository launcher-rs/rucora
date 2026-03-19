use agentkit_a2a::{
    protocol::{A2aMessage, A2aTask, AgentId, TaskId},
    transport::{A2aTransport, InProcessA2aTransport},
};
use serde_json::json;

#[tokio::test]
async fn in_process_transport_delivers_task() {
    let t = InProcessA2aTransport::new();

    let mut b_rx = t.register(AgentId("b".to_string())).await.unwrap();

    t.send(
        &AgentId("b".to_string()),
        A2aMessage::Task(A2aTask {
            id: TaskId("t1".to_string()),
            from: AgentId("a".to_string()),
            to: AgentId("b".to_string()),
            payload: json!({"q":"hi"}),
        }),
    )
    .await
    .unwrap();

    let msg = b_rx.recv().await.unwrap();
    match msg {
        A2aMessage::Task(task) => {
            assert_eq!(task.id.0, "t1");
            assert_eq!(task.from.0, "a");
        }
        _ => panic!("unexpected"),
    }
}
