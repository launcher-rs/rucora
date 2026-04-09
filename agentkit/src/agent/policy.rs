//! 工具调用策略（Policy）模块
//!
//! 用于在执行工具前进行 allow/deny 安全检查。

use async_trait::async_trait;
use serde_json::Value;

use agentkit_core::error::ToolError;
use agentkit_core::tool::types::ToolCall;

#[derive(Debug, Clone)]
/// 单次工具调用的上下文信息。
///
/// 主要用于把 `ToolCall` 传递给策略（policy）做安全检查。
pub struct ToolCallContext {
    pub tool_call: ToolCall,
}

#[async_trait]
/// 工具调用策略（Policy）。
///
/// 用于在执行工具前进行 allow/deny 检查。
///
/// 返回 `Ok(())` 表示允许执行；返回 `Err(ToolError::PolicyDenied{..})`
/// 表示拒绝并携带规则与原因。
pub trait ToolPolicy: Send + Sync {
    async fn check(&self, ctx: &ToolCallContext) -> Result<(), ToolError>;
}

#[derive(Debug, Default, Clone)]
/// 允许所有工具调用的策略（不做任何拦截）。
pub struct AllowAllToolPolicy;

#[async_trait]
impl ToolPolicy for AllowAllToolPolicy {
    async fn check(&self, _ctx: &ToolCallContext) -> Result<(), ToolError> {
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
/// 命令执行类工具的 allow/deny 配置。
///
/// 该配置用于 `DefaultToolPolicy`：
/// - `allowed_commands` 非空时，仅允许列表内命令
/// - `denied_commands` 用于显式禁止（优先级更高）
pub struct CommandPolicyConfig {
    pub allowed_commands: Vec<String>,
    pub denied_commands: Vec<String>,
}

impl CommandPolicyConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn allow_command(mut self, cmd: impl Into<String>) -> Self {
        self.allowed_commands.push(cmd.into());
        self
    }

    pub fn deny_command(mut self, cmd: impl Into<String>) -> Self {
        self.denied_commands.push(cmd.into());
        self
    }
}

#[derive(Debug, Clone)]
/// 默认工具策略。
///
/// 目前主要针对两类“命令执行”工具：
/// - `shell`：包含命令与参数
/// - `cmd_exec`：直接执行命令
///
/// 默认会拦截危险命令，并阻止常见 shell 操作符（防止链式/重定向）。
pub struct DefaultToolPolicy {
    shell: CommandPolicyConfig,
    cmd_exec: CommandPolicyConfig,
}

impl Default for DefaultToolPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultToolPolicy {
    pub fn new() -> Self {
        Self {
            shell: CommandPolicyConfig::new(),
            cmd_exec: CommandPolicyConfig::new().allow_command("curl"),
        }
    }

    pub fn with_shell_config(mut self, cfg: CommandPolicyConfig) -> Self {
        self.shell = cfg;
        self
    }

    pub fn with_cmd_exec_config(mut self, cfg: CommandPolicyConfig) -> Self {
        self.cmd_exec = cfg;
        self
    }

    fn extract_command_line(tool_name: &str, input: &Value) -> Option<String> {
        match tool_name {
            "shell" => {
                let command = input.get("command")?.as_str()?.trim().to_string();
                let args = input
                    .get("args")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|x| x.as_str())
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                if args.is_empty() {
                    Some(command)
                } else {
                    Some(format!("{} {}", command, args.join(" ")))
                }
            }
            "cmd_exec" => Some(input.get("command")?.as_str()?.trim().to_string()),
            _ => None,
        }
    }

    fn first_token(command_line: &str) -> Option<String> {
        let t = command_line.trim();
        if t.is_empty() {
            return None;
        }
        let mut token = t
            .split_whitespace()
            .next()?
            .trim_matches('"')
            .trim_matches('\'');
        if token.ends_with(".exe") {
            token = token.trim_end_matches(".exe");
        }
        Some(token.to_ascii_lowercase())
    }

    fn is_dangerous_command(cmd: &str) -> bool {
        matches!(
            cmd,
            "rm" | "del"
                | "erase"
                | "rmdir"
                | "rd"
                | "format"
                | "mkfs"
                | "dd"
                | "shutdown"
                | "reboot"
                | "poweroff"
                | "reg"
                | "diskpart"
                | "bcdedit"
                | "sc"
                | "net"
        )
    }

    fn contains_shell_operators(command_line: &str) -> bool {
        let forbidden = ["|", "&&", ";", ">", "<", "`", "$(", "\n", "\r"];
        forbidden.iter().any(|x| command_line.contains(x))
    }

    fn check_command(cfg: &CommandPolicyConfig, command_line: &str) -> Result<(), ToolError> {
        if Self::contains_shell_operators(command_line) {
            return Err(ToolError::PolicyDenied {
                rule_id: "default.shell_operators".to_string(),
                reason: "command contains forbidden shell operators".to_string(),
            });
        }

        let cmd = Self::first_token(command_line).ok_or_else(|| ToolError::PolicyDenied {
            rule_id: "default.empty_command".to_string(),
            reason: "empty command".to_string(),
        })?;

        if cfg
            .denied_commands
            .iter()
            .any(|x| x.eq_ignore_ascii_case(&cmd))
        {
            return Err(ToolError::PolicyDenied {
                rule_id: "config.denied_command".to_string(),
                reason: format!("command '{}' is denied", cmd),
            });
        }

        if Self::is_dangerous_command(&cmd) {
            return Err(ToolError::PolicyDenied {
                rule_id: "default.dangerous_command".to_string(),
                reason: format!("dangerous command '{}' is blocked by default", cmd),
            });
        }

        if !cfg.allowed_commands.is_empty()
            && !cfg
                .allowed_commands
                .iter()
                .any(|x| x.eq_ignore_ascii_case(&cmd))
        {
            return Err(ToolError::PolicyDenied {
                rule_id: "config.not_allowed".to_string(),
                reason: format!("command '{}' is not in allowlist", cmd),
            });
        }

        Ok(())
    }
}

#[async_trait]
impl ToolPolicy for DefaultToolPolicy {
    async fn check(&self, ctx: &ToolCallContext) -> Result<(), ToolError> {
        let name = ctx.tool_call.name.as_str();
        let input = &ctx.tool_call.input;
        let Some(command_line) = Self::extract_command_line(name, input) else {
            return Ok(());
        };

        match name {
            "shell" => Self::check_command(&self.shell, &command_line),
            "cmd_exec" => Self::check_command(&self.cmd_exec, &command_line),
            _ => Ok(()),
        }
    }
}
