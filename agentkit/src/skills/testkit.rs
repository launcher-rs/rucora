use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use agentkit_core::error::SkillError;
use serde_json::{Value, json};
use tokio::fs;

use super::{RhaiToolInvoker, load_skills_from_dir_with_rhai, rhai_stdlib_registrar};

/// 测试用工具处理函数：接收 JSON 入参，返回 JSON 输出或错误字符串。
pub type ToolHandler = Arc<dyn Fn(Value) -> Result<Value, String> + Send + Sync>;

/// 用于测试的“工具调用器”（mock）。
///
/// 设计目的：
/// - 在 skill 单测中，不依赖真实 ToolRegistry / 外部服务。
/// - 通过 `register(name, handler)` 注册若干工具的模拟实现。
/// - 再用 `to_invoker()` 生成一个可注入到 `rhai_stdlib_registrar` 的 invoker。
#[derive(Default, Clone)]
pub struct MockToolInvoker {
    handlers: Arc<Mutex<HashMap<String, ToolHandler>>>,
}

impl MockToolInvoker {
    /// 创建一个空的 mock invoker。
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册一个工具处理函数。
    ///
    /// 说明：
    /// - `name` 为 tool 名称（对应脚本里 `call_tool("name", ...)` 的第一个参数）。
    /// - `handler` 负责实现该工具的输入/输出约定。
    pub fn register(self, name: impl Into<String>, handler: ToolHandler) -> Self {
        {
            let mut h = self
                .handlers
                .lock()
                .expect("mock tool invoker lock poisoned");
            h.insert(name.into(), handler);
        }
        self
    }

    /// 转成 `RhaiToolInvoker`（可注入 Rhai 标准库注册器）。
    pub fn to_invoker(&self) -> RhaiToolInvoker {
        let handlers = self.handlers.clone();
        Arc::new(
            move |tool_name: &str, args: Value| -> Result<Value, String> {
                let h = handlers
                    .lock()
                    .map_err(|_| "mock tool invoker lock poisoned".to_string())?;
                let Some(handler) = h.get(tool_name) else {
                    return Err(format!("tool not found: {}", tool_name));
                };
                (handler)(args)
            },
        )
    }
}

/// 生成一个尽量不冲突的临时目录路径（不负责创建）。
pub fn unique_temp_dir(prefix: &str) -> PathBuf {
    let mut base = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    base.push(format!("agentkit-{}-{}", prefix, nanos));
    base
}

/// 在 `skills_root/<skill_name>/` 下写入一个最小可加载的 Rhai skill：
/// - `meta.yaml`
/// - `SKILL.rhai`
/// - （可选）`SKILL.md`
///
/// 返回：skill 目录路径。
pub async fn write_skill_rhai(
    skills_root: &Path,
    skill_name: &str,
    meta_yaml: &str,
    script: &str,
    skill_md: Option<&str>,
) -> Result<PathBuf, SkillError> {
    let skill_dir = skills_root.join(skill_name);
    fs::create_dir_all(&skill_dir)
        .await
        .map_err(|e| SkillError::Message(format!("create skill dir failed: {}", e)))?;

    fs::write(skill_dir.join("meta.yaml"), meta_yaml)
        .await
        .map_err(|e| SkillError::Message(format!("write meta.yaml failed: {}", e)))?;

    fs::write(skill_dir.join("SKILL.rhai"), script)
        .await
        .map_err(|e| SkillError::Message(format!("write SKILL.rhai failed: {}", e)))?;

    if let Some(md) = skill_md {
        fs::write(skill_dir.join("SKILL.md"), md)
            .await
            .map_err(|e| SkillError::Message(format!("write SKILL.md failed: {}", e)))?;
    }

    Ok(skill_dir)
}

/// 使用 mock 工具集加载 skills 目录。
///
/// 说明：
/// - 内部会用 `rhai_stdlib_registrar` 注入标准库（`call_tool/is_error/json_*` 等）。
/// - 因此测试代码不需要自定义 registrar，即可执行 blockcell 风格脚本。
pub async fn load_skills_with_mock_tools(
    skills_root: &Path,
    mock: MockToolInvoker,
) -> Result<super::SkillRegistry, SkillError> {
    let registrar = rhai_stdlib_registrar(mock.to_invoker());
    load_skills_from_dir_with_rhai(skills_root, Some(registrar)).await
}

/// 常用返回值包装：统一输出 `{ success: true, output: ... }`。
pub fn ok(v: Value) -> Value {
    json!({"success": true, "output": v})
}
