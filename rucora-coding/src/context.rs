//! Coding 上下文组装。
//!
//! AI 编程不能把整个项目塞进上下文——[`ContextAssembler`] 按"预算"动态
//! 决定哪些内容进入上下文：
//!
//! 1. **项目指令**（[`ProjectInstructions`]）：AGENTS.md / CLAUDE.md / README.md
//! 2. **仓库结构**（[`RepoMap`]）：目录树 + 文件体积标注（忽略构建产物）
//! 3. **聚焦文件**（预留）：当前打开文件、选中代码（由调用方传入）
//!
//! 输出格式固定为带 XML 标签的上下文块 + 任务，便于模型区分指令与任务：
//!
//! ```text
//! <project-context>
//! ## 项目指令
//! ...
//! ## 仓库结构
//! ...
//! </project-context>
//!
//! <task>...</task>
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use tracing::debug;

/// 默认项目指令预算（字符）。
pub const DEFAULT_INSTRUCTIONS_BUDGET: usize = 4000;

/// 默认仓库结构预算（字符）。
pub const DEFAULT_REPO_MAP_BUDGET: usize = 3000;

/// 默认目录遍历深度。
pub const DEFAULT_MAX_DEPTH: usize = 6;

/// 忽略的目录名（构建产物、IDE 配置等）。
const IGNORED_DIRS: &[&str] = &[
    ".git", "target", "node_modules", "dist", "build", "out", ".idea", ".vscode", "__pycache__",
    "vendor", "coverage", ".venv", "venv", "sessions",
];

/// 上下文配置。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContextConfig {
    /// 是否启用上下文注入（关闭时 `assemble` 原样返回任务文本）。
    pub enabled: bool,
    /// 项目指令预算（字符）。
    pub instructions_budget: usize,
    /// 仓库结构预算（字符）。
    pub repo_map_budget: usize,
    /// 目录遍历深度上限。
    pub max_depth: usize,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            instructions_budget: DEFAULT_INSTRUCTIONS_BUDGET,
            repo_map_budget: DEFAULT_REPO_MAP_BUDGET,
            max_depth: DEFAULT_MAX_DEPTH,
        }
    }
}

/// 粗略 token 估算（中英混合文本按约 3 字符/token 估计，仅作参考）。
pub fn estimate_tokens(text: &str) -> usize {
    text.chars().count() / 3
}

/// 仓库结构地图：目录树 + 文件体积标注，按预算截断。
pub struct RepoMap {
    workspace: PathBuf,
    max_depth: usize,
}

impl RepoMap {
    /// 创建仓库地图（基于工作区路径）。
    pub fn new(workspace: impl Into<PathBuf>) -> Self {
        Self {
            workspace: workspace.into(),
            max_depth: DEFAULT_MAX_DEPTH,
        }
    }

    /// 设置遍历深度上限。
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// 渲染目录树，截断到预算字符数。
    ///
    /// 规则：目录优先、字典序；忽略 [`IGNORED_DIRS`] 与超深度；每行标注
    /// 文件体积（`12.3 KB`）。预算截断时以 `…（已截断）` 收尾。
    pub fn render(&self, budget_chars: usize) -> String {
        let mut lines: Vec<String> = Vec::new();
        let mut used = 0usize;
        let root_name = self
            .workspace
            .file_name()
            .map_or_else(|| self.workspace.display().to_string(), |n| n.to_string_lossy().to_string());
        lines.push(format!("{root_name}/"));
        used += lines[0].len() + 1;
        self.walk(&self.workspace, "", 0, &mut lines, &mut used, budget_chars);

        let mut out = lines.join("\n");
        if out.chars().count() > budget_chars {
            // 极端情况：按字符边界硬截断（多字节安全）
            out = truncate_chars(&out, budget_chars);
        }
        out
    }

    /// 递归遍历目录并生成树行。
    fn walk(
        &self,
        dir: &Path,
        prefix: &str,
        depth: usize,
        lines: &mut Vec<String>,
        used: &mut usize,
        budget: usize,
    ) {
        if depth >= self.max_depth || *used > budget {
            return;
        }
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        let mut dirs: Vec<(PathBuf, String)> = Vec::new();
        let mut files: Vec<(PathBuf, String)> = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path();
            if path.is_dir() {
                if IGNORED_DIRS.contains(&name.as_str()) {
                    continue;
                }
                dirs.push((path, name));
            } else {
                files.push((path, name));
            }
        }
        dirs.sort_by(|a, b| a.1.cmp(&b.1));
        files.sort_by(|a, b| a.1.cmp(&b.1));

