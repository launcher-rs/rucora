use agentkit::{SkillRegistry, skills::EchoSkill};
use agentkit_core::skill::types::SkillContext;
use serde_json::json;

#[tokio::main]
async fn main() {
    let skills = SkillRegistry::new().register(EchoSkill);

    let out = skills
        .run(
            "echo",
            SkillContext {
                input: json!({
                    "text": "hello",
                    "n": 1,
                }),
            },
        )
        .await;

    match out {
        Ok(v) => println!("{}", v.output),
        Err(e) => eprintln!("调用失败：{}", e),
    }
}
