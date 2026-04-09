//! `cmd_exec` 工具实现。
//!
//! 该工具用于执行**受限**的命令行指令。
//!
//! 设计目标：
//! - 允许 Agent 在可控范围内调用外部命令（当前默认仅允许 `curl`）
//! - 通过白名单 + 禁止 shell 操作符来降低注入与破坏性风险
//! - 对输出进行截断，避免返回内容过大
//!
use agentkit_core::{
    error::ToolError,
    tool::{Tool, ToolCategory},
};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, info, warn};

use crate::shell::{MAX_OUTPUT_BYTES, SHELL_TIMEOUT_SECS, execute_shell_command, truncate_output};

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
                "command": {"type": "string", "description": "要执行的一整行命令（当前仅允许 curl）"},
                "timeout": {"type": "integer", "description": "可选，超时时间（秒）"}
            },
            "required": ["command"]
        })
    }

    /// 执行工具。
    ///
    /// 行为：
    /// - 先进行 `validate_command` 安全校验
    /// - 通过 `execute_shell_command` 执行命令，并应用超时
    /// - 返回 stdout/stderr/exit_code 等字段，并对输出做截断
    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let command = input
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Message("缺少必需的 'command' 字段".to_string()))?;

        self.validate_command(command)?;

        let timeout_secs = input
            .get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(SHELL_TIMEOUT_SECS);

        info!(
            tool.name = "cmd_exec",
            cmd.timeout_secs = timeout_secs,
            "cmd_exec.start"
        );
        debug!(tool.name = "cmd_exec", cmd.command = %command, "cmd_exec.command");

        let start = std::time::Instant::now();
        let result = timeout(
            Duration::from_secs(timeout_secs),
            execute_shell_command(command, &[], None),
        )
        .await;

        match result {
            Ok(Ok(output)) => {
                let stdout = truncate_output(String::from_utf8_lossy(&output.stdout).to_string());
                let stderr = truncate_output(String::from_utf8_lossy(&output.stderr).to_string());

                let elapsed_ms = start.elapsed().as_millis() as u64;
                let exit_code = output.status.code().unwrap_or(-1);
                let success = output.status.success();

                info!(
                    tool.name = "cmd_exec",
                    cmd.exit_code = exit_code,
                    cmd.success = success,
                    cmd.stdout_len = stdout.len().min(MAX_OUTPUT_BYTES),
                    cmd.stderr_len = stderr.len().min(MAX_OUTPUT_BYTES),
                    cmd.elapsed_ms = elapsed_ms,
                    "cmd_exec.done"
                );

                Ok(json!({
                    "command": command,
                    "stdout": stdout,
                    "stderr": stderr,
                    "exit_code": exit_code,
                    "success": success
                }))
            }
            Ok(Err(e)) => {
                warn!(tool.name = "cmd_exec", error = %e, "cmd_exec.failed");
                Err(ToolError::Message(format!("命令执行失败: {e}")))
            }
            Err(_) => {
                warn!(
                    tool.name = "cmd_exec",
                    cmd.timeout_secs = timeout_secs,
                    "cmd_exec.timeout"
                );
                Err(ToolError::Message(format!(
                    "命令执行超时（超过 {timeout_secs} 秒）"
                )))
            }
        }
    }
}