        for (path, name) in dirs.iter().chain(files.iter()) {
            if *used > budget {
                lines.push("…（已截断）".to_string());
                return;
            }
            let is_dir = path.is_dir();
            let size_note = if is_dir {
                String::new()
            } else {
                format!(" ({})", format_size(fs::metadata(path).map_or(0, |m| m.len())))
            };
            let last = is_last_entry(path, &dirs, &files);
            let connector = if last { "└── " } else { "├── " };
            let suffix = if is_dir { "/" } else { "" };
            let line = format!("{prefix}{connector}{name}{suffix}{size_note}");
            *used += line.len() + 1;
            lines.push(line);

            if is_dir {
                let child_prefix = if last {
                    format!("{prefix}    ")
                } else {
                    format!("{prefix}│   ")
                };
                self.walk(path, &child_prefix, depth + 1, lines, used, budget);
            }
        }
    }
}

/// 判断当前条目是否为同级最后一项（决定树形连接符）。
fn is_last_entry(path: &Path, dirs: &[(PathBuf, String)], files: &[(PathBuf, String)]) -> bool {
    // dirs 与 files 已按序拼接，序列的最后一个元素即为 last
    if let Some((last, _)) = files.last() {
        return last == path;
    }
    if let Some((last, _)) = dirs.last() {
        return last == path;
    }
    false
}

/// 字节数的人类可读表示。
fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// 项目指令加载器。
///
/// 按优先级查找工作区根目录下的 `AGENTS.md` → `CLAUDE.md` → `README.md`，
/// 找到第一个存在的文件并截断到预算。
pub struct ProjectInstructions;

impl ProjectInstructions {
    /// 指令文件候选（按优先级）。
    pub const CANDIDATES: &[&str] = &["AGENTS.md", "CLAUDE.md", "README.md"];

    /// 加载项目指令（截断到预算）。
    ///
    /// # 返回
    ///
    /// `(文件名, 内容)`；无任何候选文件时返回 `None`。
    pub fn load(workspace: &Path, budget_chars: usize) -> Option<(String, String)> {
        for candidate in Self::CANDIDATES {
            let path = workspace.join(candidate);
            if let Ok(content) = fs::read_to_string(&path) {
                let truncated = truncate_chars(&content, budget_chars);
                debug!(file = candidate, chars = truncated.chars().count(), "coding.context.instructions");
                return Some(((*candidate).to_string(), truncated));
            }
        }
        None
    }
}

/// 按字符边界截断（多字节安全），超出时追加省略标记。
fn truncate_chars(text: &str, budget: usize) -> String {
    if text.chars().count() <= budget {
        return text.to_string();
    }
    let mut out: String = text.chars().take(budget.saturating_sub(20)).collect();
    out.push_str("\n…（已截断）");
    out
}

/// 上下文组装器。
pub struct ContextAssembler {
    workspace: PathBuf,
    config: ContextConfig,
}

impl ContextAssembler {
    /// 创建组装器。
    pub fn new(workspace: impl Into<PathBuf>, config: ContextConfig) -> Self {
        Self {
            workspace: workspace.into(),
            config,
        }
    }

