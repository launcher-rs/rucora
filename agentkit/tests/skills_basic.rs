//! Skills 基本功能测试

#[cfg(feature = "skills")]
#[tokio::test]
async fn test_skills_feature() {
    // 验证 skills feature 可用
    use agentkit::skills;
    
    // 编译通过即表示 skills 模块正确导出
    let _ = std::any::type_name::<skills::SkillRegistry>();
}

#[cfg(feature = "skills")]
#[tokio::test]
async fn test_file_read_skill() {
    use agentkit::skills::FileReadSkill;
    use agentkit_core::skill::Skill;
    
    let skill = FileReadSkill::new();
    
    // 验证技能基本信息
    assert_eq!(skill.name(), "file_read_skill");
    assert!(skill.description().is_some());
    
    // 验证 input schema
    let schema = skill.input_schema();
    assert!(schema.is_object());
}

#[cfg(all(feature = "skills", feature = "rhai-skills"))]
#[tokio::test]
async fn test_rhai_skills_feature() {
    // 验证 rhai-skills feature 可用
    use agentkit::skills::{RhaiEngineRegistrar, RhaiSkill};
    
    // 编译通过即表示类型正确导出
    let _ = std::any::type_name::<RhaiEngineRegistrar>();
    let _ = std::any::type_name::<RhaiSkill>();
}
