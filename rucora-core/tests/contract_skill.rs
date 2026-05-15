use async_trait::async_trait;
use rucora_core::error::SkillError;
use rucora_core::skill::Skill;
use rucora_core::tool::ToolCategory;
use serde_json::{Value, json};

struct TestSkill;

#[async_trait]
impl Skill for TestSkill {
    fn name(&self) -> &str {
        "test_skill"
    }

    fn description(&self) -> Option<&str> {
        Some("用于测试的技能")
    }

    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::Basic]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "text": {"type": "string", "description": "输入文本"}
            },
            "required": ["text"]
        })
    }

    async fn run_value(&self, input: Value) -> Result<Value, SkillError> {
        let text = input
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SkillError::Message("缺少 'text' 字段".to_string()))?;

        Ok(json!({"echo": text}))
    }
}

#[tokio::test]
async fn skill_contract_should_have_non_empty_name() {
    let skill = TestSkill;
    assert!(!skill.name().trim().is_empty());
}

#[tokio::test]
async fn skill_contract_should_have_input_schema() {
    let skill = TestSkill;
    let schema = skill.input_schema();

    assert!(schema.get("type").is_some());
    assert_eq!(schema.get("type").and_then(|v| v.as_str()), Some("object"));
}

#[tokio::test]
async fn skill_contract_run_value_should_return_json_or_error() {
    let skill = TestSkill;

    // 成功情况
    let result = skill
        .run_value(json!({"text": "hello"}))
        .await
        .unwrap();
    assert_eq!(result.get("echo").and_then(|v| v.as_str()), Some("hello"));

    // 失败情况
    let err = skill.run_value(json!({})).await.unwrap_err();
    match err {
        SkillError::Message(msg) => assert!(msg.contains("text")),
        _ => panic!("unexpected SkillError variant"),
    }
}

#[tokio::test]
async fn skill_contract_should_have_categories() {
    let skill = TestSkill;
    let categories = skill.categories();

    assert!(!categories.is_empty());
    assert!(categories.contains(&ToolCategory::Basic));
}
