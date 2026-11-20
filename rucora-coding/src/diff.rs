//! 变更追踪与 Diff 管理。
//!
//! 两层设计：
//!
//! 1. **捕获**（[`ChangeRecorder`]）：包装文件修改类工具（`file_edit` /
//!    `file_write`），在调用前后对目标文件做内容快照，记录 [`FileChange`]。
//!    不依赖 git，对未跟踪文件同样生效。
//! 2. **审查/回滚**（[`ChangeTracker`]）：对捕获的变更生成统一 diff
//!    （`similar` crate）、按文件或整体回滚到变更前状态。
//!
//! 另提供 [`git_diff`] 作为 git 仓库的可选增强视角（捕获工具链之外的改动）。
//!
//! # 示例
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use rucora::tools::FileEditTool;
//! use rucora_coding::diff::{ChangeRecorder, ChangeTracker};
//!
//! # fn example() {
//! let tracker = Arc::new(ChangeTracker::default());
//! // 包装后注册给 Agent：file_edit 的每次调用都会被记录
//! let edit = ChangeRecorder::wrap(FileEditTool::new(), tracker.clone(), "workspace");
//! # let _ = (tracker, edit);
//! # }
//! ```

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use rucora_core::error::ToolError;
use rucora_core::tool::types::{ToolContext, ToolRiskLevel};
use rucora_core::tool::ToolCategory;
use rucora_core::tool::Tool;
use serde_json::Value;
use similar::TextDiff;
use tracing::debug;

/// 会记录文件修改的工具名（精确匹配工具 `name()`）。
pub const FILE_MUTATING_TOOLS: &[&str] = &["file_edit", "file_write"];

/// 一次文件变更记录。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FileChange {
    /// 文件路径（规范化为绝对路径）。
    pub path: PathBuf,
    /// 触发变更的工具名。
    pub tool_name: String,
    /// 变更前内容；`None` 表示文件此前不存在（新建）。
    pub before: Option<String>,
    /// 当前内容；`None` 表示文件已删除。
    pub after: Option<String>,
    /// 变更时间（Unix 毫秒）。
    pub at_ms: u128,
    /// 是否已回滚。
    pub rolled_back: bool,
}

/// 单个文件的统一 diff 及统计。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileDiff {
    /// 文件路径。
    pub path: PathBuf,
    /// 统一 diff 文本（含 `---/+++` 头与 3 行上下文）。
    pub diff: String,
    /// 新增行数。
    pub added_lines: usize,
    /// 删除行数。
    pub removed_lines: usize,
    /// 是否为新建文件。
    pub created: bool,
    /// 是否已删除。
    pub deleted: bool,
}

/// 变更追踪器：聚合会话内的全部文件变更。
#[derive(Debug, Default)]
pub struct ChangeTracker {
    changes: Mutex<Vec<FileChange>>,
}

impl ChangeTracker {
    /// 记录一次变更。
    ///
    /// 同一路径的后续变更合并到首条记录：`before` 保留最初快照，
    /// `after` 更新为最新内容。
    pub fn record(&self, change: FileChange) {
        let mut changes = self.changes.lock().expect("变更锁中毒");
        if let Some(existing) = changes
            .iter_mut()
            .find(|c| c.path == change.path && !c.rolled_back)
        {
            existing.after = change.after.clone();
            existing.at_ms = change.at_ms;
            existing.tool_name = change.tool_name;
        } else {
            changes.push(change);
        }
    }

    /// 全部变更（按发生顺序）。
    pub fn changes(&self) -> Vec<FileChange> {
        self.changes.lock().expect("变更锁中毒").clone()
    }

    /// 本会话是否捕获到文件变更。
    pub fn is_empty(&self) -> bool {
        self.changes.lock().expect("变更锁中毒").is_empty()
    }

    /// 为每个变更生成统一 diff。
    pub fn diffs(&self) -> Vec<FileDiff> {
        self.changes()
            .iter()
            .filter(|c| !c.rolled_back)
            .map(Self::diff_of)
            .collect()
    }

