//! git worktree 隔离。
//!
//! 多会话并发修改同一个仓库时，用 git worktree 把每个会话隔离到独立的
//! 工作目录与分支，互不踩踏：
//!
//! ```text
//! 主仓库 repo/                      （主分支）
//! ├── worktree-a/  @ rucora/task-a  （会话 A 的工作副本）
//! └── worktree-b/  @ rucora/task-b  （会话 B 的工作副本）
//! ```
//!
//! [`create_worktree`] 基于主仓库创建新分支与工作目录；[`remove_worktree`]
//! 清理工作目录与分支。两个函数都直接调用 `git` CLI（要求环境可用 git）。
//!
//! # 示例
//!
//! ```rust,no_run
//! use rucora_coding::worktree::create_worktree;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let worktree = create_worktree(
//!     std::path::Path::new("path/to/repo"),
//!     "task-42",
//!     std::path::Path::new("path/to/repo-worktrees"),
//! ).await?;
//! // 会话的工作目录指向 worktree.path，分支为 rucora/task-42
//! println!("worktree: {} @ {}", worktree.path.display(), worktree.branch);
//! # Ok(())
//! # }
//! ```

use std::path::{Path, PathBuf};
use std::process::Stdio;

use tracing::debug;

/// 一个已创建的 worktree。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Worktree {
    /// 工作目录（绝对路径）。
    pub path: PathBuf,
    /// 关联的分支名（`rucora/{name}`）。
    pub branch: String,
}

