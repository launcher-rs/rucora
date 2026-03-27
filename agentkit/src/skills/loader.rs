//! Skill 加载器和执行器模块

use agentkit_core::skill::{SkillDefinition, SkillResult};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde_json::Value;
use tracing::{info, warn, debug};

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
            .map_err(|e| SkillLoadError::IoError(format!("读取目录失败：{}", e)))?;
        
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
    
    pub async fn load_skill(&self, skill_dir: &Path) -> Result<SkillDefinition, SkillLoadError> {
        let md_path = skill_dir.join("SKILL.md");
        if !md_path.exists() {
            return Err(SkillLoadError::NotFound(format!("SKILL.md 不存在：{:?}", md_path)));
        }
        
        let content = std::fs::read_to_string(&md_path)
            .map_err(|e| SkillLoadError::IoError(format!("读取 SKILL.md 失败：{}", e)))?;
        
        let definition = parse_skill_md(&content)?;
        
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
        self.skills.values().map(|s| s.to_tool_description()).collect()
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

fn parse_skill_md(content: &str) -> Result<SkillDefinition, SkillLoadError> {
    let frontmatter = extract_frontmatter(content)?;
    
    let definition: SkillDefinition = serde_yaml::from_str(&frontmatter)
        .map_err(|e| SkillLoadError::ParseError(format!("解析 YAML 失败：{}", e)))?;
    
    if definition.name.is_empty() {
        return Err(SkillLoadError::ParseError("缺少必需字段：name".to_string()));
    }
    
    if definition.description.is_empty() {
        return Err(SkillLoadError::ParseError("缺少必需字段：description".to_string()));
    }
    
    Ok(definition)
}

fn extract_frontmatter(content: &str) -> Result<String, SkillLoadError> {
    let mut lines = content.lines();
    
    match lines.next() {
        Some("---") => {},
        _ => return Err(SkillLoadError::ParseError("SKILL.md 必须以 --- 开始".to_string())),
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
                self.execute_python(script_path, input, definition.timeout).await
            }
            SkillImplementation::JavaScript(_) => {
                self.execute_javascript(script_path, input, definition.timeout).await
            }
            SkillImplementation::Shell(_) => {
                self.execute_shell(script_path, input, definition.timeout).await
            }
            SkillImplementation::Rhai(_) => {
                Err(SkillExecuteError::NotFound("Rhai 执行需要 rhai-skills feature".to_string()))
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
        use tokio::process::Command;
        use tokio::time::timeout as tokio_timeout;
        use tokio::io::AsyncWriteExt;
        
        let start = std::time::Instant::now();
        
        let mut child = Command::new("python3")
            .arg(script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(&self.work_dir)
            .spawn()
            .map_err(|e| SkillExecuteError::IoError(format!("启动 Python 失败：{}", e)))?;
        
        if let Some(mut stdin) = child.stdin.take() {
            let input_str = input.to_string();
            stdin.write_all(input_str.as_bytes()).await
                .map_err(|e| SkillExecuteError::IoError(format!("写入输入失败：{}", e)))?;
        }
        
        let output = tokio_timeout(
            std::time::Duration::from_secs(timeout),
            child.wait_with_output()
        )
        .await
        .map_err(|_| SkillExecuteError::Timeout(timeout))?
        .map_err(|e| SkillExecuteError::IoError(format!("等待进程失败：{}", e)))?;
        
        let execution_time_ms = start.elapsed().as_millis() as u64;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let result: SkillResult = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| SkillResult::error(format!("输出格式错误：{}", stdout)));
        
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
        use tokio::process::Command;
        use tokio::time::timeout as tokio_timeout;
        use tokio::io::AsyncWriteExt;
        
        let start = std::time::Instant::now();
        
        let mut child = Command::new("node")
            .arg(script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(&self.work_dir)
            .spawn()
            .map_err(|e| SkillExecuteError::IoError(format!("启动 Node.js 失败：{}", e)))?;
        
        if let Some(mut stdin) = child.stdin.take() {
            let input_str = input.to_string();
            stdin.write_all(input_str.as_bytes()).await
                .map_err(|e| SkillExecuteError::IoError(format!("写入输入失败：{}", e)))?;
        }
        
        let output = tokio_timeout(
            std::time::Duration::from_secs(timeout),
            child.wait_with_output()
        )
        .await
        .map_err(|_| SkillExecuteError::Timeout(timeout))?
        .map_err(|e| SkillExecuteError::IoError(format!("等待进程失败：{}", e)))?;
        
        let execution_time_ms = start.elapsed().as_millis() as u64;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let result: SkillResult = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| SkillResult::error(format!("输出格式错误：{}", stdout)));
        
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
        use tokio::process::Command;
        use tokio::time::timeout as tokio_timeout;
        use tokio::io::AsyncWriteExt;
        
        let start = std::time::Instant::now();
        
        let mut child = Command::new("bash")
            .arg(script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(&self.work_dir)
            .spawn()
            .map_err(|e| SkillExecuteError::IoError(format!("启动 Bash 失败：{}", e)))?;
        
        if let Some(mut stdin) = child.stdin.take() {
            let input_str = input.to_string();
            stdin.write_all(input_str.as_bytes()).await
                .map_err(|e| SkillExecuteError::IoError(format!("写入输入失败：{}", e)))?;
        }
        
        let output = tokio_timeout(
            std::time::Duration::from_secs(timeout),
            child.wait_with_output()
        )
        .await
        .map_err(|_| SkillExecuteError::Timeout(timeout))?
        .map_err(|e| SkillExecuteError::IoError(format!("等待进程失败：{}", e)))?;
        
        let execution_time_ms = start.elapsed().as_millis() as u64;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let result: SkillResult = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| SkillResult::error(format!("输出格式错误：{}", stdout)));
        
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