    /// 人类可读的变更摘要（如 `2 个文件，+12/-3`）。
    pub fn summary(&self) -> String {
        let diffs = self.diffs();
        let (added, removed) = diffs
            .iter()
            .fold((0, 0), |(a, r), d| (a + d.added_lines, r + d.removed_lines));
        format!("{} 个文件，+{added}/-{removed}", diffs.len())
    }

    /// 回滚单个文件的变更（按路径匹配）。
    ///
    /// # 返回
    ///
    /// 命中并回滚成功返回 `true`；未捕获该文件的变更返回 `false`。
    ///
    /// # Errors
    ///
    /// 文件写入/删除失败时返回 IO 错误。
    pub fn rollback(&self, path: &Path) -> std::io::Result<bool> {
        let mut changes = self.changes.lock().expect("变更锁中毒");
        let Some(change) = changes
            .iter_mut()
            .find(|c| c.path == path && !c.rolled_back)
        else {
            return Ok(false);
        };
        restore(change)?;
        change.rolled_back = true;
        Ok(true)
    }

    /// 回滚全部变更。
    ///
    /// # 返回
    ///
    /// 成功回滚的文件数。
    ///
    /// # Errors
    ///
    /// 任一文件回滚失败时立即返回该 IO 错误（已回滚的文件保持回滚状态）。
    pub fn rollback_all(&self) -> std::io::Result<usize> {
        let mut changes = self.changes.lock().expect("变更锁中毒");
        let mut count = 0;
        for change in changes.iter_mut().filter(|c| !c.rolled_back) {
            restore(change)?;
            change.rolled_back = true;
            count += 1;
        }
        Ok(count)
    }

    /// 生成单个变更的统一 diff 与统计。
    fn diff_of(change: &FileChange) -> FileDiff {
        let empty = "";
        let before = change.before.as_deref().unwrap_or(empty);
        let after = change.after.as_deref().unwrap_or(empty);
        let diff = TextDiff::from_lines(before, after);
        let mut added = 0usize;
        let mut removed = 0usize;
        for change_item in diff.iter_all_changes() {
            use similar::ChangeTag::*;
            match change_item.tag() {
                Insert => added += 1,
                Delete => removed += 1,
                Equal => {}
            }
        }
        let path_str = change.path.display().to_string();
        let unified = diff
            .unified_diff()
            .context_radius(3)
            .header(&path_str, &path_str)
            .to_string();
        FileDiff {
            path: change.path.clone(),
            diff: unified,
            added_lines: added,
            removed_lines: removed,
            created: change.before.is_none(),
            deleted: change.after.is_none(),
        }
    }
}

/// 将变更回滚到变更前状态。
fn restore(change: &FileChange) -> std::io::Result<()> {
    match &change.before {
        // 新建的文件 → 删除
        None => {
            if fs::metadata(&change.path).is_ok() {
                fs::remove_file(&change.path)?;
            }
        }
        // 修改的文件 → 恢复旧内容
        Some(before) => {
            if let Some(parent) = change.path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&change.path, before)?;
        }
    }
    debug!(path = %change.path.display(), "coding.diff.restored");
    Ok(())
}

/// 变更捕获工具包装器。
///
/// 包装文件修改类工具，在调用前后对目标文件做内容快照并写入追踪器。
/// 非文件修改类工具（名称不在 [`FILE_MUTATING_TOOLS`] 中）直接透传。
pub struct ChangeRecorder<T: Tool> {
    inner: T,
    tracker: Arc<ChangeTracker>,
    workspace: PathBuf,
}

impl<T: Tool> ChangeRecorder<T> {
    /// 包装工具。
    pub fn wrap(
        inner: T,
        tracker: Arc<ChangeTracker>,
        workspace: impl Into<PathBuf>,
    ) -> Self {
        Self {
            inner,
            tracker,
            workspace: workspace.into(),
        }
    }