/// 创建 worktree（新分支 `rucora/{name}`，基于可选的 base 引用，默认 HEAD）。
///
/// `worktree_root` 是存放所有 worktree 的根目录（建议放在主仓库之外的兄弟目录），
/// 不存在时自动创建。返回的 `Worktree.path` 即为隔离会话的工作目录。
///
/// # Errors
///
/// git 不可用、目录不是 git 仓库、分支已存在或路径冲突时返回 IO 错误。
pub async fn create_worktree(
    repo: &Path,
    name: &str,
    worktree_root: &Path,
) -> std::io::Result<Worktree> {
    let branch = format!("rucora/{name}");
    let path = worktree_root.join(name);

    if path.exists() {
        return Err(std::io::Error::other(format!(
            "worktree 路径已存在：{}",
            path.display()
        )));
    }
    fs_err_ok(create_dir_all(worktree_root).await)?;

    // git -C repo worktree add -b <branch> <path> [base]
    let mut cmd = tokio::process::Command::new("git");
    cmd.arg("-C").arg(repo).args(["worktree", "add", "-b", &branch]);
    cmd.arg(&path);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd.output().await?;
    if !output.status.success() {
        return Err(std::io::Error::other(format!(
            "创建 worktree 失败：{}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    debug!(repo = %repo.display(), branch = %branch, path = %path.display(), "coding.worktree.created");
    Ok(Worktree {
        path: strip_unc(&path),
        branch,
    })
}

/// 去掉 Windows canonicalize 产生的 `\\?\` UNC 前缀（git CLI 不识别该前缀）。
fn strip_unc(path: &Path) -> PathBuf {
    let raw = path.to_string_lossy();
    if let Some(stripped) = raw.strip_prefix(r#"\\?"#) {
        PathBuf::from(stripped)
    } else {
        path.to_path_buf()
    }
}

/// 移除 worktree（清理工作目录，并删除关联分支）。
///
/// 使用 `--force`：worktree 中未提交的修改会被丢弃——回滚场景是刻意为之。
///
/// # Errors
///
/// git 命令失败时返回 IO 错误；worktree 不存在同样视为错误
/// （调用方可先检查路径）。
pub async fn remove_worktree(repo: &Path, worktree: &Worktree) -> std::io::Result<()> {
    let output = tokio::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["worktree", "remove", "--force"])
        .arg(&worktree.path)
        .output()
        .await?;
    if !output.status.success() {
        return Err(std::io::Error::other(format!(
            "移除 worktree 失败：{}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    // 清理分支
    let branch = tokio::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["branch", "-D", &worktree.branch])
        .output()
        .await?;
    if !branch.status.success() {
        debug!(
            branch = %worktree.branch,
            stderr = %String::from_utf8_lossy(&branch.stderr),
            "coding.worktree.branch_delete_skipped"
        );
    }

    debug!(path = %worktree.path.display(), "coding.worktree.removed");
    Ok(())
}

/// 列出主仓库下的全部 worktree（`git worktree list --porcelain` 解析）。
///
/// # Errors
///
/// git 命令失败时返回 IO 错误。
pub async fn list_worktrees(repo: &Path) -> std::io::Result<Vec<Worktree>> {
    let output = tokio::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["worktree", "list", "--porcelain"])
        .output()
        .await?;
    if !output.status.success() {
        return Err(std::io::Error::other(format!(
            "列出 worktree 失败：{}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut worktrees = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    for line in text.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            current_path = Some(PathBuf::from(path));
        } else if let Some(branch) = line.strip_prefix("branch ") {
            // branch refs/heads/rucora/task-42 → rucora/task-42
            let branch = branch.strip_prefix("refs/heads/").unwrap_or(branch);
            if let Some(path) = current_path.take() {
                worktrees.push(Worktree { path, branch: branch.to_string() });
            }
        }
    }
    Ok(worktrees)
}

/// 异步创建目录的小包装（错误转 IO）。
async fn create_dir_all(dir: &Path) -> std::io::Result<()> {
    tokio::fs::create_dir_all(dir).await
}

/// 把 `Result<T, E>` 拍平成 `std::io::Result<T>`（E 已是 io::Error 时直通）。
fn fs_err_ok<T>(result: std::io::Result<T>) -> std::io::Result<T> {
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// 初始化一个带首次提交的临时 git 仓库（worktree 依赖 HEAD）。
    fn init_repo(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "rucora-coding-wt-{tag}-{}",
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&dir).expect("创建仓库目录");

        let output = std::process::Command::new("git")
            .arg("init")
            .current_dir(&dir)
            .output()
            .expect("git init");
        assert!(output.status.success(), "git init 失败");

        fs::write(dir.join("README.md"), "# test repo\n").expect("写文件");
        // 首次提交（内联身份配置，避免依赖全局 git config）
        let commit = std::process::Command::new("git")
            .args([
                "-c", "user.name=test", "-c", "user.email=test@test.local",
                "add", "-A",
            ])
            .current_dir(&dir)
            .output()
            .expect("git add");
        assert!(commit.status.success(), "git add 失败");
        let commit = std::process::Command::new("git")
            .args([
                "-c", "user.name=test", "-c", "user.email=test@test.local",
                "commit", "-m", "init",
            ])
            .current_dir(&dir)
            .output()
            .expect("git commit");
        assert!(commit.status.success(), "git commit 失败");
        dir
    }

    #[tokio::test]
    async fn test_create_list_remove_lifecycle() {
        let repo = init_repo("lifecycle");
        let wt_root = repo.parent().expect("临时目录必有父").join("worktrees");

        // 创建
        let wt = create_worktree(&repo, "task-a", &wt_root)
            .await
            .expect("创建 worktree");
        assert_eq!(wt.branch, "rucora/task-a");
        assert!(wt.path.join("README.md").exists(), "worktree 应含仓库文件");

        // 列表
        let list = list_worktrees(&repo).await.expect("列出 worktree");
        assert!(
            list.iter().any(|w| w.branch == "rucora/task-a"),
            "列表应包含新建 worktree：{list:?}"
        );

        // worktree 内的修改不影响主仓库
        fs::write(wt.path.join("changed.txt"), "isolated").expect("在 worktree 写文件");
        let main_status = std::process::Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&repo)
            .output()
            .expect("git status");
        assert!(
            String::from_utf8_lossy(&main_status.stdout).trim().is_empty(),
            "主仓库应保持干净（隔离生效）"
        );

        // 移除
        remove_worktree(&repo, &wt).await.expect("移除 worktree");
        assert!(!wt.path.exists(), "worktree 目录应被清理");
        let list = list_worktrees(&repo).await.expect("再列 worktree");
        assert!(
            !list.iter().any(|w| w.branch == "rucora/task-a"),
            "移除后列表不应包含该 worktree"
        );

        fs::remove_dir_all(&wt_root).ok();
        fs::remove_dir_all(&repo).ok();
    }

    #[tokio::test]
    async fn test_duplicate_name_fails() {
        let repo = init_repo("dup");
        let wt_root = repo.parent().expect("临时目录必有父").join("worktrees");

        let wt = create_worktree(&repo, "task-b", &wt_root)
            .await
            .expect("第一次创建");
        let again = create_worktree(&repo, "task-b", &wt_root).await;
        assert!(again.is_err(), "同名/同路径重复创建应失败");

        remove_worktree(&repo, &wt).await.expect("清理");
        fs::remove_dir_all(&wt_root).ok();
        fs::remove_dir_all(&repo).ok();
    }

    #[tokio::test]
    async fn test_non_git_dir_fails() {
        let dir = std::env::temp_dir().join(format!(
            "rucora-coding-nogit-{}",
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&dir).expect("创建普通目录");
        let result = create_worktree(&dir, "x", &dir.join("wts")).await;
        assert!(result.is_err(), "非 git 目录应失败");
        fs::remove_dir_all(&dir).ok();
    }
}
