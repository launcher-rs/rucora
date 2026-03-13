//! Skills（技能）相关实现。
//!
//! 本模块包含具体的技能实现，是对 agentkit-core 中技能类型定义的具体化。
//!
//! 设计理念：
//! - Skill 是对 Tool/Provider/Memory 的组合封装，提供更高层次的抽象
//! - 每个技能都是独立的可执行单元，具有明确的输入输出
//! - 技能应该专注于解决特定领域的问题
//!
//! 使用示例：
//! ```rust
//! use agentkit::skills::EchoSkill;
//!
//! let skill = EchoSkill::new();
//! let ctx = SkillContext { input: json!("hello") };
//! let result = skill.run(ctx).await?;
//! ```

use agentkit_core::{
    error::SkillError,
    skill::types::{SkillContext, SkillOutput},
};
use serde_json::{Value, json};

/// 一个最简单的示例 Skill：把输入原样返回。
///
/// 这个技能展示了如何实现一个基本的技能：
/// - 定义技能的基本信息（名称、描述）
/// - 实现异步执行方法
/// - 处理输入和输出数据
///
/// 适用场景：
/// - 作为技能实现的参考模板
/// - 测试和调试环境中的占位技能
/// - 数据传递和验证
pub struct EchoSkill {
    /// 技能名称
    pub name: String,
    /// 技能描述
    pub description: Option<String>,
}

impl EchoSkill {
    /// 创建一个新的 EchoSkill 实例。
    ///
    /// 返回的实例具有默认的名称和描述。
    ///
    /// # 示例
    /// ```
    /// let skill = EchoSkill::new();
    /// assert_eq!(skill.name(), "echo");
    /// ```
    pub fn new() -> Self {
        Self {
            name: "echo".to_string(),
            description: Some("原样返回输入参数".to_string()),
        }
    }

    /// 创建具有自定义名称的 EchoSkill。
    ///
    /// # 参数
    /// - `name` - 技能的名称
    ///
    /// # 示例
    /// ```
    /// let skill = EchoSkill::with_name("custom_echo");
    /// assert_eq!(skill.name(), "custom_echo");
    /// ```
    pub fn with_name(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: Some("原样返回输入参数".to_string()),
        }
    }

    /// 创建具有自定义名称和描述的 EchoSkill。
    ///
    /// # 参数
    /// - `name` - 技能的名称
    /// - `description` - 技能的描述
    ///
    /// # 示例
    /// ```
    /// let skill = EchoSkill::with_name_and_desc("custom", "自定义描述");
    /// assert_eq!(skill.name(), "custom");
    /// assert_eq!(skill.description(), Some("自定义描述"));
    /// ```
    pub fn with_name_and_desc(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: Some(description.into()),
        }
    }

    /// 获取技能名称。
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 获取技能描述。
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// 执行技能。
    ///
    /// 将输入的 JSON 数据原样返回，包装在 SkillOutput 中。
    ///
    /// # 参数
    /// - `ctx` - 技能执行上下文，包含输入数据
    ///
    /// # 返回值
    /// - `Ok(SkillOutput)` - 包含原样输入数据的输出
    /// - `Err(SkillError)` - 执行过程中发生的错误
    ///
    /// # 示例
    /// ```no_run
    /// let skill = EchoSkill::new();
    /// let ctx = SkillContext { input: json!("hello") };
    /// let result = skill.run(ctx).await?;
    /// assert_eq!(result.output, json!({"input": "hello"}));
    /// ```
    pub async fn run(&self, ctx: SkillContext) -> Result<SkillOutput, SkillError> {
        let input: Value = ctx.input;

        Ok(SkillOutput {
            output: json!({
                "input": input,
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_echo_skill_basic() {
        let skill = EchoSkill::new();
        assert_eq!(skill.name(), "echo");
        assert_eq!(skill.description(), Some("原样返回输入参数"));
    }

    #[tokio::test]
    async fn test_echo_skill_with_custom_name() {
        let skill = EchoSkill::with_name("custom_echo");
        assert_eq!(skill.name(), "custom_echo");
    }

    #[tokio::test]
    async fn test_echo_skill_execution() {
        let skill = EchoSkill::new();
        let ctx = SkillContext {
            input: json!({
                "message": "hello world",
                "number": 42
            }),
        };

        let result = skill.run(ctx).await.unwrap();
        assert_eq!(
            result.output,
            json!({
                "input": {
                    "message": "hello world",
                    "number": 42
                }
            })
        );
    }
}
