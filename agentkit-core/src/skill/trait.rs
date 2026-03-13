use async_trait::async_trait;

use crate::{
    error::SkillError,
    skill::types::{SkillContext, SkillOutput},
};

/// Skill（技能）接口。
#[async_trait]
pub trait Skill: Send + Sync {
    /// 技能名称（建议唯一）。
    fn name(&self) -> &str;

    /// 技能描述（可选）。
    fn description(&self) -> Option<&str> {
        None
    }

    /// 执行技能。
    async fn run(&self, ctx: SkillContext) -> Result<SkillOutput, SkillError>;
}
