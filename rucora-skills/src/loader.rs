//! Skill 加载器和执行器模块

use rucora_core::skill::{SkillDefinition, SkillResult};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

fn parse_skill_stdout(stdout: &str) -> SkillResult {
    // 1) Try strict SkillResult
    if let Ok(mut r) = serde_json::from_str::<SkillResult>(stdout) {
        // Backward/loose compatibility:
        // If a script prints {success:true, city:..., weather:...} without `data`,
        // serde will ignore unknown fields and we end up with data=None.
        // In that case, wrap the remaining fields into data.
        if r.success
            && r.data.is_none()
            && let Ok(v) = serde_json::from_str::<Value>(stdout)
            && let Some(obj) = v.as_object()
        {
            let mut data_obj = serde_json::Map::new();
            for (k, val) in obj {
                if k == "success" || k == "error" || k == "execution_time_ms" {
                    continue;
                }
                data_obj.insert(k.clone(), val.clone());
            }
            if !data_obj.is_empty() {
                r.data = Some(Value::Object(data_obj));
            }
        }
        return r;
    }

    // 2) Try generic JSON
    match serde_json::from_str::<Value>(stdout) {
        Ok(v) => {
            if v.get("success").and_then(|x| x.as_bool()).is_some() {
                // Has `success` but doesn't match SkillResult schema
                let success = v.get("success").and_then(|x| x.as_bool()).unwrap_or(false);
                if success {
                    SkillResult::success(v)
                } else {
                    let msg = v
                        .get("error")
                        .and_then(|x| x.as_str())
                        .unwrap_or("Skill 执行失败");
                    SkillResult::error(msg)
                }
            } else {
                // No `success` field: treat whole JSON as successful payload
                SkillResult::success(v)
            }
        }
        Err(_) => SkillResult::error(format!("输出格式错误：{stdout}")),
    }
}

/// Skill 实现类型
#[derive(Debug, Clone, PartialEq)]
pub enum SkillImplementation {
    Python(PathBuf),
    JavaScript(PathBuf),
    Shell(PathBuf),
    Rhai(PathBuf),
    Unknown,
}

/// Skill 加载器
pub struct SkillLoader {
    base_dir: PathBuf,
    skills: HashMap<String, SkillDefinition>,
}

impl SkillLoader {
    pub fn new(base_dir: impl AsRef<Path>) -> Self {
        Self {
            base_dir: base_dir.as_ref().to_path_buf(),
            skills: HashMap::new(),
        }
    }

    pub async fn load_from_dir(&mut self) -> Result<Vec<SkillDefinition>, SkillLoadError> {
        let mut skills = Vec::new();

        let entries = std::fs::read_dir(&self.base_dir)
            .map_err(|e| SkillLoadError::IoError(format!("读取目录失败：{e}")))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            match self.load_skill(&path).await {
                Ok(skill) => {
                    info!("加载技能：{} (v{})", skill.name, skill.version);
                    skills.push(skill.clone());
                    self.skills.insert(skill.name.clone(), skill);
                }
                Err(e) => {
                    warn!("跳过技能 {:?}: {}", path, e);
                }
            }
        }

