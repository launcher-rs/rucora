use async_trait::async_trait;
use serde_json::Value;

use crate::{error::SkillError, tool::ToolCategory};

/// Skill 抽象：对“可执行能力”的统一描述。
///
/// 说明：
/// - core 层只定义抽象接口（Skill 是什么）
/// - 具体 skill 的实现与注册/编排应放在上层 crate（例如 agentkit / runtime）
#[async_trait]
pub trait Skill: Send + Sync {
    /// Skill 的唯一名称（也可以作为暴露给 LLM 的 tool 名称）。
    fn name(&self) -> &str;

    fn description(&self) -> Option<&str> {
        None
    }

    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::Basic]
    }

    /// 输入参数 schema（JSON Schema）。
    fn input_schema(&self) -> Value;

    /// 执行 skill（输入/输出均为 JSON）。
    async fn run_value(&self, input: Value) -> Result<Value, SkillError>;
}