    /// 组装带上下文的输入文本。
    ///
    /// 关闭（`enabled: false`）时原样返回任务文本。
    pub fn assemble(&self, task: &str) -> String {
        if !self.config.enabled {
            return task.to_string();
        }

        let mut blocks: Vec<String> = Vec::new();

        if let Some((file, content)) =
            ProjectInstructions::load(&self.workspace, self.config.instructions_budget)
        {
            blocks.push(format!("## 项目指令（{file}）\n\n{content}"));
        }

        let repo_map = RepoMap::new(&self.workspace)
            .with_max_depth(self.config.max_depth)
            .render(self.config.repo_map_budget);
        blocks.push(format!("## 仓库结构\n\n{repo_map}"));

        format!(
            "<project-context>\n{}\n</project-context>\n\n<task>{task}</task>",
            blocks.join("\n\n")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_workspace(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "rucora-coding-ctx-{tag}-{}",
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&dir).expect("创建临时工作区");
        dir
    }

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("创建父目录");
        }
        fs::write(path, content).expect("写入文件");
    }

    #[test]
    fn test_repo_map_renders_tree_and_ignores_build_dirs() {
        let ws = temp_workspace("repomap");
        write(&ws.join("src/main.rs"), "fn main() {}\n");
        write(&ws.join("src/lib/util.rs"), "// util\n");
        write(&ws.join("README.md"), "# demo\n");
        write(&ws.join("target/debug/artifact.bin"), "binary");
        write(&ws.join(".git/config"), "[core]");

        let map = RepoMap::new(&ws).render(10_000);
        assert!(map.contains("src/"), "应包含 src 目录：{map}");
        assert!(map.contains("main.rs"), "应包含 main.rs：{map}");
        assert!(map.contains("util.rs"), "应包含嵌套文件：{map}");
        assert!(map.contains("README.md"), "应包含 README：{map}");
        assert!(!map.contains("target"), "应忽略 target：{map}");
        assert!(!map.contains(".git"), "应忽略 .git：{map}");
        assert!(map.contains("KB") || map.contains("B"), "应标注文件体积：{map}");
        fs::remove_dir_all(&ws).ok();
    }

    #[test]
    fn test_repo_map_budget_truncates() {
        let ws = temp_workspace("budget");
        for i in 0..80 {
            write(&ws.join(format!("dir{i:02}/file{i:02}.rs")), "x");
        }
        let map = RepoMap::new(&ws).render(600);
        assert!(map.chars().count() <= 700, "应截断到预算附近：{}", map.chars().count());
        assert!(map.contains("已截断"), "截断应有标记：{map}");
        fs::remove_dir_all(&ws).ok();
    }

    #[test]
    fn test_instructions_prefers_agents_md() {
        let ws = temp_workspace("instructions");
        write(&ws.join("README.md"), "# README");
        write(&ws.join("AGENTS.md"), "# AGENTS 指令");

        let (file, content) =
            ProjectInstructions::load(&ws, 4000).expect("应有指令文件");
        assert_eq!(file, "AGENTS.md");
        assert!(content.contains("AGENTS 指令"));
        fs::remove_dir_all(&ws).ok();
    }

    #[test]
    fn test_instructions_fallback_and_budget() {
        // 无任何候选文件 → None
        let ws = temp_workspace("none");
        assert!(ProjectInstructions::load(&ws, 4000).is_none());
        fs::remove_dir_all(&ws).ok();

        // README 兜底 + 预算截断
        let ws = temp_workspace("fallback");
        let long = "# 长文档\n".repeat(2000);
        write(&ws.join("README.md"), &long);
        let (file, content) = ProjectInstructions::load(&ws, 500).expect("应有 README");
        assert_eq!(file, "README.md");
        assert!(content.chars().count() <= 500);
        assert!(content.contains("已截断"));
        fs::remove_dir_all(&ws).ok();
    }

    #[test]
    fn test_assemble_wraps_task() {
        let ws = temp_workspace("assemble");
        write(&ws.join("AGENTS.md"), "# 项目约定：使用中文注释");
        write(&ws.join("src/app.rs"), "fn app() {}\n");

        let assembler = ContextAssembler::new(&ws, ContextConfig::default());
        let out = assembler.assemble("修复编译错误");

        assert!(out.starts_with("<project-context>"));
        assert!(out.contains("<task>修复编译错误</task>"));
        assert!(out.contains("项目约定"));
        assert!(out.contains("## 仓库结构"));
        assert!(out.contains("app.rs"));
        assert!(estimate_tokens(&out) > 0);
        fs::remove_dir_all(&ws).ok();
    }

    #[test]
    fn test_assemble_disabled_returns_task_only() {
        let ws = temp_workspace("disabled");
        write(&ws.join("AGENTS.md"), "# 指令");
        let assembler = ContextAssembler::new(
            &ws,
            ContextConfig {
                enabled: false,
                ..ContextConfig::default()
            },
        );
        assert_eq!(assembler.assemble("任务文本"), "任务文本");
        fs::remove_dir_all(&ws).ok();
    }
}