        info!("成功加载 {} 个技能", skills.len());
        Ok(skills)
    }

    #[allow(clippy::unused_async)]
    pub async fn load_skill(&self, skill_dir: &Path) -> Result<SkillDefinition, SkillLoadError> {
        // 配置文件优先级：skill.yaml > skill.toml > skill.json > SKILL.md
        let definition = if skill_dir.join("skill.yaml").exists() {
            // 读取 skill.yaml
            let content = std::fs::read_to_string(skill_dir.join("skill.yaml"))
                .map_err(|e| SkillLoadError::IoError(format!("读取 skill.yaml 失败：{e}")))?;
            serde_yaml::from_str(&content)
                .map_err(|e| SkillLoadError::ParseError(format!("解析 skill.yaml 失败：{e}")))?
        } else if skill_dir.join("skill.toml").exists() {
            // 读取 skill.toml
            let content = std::fs::read_to_string(skill_dir.join("skill.toml"))
                .map_err(|e| SkillLoadError::IoError(format!("读取 skill.toml 失败：{e}")))?;
            toml::from_str(&content)
                .map_err(|e| SkillLoadError::ParseError(format!("解析 skill.toml 失败：{e}")))?
        } else if skill_dir.join("skill.json").exists() {
            // 读取 skill.json
            let content = std::fs::read_to_string(skill_dir.join("skill.json"))
                .map_err(|e| SkillLoadError::IoError(format!("读取 skill.json 失败：{e}")))?;
            serde_json::from_str(&content)
                .map_err(|e| SkillLoadError::ParseError(format!("解析 skill.json 失败：{e}")))?
        } else {
            // 读取 SKILL.md
            let md_path = skill_dir.join("SKILL.md");
            if !md_path.exists() {
                return Err(SkillLoadError::NotFound(format!(
                    "SKILL.md 不存在：{md_path:?}"
                )));
            }

            let content = std::fs::read_to_string(&md_path)
                .map_err(|e| SkillLoadError::IoError(format!("读取 SKILL.md 失败：{e}")))?;

            parse_skill_md(&content)?
        };

        let implementation = detect_implementation(skill_dir);
        debug!("技能 {} 使用实现：{:?}", definition.name, implementation);

        Ok(definition)
    }

    pub fn get_skill(&self, name: &str) -> Option<&SkillDefinition> {
        self.skills.get(name)
    }

    pub fn get_all_skills(&self) -> Vec<&SkillDefinition> {
        self.skills.values().collect()
    }

    pub fn to_tool_descriptions(&self) -> Vec<Value> {
        self.skills
            .values()
            .map(|s| s.to_tool_description())
            .collect()
    }
}

fn parse_skill_md(content: &str) -> Result<SkillDefinition, SkillLoadError> {
    let frontmatter = extract_frontmatter(content)?;

    let definition: SkillDefinition = serde_yaml::from_str(&frontmatter)
        .map_err(|e| SkillLoadError::ParseError(format!("解析 YAML 失败：{e}")))?;

    if definition.name.is_empty() {
        return Err(SkillLoadError::ParseError("缺少必需字段：name".to_string()));
    }

    if definition.description.is_empty() {
        return Err(SkillLoadError::ParseError(
            "缺少必需字段：description".to_string(),
        ));
    }

    Ok(definition)
}

fn extract_frontmatter(content: &str) -> Result<String, SkillLoadError> {
    let mut lines = content.lines();

    match lines.next() {
        Some("---") => {}
        _ => {
            return Err(SkillLoadError::ParseError(
                "SKILL.md 必须以 --- 开始".to_string(),
            ));
        }
    }

    let mut frontmatter = String::new();
    for line in lines {
        if line.trim() == "---" {
            break;
        }
        frontmatter.push_str(line);
        frontmatter.push('\n');
    }

    if frontmatter.trim().is_empty() {
        return Err(SkillLoadError::ParseError("Frontmatter 为空".to_string()));
    }

    Ok(frontmatter)
}

fn detect_implementation(skill_dir: &Path) -> SkillImplementation {
    let py_path = skill_dir.join("SKILL.py");
    if py_path.exists() {
        return SkillImplementation::Python(py_path);
    }

    let js_path = skill_dir.join("SKILL.js");
    if js_path.exists() {
        return SkillImplementation::JavaScript(js_path);
    }

    let sh_path = skill_dir.join("SKILL.sh");
    if sh_path.exists() {
        return SkillImplementation::Shell(sh_path);
    }

    let rhai_path = skill_dir.join("SKILL.rhai");
    if rhai_path.exists() {
        return SkillImplementation::Rhai(rhai_path);
    }

    SkillImplementation::Unknown
}

/// Skill 执行器
pub struct SkillExecutor {
    work_dir: PathBuf,
}

impl SkillExecutor {
    pub fn new() -> Self {
        Self {
            work_dir: std::env::current_dir().unwrap_or_default(),
        }
    }

