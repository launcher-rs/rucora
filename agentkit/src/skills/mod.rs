//! Skills（技能）相关实现。
//!
//! Skill 往往是对 Tool/Provider/Memory 的组合封装。
//! 目前这里先提供一个占位模块，便于后续添加可复用技能。

use agentkit_core::{
    error::SkillError,
    skill::{types::{SkillContext, SkillOutput}, Skill},
};
use async_trait::async_trait;
use serde_json::{json, Value};

/// 一个最简单的示例 Skill：把输入原样返回。
pub struct EchoSkill;

#[async_trait]
impl Skill for EchoSkill {
    fn name(&self) -> &str {
        "echo"
    }

    fn description(&self) -> Option<&str> {
        Some("原样返回输入参数")
    }

    async fn run(&self, ctx: SkillContext) -> Result<SkillOutput, SkillError> {
        let input: Value = ctx.input;

        Ok(SkillOutput {
            output: json!({
                "input": input,
            }),
        })
    }
}