    /// 从工具输入中提取目标文件路径（`path` 字段），并规范化为绝对路径。
    ///
    /// 相对路径按工作区解析；不存在的文件回退到父目录的规范化结果。
    fn resolve_path(&self, input: &Value) -> Option<PathBuf> {
        let raw = input.get("path")?.as_str()?;
        let path = Path::new(raw);
        let joined = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.workspace.join(path)
        };
        Some(
            fs::canonicalize(&joined)
                .unwrap_or_else(|_| normalize_parent(&joined)),
        )
    }

    /// 读取文件内容；文件不存在返回 `None`，不可读（二进制等）返回 `Err`。
    fn snapshot(path: &Path) -> Result<Option<String>, ()> {
        if !path.exists() {
            return Ok(None);
        }
        fs::read_to_string(path).map(Some).map_err(|_| ())
    }
}

/// 规范化父目录并拼接文件名（用于尚未存在的文件）。
fn normalize_parent(path: &Path) -> PathBuf {
    let file_name = path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().to_string(),
    );
    match path.parent() {
        Some(parent) => fs::canonicalize(parent)
            .unwrap_or_else(|_| parent.to_path_buf())
            .join(file_name),
        None => path.to_path_buf(),
    }
}

#[async_trait]
impl<T: Tool> Tool for ChangeRecorder<T> {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn description(&self) -> Option<&str> {
        self.inner.description()
    }

    fn categories(&self) -> &'static [ToolCategory] {
        self.inner.categories()
    }

    fn input_schema(&self) -> Value {
        self.inner.input_schema()
    }

    fn risk_level(&self) -> ToolRiskLevel {
        self.inner.risk_level()
    }

    async fn call(&self, input: Value, context: &ToolContext) -> Result<Value, ToolError> {
        // 非文件修改工具直接透传
        if !FILE_MUTATING_TOOLS.contains(&self.inner.name()) {
            return self.inner.call(input, context).await;
        }

        // 快照"变更前"内容；不可读（二进制/无权限）则不追踪
        let target = self.resolve_path(&input);
        let before = match &target {
            Some(path) => Self::snapshot(path).unwrap_or(None),
            None => None,
        };
        if target.is_none() || (before.is_none() && target.as_ref().is_some_and(|p| p.exists())) {
            // 无法解析路径，或文件存在但不可读：放弃追踪，直接执行
            return self.inner.call(input, context).await;
        }

        let result = self.inner.call(input, context).await?;
        let at_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis());

        let path = target.expect("上方已确认可解析");
        let after = Self::snapshot(&path).unwrap_or(None);
        self.tracker.record(FileChange {
            path,
            tool_name: self.inner.name().to_string(),
            before,
            after,
            at_ms,
            rolled_back: false,
        });
        Ok(result)
    }
}

/// git 视角的变更（可选增强）。
///
/// 返回 `git --no-pager diff HEAD` 的输出（跟踪文件的改动），
/// 并附 `git status --porcelain` 的简报。以下情况返回 `None`：
/// git 不可用、目录不是 git 仓库、或命令执行失败。
pub async fn git_diff(workspace: &Path) -> Option<String> {
    let inside = tokio::process::Command::new("git")
        .arg("-C")
        .arg(workspace)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .await
        .ok()?;
    if !inside.status.success() {
        return None;
    }

    let diff = tokio::process::Command::new("git")
        .arg("-C")
        .arg(workspace)
        .args(["--no-pager", "diff", "HEAD"])
        .output()
        .await
        .ok()?;
    let status = tokio::process::Command::new("git")
        .arg("-C")
        .arg(workspace)
        .args(["status", "--porcelain"])
        .output()
        .await
        .ok()?;

    let diff_text = String::from_utf8_lossy(&diff.stdout).to_string();
    let status_text = String::from_utf8_lossy(&status.stdout).to_string();
    if diff_text.trim().is_empty() && status_text.trim().is_empty() {
        return Some("（工作区干净，无变更）".to_string());
    }

    let mut out = String::new();
    if !diff_text.trim().is_empty() {
        out.push_str(&diff_text);
    }
    if !status_text.trim().is_empty() {
        out.push_str("\n# 未提交变更一览（git status --porcelain）\n");
        out.push_str(&status_text);
    }
    Some(out)
}
