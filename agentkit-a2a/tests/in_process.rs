use agentkit_a2a::types::{Message, Part};

#[test]
fn ra2a_message_text_roundtrip() {
    let msg = Message::user(vec![Part::text("hi")]);
    assert_eq!(msg.text_content().unwrap_or_default(), "hi");
}
