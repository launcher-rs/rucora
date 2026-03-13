use agentkit::skills::EchoSkill;
use agentkit_core::skill::types::SkillContext;
use serde_json::json;

#[tokio::main]
async fn main() {
    // 直接创建和使用技能，无需注册表
    let echo_skill = EchoSkill::new();

    println!("技能名称: {}", echo_skill.name());
    println!("技能描述: {:?}", echo_skill.description());

    let ctx = SkillContext {
        input: json!({
            "text": "hello",
            "n": 1,
        }),
    };

    match echo_skill.run(ctx).await {
        Ok(result) => println!("技能执行结果: {}", result.output),
        Err(e) => eprintln!("调用失败：{}", e),
    }
}
