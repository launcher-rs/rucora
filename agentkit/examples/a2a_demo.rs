#[cfg(not(feature = "a2a"))]
fn main() {
    eprintln!(
        "This example requires feature 'a2a'. Try: cargo run -p agentkit --example a2a_demo --features a2a"
    );
}

#[cfg(feature = "a2a")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use agentkit::a2a::{
        client::Client,
        types::{Message, MessageSendParams, Part, SendMessageResult},
    };

    let base_url =
        std::env::var("A2A_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());
    let client = Client::from_url(&base_url)?;

    let msg = Message::user(vec![Part::text("Hello from agentkit!")]);
    let result = client.send_message(&MessageSendParams::new(msg)).await?;
    match result {
        SendMessageResult::Task(task) => {
            let reply = task.status.message.as_ref().and_then(|m| m.text_content());
            println!("[{:?}] {}", task.status.state, reply.unwrap_or_default());
        }
        SendMessageResult::Message(msg) => println!("{}", msg.text_content().unwrap_or_default()),
    }
    Ok(())
}
