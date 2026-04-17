//! 受限命令执行工具
//!
//! 该工具用于执行**受限**的命令行指令。
//!
//! 设计目标：
//! - 允许 Agent 在可控范围内调用外部命令（当前默认仅允许 `curl`）
//! - 通过白名单 + 禁止 shell 操作符来降低注入与破坏性风险
//! - 对输出进行截断，避免返回内容过大

use agentkit_core::{
    error::ToolError,
    tool::{Tool, ToolCategory},
};
use async_trait::async_trait;
use serde_json::{Value, json};

use super::shell::{execute_shell_command, SHELL_TIMEOUT_SECS};

/// 受限命令执行工具。
///
/// 当前实现默认仅允许执行 `curl`（包含 `curl.exe`），并禁止常见 shell 操作符。
pub struct CmdExecTool {
    /// 允许的命令前缀白名单。
    ///
    /// 只要输入命令行以任一前缀开头（或 `"{prefix} "` 开头），即视为允许。
    pub allowed_prefixes: &'static [&'static str],
}

impl CmdExecTool {
    /// 创建一个默认的 `CmdExecTool`。
    ///
    /// 默认白名单为：`curl` / `curl.exe`。
    pub fn new() -> Self {
        Self {
            allowed_prefixes: &["curl", "curl.exe"],
        }
    }

    /// 校验命令行是否符合安全约束。
    ///
    /// - 必须以白名单前缀开头
    /// - 禁止管道/重定向/链式/多行等 shell 操作符
    fn validate_command(&self, cmd: &str) -> Result<(), ToolError> {
        let t = cmd.trim();
        let prefix_ok = self
            .allowed_prefixes
            .iter()
            .any(|p| t.starts_with(p) || t.starts_with(&format!("{p} ")));

        if !prefix_ok {
            return Err(ToolError::Message(
                "出于安全考虑，cmd_exec 目前仅允许执行 curl 命令".to_string(),
            ));
        }

        let forbidden = ["|", "&&", ";", ">", "<", "`", "$ (", "$(", "\n", "\r"];
        if forbidden.iter().any(|x| t.contains(x)) {
            return Err(ToolError::Message(
                "出于安全考虑，cmd_exec 禁止管道/重定向/链式/多行命令".to_string(),
            ));
        }

        Ok(())
    }
}

impl Default for CmdExecTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for CmdExecTool {
    /// 工具名称（用于让模型在 tool_call 中引用）。
    fn name(&self) -> &str {
        "cmd_exec"
    }

    /// 工具描述（会作为 tool/function 定义提供给模型）。
    fn description(&self) -> Option<&str> {
        Some("执行受限的命令行（当前仅允许 curl）")
    }

    /// 工具分类。
    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::System]
    }

    /// 输入 schema（JSON Schema）。
    ///
    /// - `command`：要执行的一整行命令
    /// - `timeout`：可选超时时间（秒）
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "要执行的命令行（仅允许以 curl 开头）"
                },
                "timeout": {
                    "type": "integer",
                    "description": "超时时间（秒），默认 60 秒",
                    "default": 60
                }
            },
            "required": ["command"]
        })
    }

    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let command = input
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Message("缺少必需的 'command' 字段".to_string()))?;

        let timeout_secs = input
            .get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(SHELL_TIMEOUT_SECS);

        // 校验命令
        self.validate_command(command)?;

        // 执行命令
        let result = execute_shell_command(command, timeout_secs, None).await?;

        Ok(json!({
            "command": command,
            "stdout": result.stdout,
            "stderr": result.stderr,
            "exit_code": result.exit_code,
            "success": result.exit_code == 0,
            "truncated": result.truncated
        }))
    }
}
