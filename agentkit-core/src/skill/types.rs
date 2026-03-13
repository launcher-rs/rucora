//! Skill（技能）相关的类型定义。

use serde_json::Value;

/// 技能执行上下文。
///
/// 为了保持 core 层通用性，这里使用 JSON 作为输入载体。
#[derive(Debug, Clone)]
pub struct SkillContext {
    /// 技能输入参数。
    pub input: Value,
}

/// 技能执行结果。
#[derive(Debug, Clone)]
pub struct SkillOutput {
    /// 技能输出。
    pub output: Value,
}
