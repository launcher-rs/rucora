//! Skill 加载器和执行器模块

use crate::config::SkillConfig;
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
        let mut definition = load_skill_definition(skill_dir)?;
        definition.location = Some(skill_dir.to_path_buf());

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

fn load_skill_definition(skill_dir: &Path) -> Result<SkillDefinition, SkillLoadError> {
    let yaml_path = skill_dir.join("skill.yaml");
    if yaml_path.exists() {
        match parse_skill_config_file(&yaml_path) {
            Ok(definition) => return Ok(definition),
            Err(e) => {
                warn!(
                    "读取 skill.yaml 失败，尝试读取 Markdown 头部信息 {:?}: {}",
                    yaml_path, e
                );
                if let Ok(definition) = load_skill_md(skill_dir) {
                    return Ok(definition);
                }
                return Err(e);
            }
        }
    }

    if let Ok(definition) = load_skill_md(skill_dir) {
        return Ok(definition);
    }

    for config_file in ["skill.toml", "skill.json"] {
        let path = skill_dir.join(config_file);
        if path.exists() {
            return parse_skill_config_file(&path);
        }
    }

    Err(SkillLoadError::NotFound(format!(
        "未找到 skill.yaml、SKILL.md、skill.md、skill.toml 或 skill.json：{skill_dir:?}"
    )))
}

fn parse_skill_config_file(path: &Path) -> Result<SkillDefinition, SkillLoadError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| SkillLoadError::IoError(format!("读取 {:?} 失败：{e}", path.file_name())))?;

    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "yaml" | "yml" => parse_yaml_skill_definition(&content)
            .map_err(|e| SkillLoadError::ParseError(format!("解析 {path:?} 失败：{e}"))),
        "toml" => parse_toml_skill_definition(&content)
            .map_err(|e| SkillLoadError::ParseError(format!("解析 {path:?} 失败：{e}"))),
        "json" => parse_json_skill_definition(&content)
            .map_err(|e| SkillLoadError::ParseError(format!("解析 {path:?} 失败：{e}"))),
        ext => Err(SkillLoadError::ParseError(format!(
            "不支持的配置格式：{ext}"
        ))),
    }
}

fn parse_yaml_skill_definition(content: &str) -> Result<SkillDefinition, serde_yaml::Error> {
    serde_yaml::from_str(content).or_else(|_| {
        let config: SkillConfig = serde_yaml::from_str(content)?;
        Ok(skill_config_to_definition(config))
    })
}

fn parse_toml_skill_definition(content: &str) -> Result<SkillDefinition, toml::de::Error> {
    toml::from_str(content).or_else(|_| {
        let config: SkillConfig = toml::from_str(content)?;
        Ok(skill_config_to_definition(config))
    })
}

fn parse_json_skill_definition(content: &str) -> Result<SkillDefinition, serde_json::Error> {
    serde_json::from_str(content).or_else(|_| {
        let config: SkillConfig = serde_json::from_str(content)?;
        Ok(skill_config_to_definition(config))
    })
}

fn skill_config_to_definition(config: SkillConfig) -> SkillDefinition {
    let metadata = if config.metadata.is_empty() {
        None
    } else {
        serde_json::to_value(config.metadata).ok()
    };

    SkillDefinition {
        name: config.skill.name,
        description: config.skill.description,
        version: config.skill.version,
        author: config.skill.author,
        tags: config.skill.tags,
        timeout: config.execution.map_or(30, |execution| execution.timeout),
        input_schema: config.input_schema.unwrap_or(Value::Null),
        output_schema: config.output_schema.unwrap_or(Value::Null),
        homepage: None,
        metadata,
        location: None,
    }
}

fn load_skill_md(skill_dir: &Path) -> Result<SkillDefinition, SkillLoadError> {
    let md_path = find_skill_md(skill_dir).ok_or_else(|| {
        SkillLoadError::NotFound(format!("SKILL.md 或 skill.md 不存在：{skill_dir:?}"))
    })?;
    let content = std::fs::read_to_string(&md_path)
        .map_err(|e| SkillLoadError::IoError(format!("读取 {md_path:?} 失败：{e}")))?;

    parse_skill_md(&content)
}

fn find_skill_md(skill_dir: &Path) -> Option<PathBuf> {
    ["SKILL.md", "skill.md"]
        .into_iter()
        .map(|file_name| skill_dir.join(file_name))
        .find(|path| path.exists())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_skill_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("系统时间应晚于 UNIX_EPOCH")
            .as_nanos();
        std::env::temp_dir().join(format!("rucora-skills-{name}-{nanos}"))
    }

    #[test]
    fn load_definition_falls_back_to_lowercase_skill_md_when_yaml_fails() {
        let dir = temp_skill_dir("yaml-fallback");
        fs::create_dir_all(&dir).expect("应能创建临时 skill 目录");
        fs::write(dir.join("skill.yaml"), "name: [").expect("应能写入损坏的 skill.yaml");
        fs::write(
            dir.join("skill.md"),
            "---\nname: fallback-skill\ndescription: 来自 Markdown 头部\n---\n正文\n",
        )
        .expect("应能写入 skill.md");

        let definition = load_skill_definition(&dir).expect("应回退读取 Markdown 头部");

        assert_eq!(definition.name, "fallback-skill");
        assert_eq!(definition.description, "来自 Markdown 头部");

        fs::remove_dir_all(&dir).expect("应能清理临时 skill 目录");
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