    pub fn with_work_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.work_dir = dir.into();
        self
    }

    pub async fn execute(
        &self,
        definition: &SkillDefinition,
        script_path: &Path,
        input: &Value,
    ) -> Result<SkillResult, SkillExecuteError> {
        if let Err(e) = definition.validate_input(input) {
            return Ok(SkillResult::error(e));
        }

        let implementation = detect_implementation(script_path.parent().unwrap());

        match implementation {
            SkillImplementation::Python(_) => {
                self.execute_python(script_path, input, definition.timeout)
                    .await
            }
            SkillImplementation::JavaScript(_) => {
                self.execute_javascript(script_path, input, definition.timeout)
                    .await
            }
            SkillImplementation::Shell(_) => {
                self.execute_shell(script_path, input, definition.timeout)
                    .await
            }
            SkillImplementation::Rhai(_) => Err(SkillExecuteError::NotFound(
                "Rhai 执行需要 rhai-skills feature".to_string(),
            )),
            SkillImplementation::Unknown => {
                Err(SkillExecuteError::NotFound("未找到脚本实现".to_string()))
            }
        }
    }

    async fn execute_python(
        &self,
        script_path: &Path,
        input: &Value,
        timeout: u64,
    ) -> Result<SkillResult, SkillExecuteError> {
        use std::process::Stdio;
        use tokio::io::AsyncWriteExt;
        use tokio::process::Command;
        use tokio::time::timeout as tokio_timeout;

        let start = std::time::Instant::now();

        // 尝试 python 或 python3
        let python_cmd = if cfg!(windows) { "python" } else { "python3" };

        debug!(
            skill.impl = "python",
            skill.script = %script_path.display(),
            skill.timeout_s = timeout,
            skill.work_dir = %self.work_dir.display(),
            "skill.exec.start"
        );

        let mut child = Command::new(python_cmd)
            .arg("-X")
            .arg("utf8")
            .arg(script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(&self.work_dir)
            .env("PYTHONIOENCODING", "utf-8")
            .env("PYTHONUTF8", "1")
            .spawn()
            .map_err(|e| {
                SkillExecuteError::IoError(format!(
                    "启动 Python 失败：{e}，请确保 Python 已安装并添加到 PATH"
                ))
            })?;

        if let Some(mut stdin) = child.stdin.take() {
            let input_str = input.to_string();
            stdin
                .write_all(input_str.as_bytes())
                .await
                .map_err(|e| SkillExecuteError::IoError(format!("写入输入失败：{e}")))?;
        }

        let output = tokio_timeout(
            std::time::Duration::from_secs(timeout),
            child.wait_with_output(),
        )
        .await
        .map_err(|_| SkillExecuteError::Timeout(timeout))?
        .map_err(|e| SkillExecuteError::IoError(format!("等待进程失败：{e}")))?;

        let execution_time_ms = start.elapsed().as_millis() as u64;

        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.trim().is_empty() {
            let preview: String = stderr.chars().take(400).collect();
            warn!(
                skill.impl = "python",
                skill.script = %script_path.display(),
                skill.stderr_preview = %preview,
                "skill.exec.stderr"
            );
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        debug!(
            skill.impl = "python",
            skill.script = %script_path.display(),
            skill.stdout_len = stdout.len(),
            skill.elapsed_ms = execution_time_ms,
            "skill.exec.stdout"
        );

        // 如果输出为空，返回错误
        if stdout.trim().is_empty() {
            return Ok(SkillResult::error("脚本输出为空"));
        }

        let result = parse_skill_stdout(&stdout);

        Ok(SkillResult {
            execution_time_ms: Some(execution_time_ms),
            ..result
        })
    }

    async fn execute_javascript(
        &self,
        script_path: &Path,
        input: &Value,
        timeout: u64,
    ) -> Result<SkillResult, SkillExecuteError> {
        use std::process::Stdio;
        use tokio::io::AsyncWriteExt;
        use tokio::process::Command;
        use tokio::time::timeout as tokio_timeout;

        let start = std::time::Instant::now();

        // 尝试 node 或 nodejs
        let node_cmd = if cfg!(windows) { "node" } else { "nodejs" };

        debug!(
            skill.impl = "javascript",
            skill.script = %script_path.display(),
            skill.timeout_s = timeout,
            skill.work_dir = %self.work_dir.display(),
            "skill.exec.start"
        );

        let mut child = Command::new(node_cmd)
            .arg(script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(&self.work_dir)
            .spawn()
            .map_err(|e| SkillExecuteError::IoError(format!("启动 Node.js 失败：{e}")))?;

        if let Some(mut stdin) = child.stdin.take() {
            let input_str = input.to_string();
            stdin
                .write_all(input_str.as_bytes())
                .await
                .map_err(|e| SkillExecuteError::IoError(format!("写入输入失败：{e}")))?;
        }

        let output = tokio_timeout(
            std::time::Duration::from_secs(timeout),
            child.wait_with_output(),
        )
        .await
        .map_err(|_| SkillExecuteError::Timeout(timeout))?
        .map_err(|e| SkillExecuteError::IoError(format!("等待进程失败：{e}")))?;

        let execution_time_ms = start.elapsed().as_millis() as u64;

        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.trim().is_empty() {
            let preview: String = stderr.chars().take(400).collect();
            warn!(
                skill.impl = "javascript",
                skill.script = %script_path.display(),
                skill.stderr_preview = %preview,
                "skill.exec.stderr"
            );
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        debug!(
            skill.impl = "javascript",
            skill.script = %script_path.display(),
            skill.stdout_len = stdout.len(),
            skill.elapsed_ms = execution_time_ms,
            "skill.exec.stdout"
        );

        if stdout.trim().is_empty() {
            return Ok(SkillResult::error("脚本输出为空"));
        }

        let result = parse_skill_stdout(&stdout);

        Ok(SkillResult {
            execution_time_ms: Some(execution_time_ms),
            ..result
        })
    }

    async fn execute_shell(
        &self,
        script_path: &Path,
        input: &Value,
        timeout: u64,
    ) -> Result<SkillResult, SkillExecuteError> {
        use std::process::Stdio;
        use tokio::io::AsyncWriteExt;
        use tokio::process::Command;
        use tokio::time::timeout as tokio_timeout;

        let start = std::time::Instant::now();

        debug!(
            skill.impl = "shell",
            skill.script = %script_path.display(),
            skill.timeout_s = timeout,
            skill.work_dir = %self.work_dir.display(),
            "skill.exec.start"
        );

        let mut child = Command::new("bash")
            .arg(script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(&self.work_dir)
            .spawn()
            .map_err(|e| SkillExecuteError::IoError(format!("启动 Bash 失败：{e}")))?;

        if let Some(mut stdin) = child.stdin.take() {
            let input_str = input.to_string();
            stdin
                .write_all(input_str.as_bytes())
                .await
                .map_err(|e| SkillExecuteError::IoError(format!("写入输入失败：{e}")))?;
        }

        let output = tokio_timeout(
            std::time::Duration::from_secs(timeout),
            child.wait_with_output(),
        )
        .await
        .map_err(|_| SkillExecuteError::Timeout(timeout))?
        .map_err(|e| SkillExecuteError::IoError(format!("等待进程失败：{e}")))?;

        let execution_time_ms = start.elapsed().as_millis() as u64;

        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.trim().is_empty() {
            let preview: String = stderr.chars().take(400).collect();
            warn!(
                skill.impl = "shell",
                skill.script = %script_path.display(),
                skill.stderr_preview = %preview,
                "skill.exec.stderr"
            );
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        debug!(
            skill.impl = "shell",
            skill.script = %script_path.display(),
            skill.stdout_len = stdout.len(),
            skill.elapsed_ms = execution_time_ms,
            "skill.exec.stdout"
        );

        // 如果输出为空，返回错误
        if stdout.trim().is_empty() {
            return Ok(SkillResult::error("脚本输出为空"));
        }

        let result = parse_skill_stdout(&stdout);

        Ok(SkillResult {
            execution_time_ms: Some(execution_time_ms),
            ..result
        })
    }
}

impl Default for SkillExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SkillLoadError {
    #[error("IO 错误：{0}")]
    IoError(String),
    #[error("解析错误：{0}")]
    ParseError(String),
    #[error("未找到：{0}")]
    NotFound(String),
}

#[derive(Debug, thiserror::Error)]
pub enum SkillExecuteError {
    #[error("IO 错误：{0}")]
    IoError(String),
    #[error("超时：{0}秒")]
    Timeout(u64),
    #[error("未找到：{0}")]
    NotFound(String),
    #[error("验证失败：{0}")]
    ValidationError(String),
}
