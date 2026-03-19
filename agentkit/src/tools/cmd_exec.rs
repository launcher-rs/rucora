use agentkit_core::{
    error::ToolError,
    tool::{Tool, ToolCategory},
};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, info, warn};

use super::shell::{MAX_OUTPUT_BYTES, SHELL_TIMEOUT_SECS, execute_shell_command, truncate_output};

pub struct CmdExecTool {
    pub allowed_prefixes: &'static [&'static str],
}

impl CmdExecTool {
    pub fn new() -> Self {
        Self {
            allowed_prefixes: &["curl", "curl.exe"],
        }
    }

    fn validate_command(&self, cmd: &str) -> Result<(), ToolError> {
        let t = cmd.trim();
        let prefix_ok = self
            .allowed_prefixes
            .iter()
            .any(|p| t.starts_with(p) || t.starts_with(&format!("{} ", p)));

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
    fn name(&self) -> &str {
        "cmd_exec"
    }

    fn description(&self) -> Option<&str> {
        Some("执行受限的命令行（当前仅允许 curl）")
    }

    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::System]
    }

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
            execute_shell_command(command, &[]),
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
                Err(ToolError::Message(format!("命令执行失败: {}", e)))
            }
            Err(_) => {
                warn!(
                    tool.name = "cmd_exec",
                    cmd.timeout_secs = timeout_secs,
                    "cmd_exec.timeout"
                );
                Err(ToolError::Message(format!(
                    "命令执行超时（超过 {} 秒）",
                    timeout_secs
                )))
            }
        }
    }
}
