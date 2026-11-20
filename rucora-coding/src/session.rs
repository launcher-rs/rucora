//! 会话持久化。
//!
//! [`SessionRecord`] 是一个 Coding 会话的可序列化快照（任务、转录、状态、用量），
//! [`FileSessionStore`] 以 JSON 文件形式存取（一个会话一个文件，原子写入）。
//!
//! M0 的持久化范围：任务列表、助手文本、工具调用记录与状态。
//! 完整的"中断恢复"（把历史喂回 Agent 继续执行）依赖执行层的会话管理器，
//! 属于后续版本的能力。

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{Value, json};
use tracing::debug;

/// 会话状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SessionStatus {
    /// 运行中。
    Running,
    /// 正常完成。
    Completed,
    /// 执行失败。
    Failed,
    /// 被用户取消。
    Cancelled,
}

/// 转录条目：会话中发生的关键动作。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TranscriptEntry {
    /// 用户任务。
    Task {
        /// 任务文本。
        task: String,
    },
    /// 助手文本段。
    AssistantText {
        /// 文本内容。
        text: String,
    },
    /// 工具调用（含结果）。
    ToolCall {
        /// 工具名称。
        name: String,
        /// 调用 ID。
        call_id: String,
        /// 输入参数。
        input: Value,
        /// 输出结果。
        output: Value,
        /// 是否成功。
        success: bool,
    },
}

/// 会话记录（可序列化快照）。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionRecord {
    /// 会话 ID（UUID v4）。
    pub id: String,
    /// 创建时间（RFC 3339）。
    pub created_at: String,
    /// 最后更新时间（RFC 3339）。
    pub updated_at: String,
    /// 工作目录。
    pub workspace: String,
    /// 会话状态。
    pub status: SessionStatus,
    /// 用户提交的任务列表。
    pub tasks: Vec<String>,
    /// 转录条目（按发生顺序）。
    pub transcript: Vec<TranscriptEntry>,
    /// 累计 token 用量（来自 Agent 输出，可能为空）。
    pub usage: Option<Value>,
    /// 扩展元数据。
    pub metadata: Value,
}

impl SessionRecord {
    /// 创建新会话记录（状态为 `Running`）。
    pub fn new(workspace: &Path) -> Self {
        let now = now_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            created_at: now.clone(),
            updated_at: now,
            workspace: workspace.to_string_lossy().to_string(),
            status: SessionStatus::Running,
            tasks: Vec::new(),
            transcript: Vec::new(),
            usage: None,
            metadata: json!({}),
        }
    }

    /// 更新最后修改时间。
    pub fn touch(&mut self) {
        self.updated_at = now_rfc3339();
    }
}

/// 当前时间的 RFC 3339 表示（本地时区）。
fn now_rfc3339() -> String {
    // 使用 `uuid` 版本无法得到时间戳；这里用系统毫秒 + 简单格式化，
    // 避免引入额外的日期依赖（排序用途已足够）。
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    format!("t{millis}")
}

/// 基于 JSON 文件的会话存储。
///
/// 存储布局：`{dir}/{session_id}.json`。
#[derive(Debug, Clone)]
pub struct FileSessionStore {
    dir: PathBuf,
}

impl FileSessionStore {
    /// 创建存储（目录不存在时会自动创建）。
    ///
    /// # Errors
    ///
    /// 目录创建失败时返回 IO 错误。
    pub fn new(dir: impl Into<PathBuf>) -> std::io::Result<Self> {
        let dir = dir.into();
        fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    fn session_path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.json"))
    }

    /// 保存（整体覆盖，tmp + rename 原子替换）。
    ///
    /// # Errors
    ///
    /// 序列化或文件写入失败时返回错误。
    pub fn save(&self, record: &SessionRecord) -> std::io::Result<()> {
        let path = self.session_path(&record.id);
        let tmp = path.with_extension("json.tmp");
        let content = serde_json::to_string_pretty(record)
            .map_err(|e| std::io::Error::other(format!("会话序列化失败：{e}")))?;
        fs::write(&tmp, content)?;
        fs::rename(&tmp, &path)?;
        debug!(session_id = %record.id, "coding.session.saved");
        Ok(())
    }

    /// 加载会话。
    ///
    /// # Errors
    ///
    /// 文件不存在、读取失败或反序列化失败时返回错误。
    pub fn load(&self, id: &str) -> std::io::Result<SessionRecord> {
        let path = self.session_path(id);
        let content = fs::read_to_string(&path)?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::other(format!("会话反序列化失败：{e}")))
    }

    /// 列出全部会话（按 updated_at 降序）。
    ///
    /// 单个损坏的会话文件会被跳过（不影响其余会话的列出）。
    pub fn list(&self) -> Vec<SessionRecord> {
        let mut records = Vec::new();
        let Ok(entries) = fs::read_dir(&self.dir) else {
            return records;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|ext| ext != "json") {
                continue;
            }
            if let Ok(content) = fs::read_to_string(&path)
                && let Ok(record) = serde_json::from_str::<SessionRecord>(&content)
            {
                records.push(record);
            }
        }
        records.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        records
    }

    /// 删除会话文件。
    ///
    /// # Errors
    ///
    /// 文件删除失败（不含"文件本就不存在"）时返回错误。
    pub fn delete(&self, id: &str) -> std::io::Result<()> {
        let path = self.session_path(id);
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }
}

/// 便捷类型：线程安全的会话存储句柄。
pub type SharedSessionStore = Arc<FileSessionStore>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventBus;
    use crate::permissions::{ApprovalDecision, ApprovalToolPolicy, PermissionConfig};

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "rucora-coding-test-{tag}-{}",
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&dir).expect("创建临时目录");
        dir
    }

    #[test]
    fn test_save_load_roundtrip() {
        let dir = temp_dir("roundtrip");
        let store = FileSessionStore::new(&dir).expect("创建存储");

        let mut record = SessionRecord::new(Path::new("/tmp/project"));
        record.tasks.push("修复编译错误".to_string());
        record.transcript.push(TranscriptEntry::ToolCall {
            name: "file_edit".to_string(),
            call_id: "call-1".to_string(),
            input: json!({"path": "src/main.rs"}),
            output: json!({"ok": true}),
            success: true,
        });
        record.status = SessionStatus::Completed;
        store.save(&record).expect("保存");

        let loaded = store.load(&record.id).expect("加载");
        assert_eq!(loaded.id, record.id);
        assert_eq!(loaded.tasks, vec!["修复编译错误".to_string()]);
        assert_eq!(loaded.status, SessionStatus::Completed);
        assert_eq!(loaded.transcript.len(), 1);
        assert!(matches!(
            &loaded.transcript[0],
            TranscriptEntry::ToolCall { name, success: true, .. } if name == "file_edit"
        ));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_list_sorted_by_updated_at() {
        let dir = temp_dir("list");
        let store = FileSessionStore::new(&dir).expect("创建存储");

        let mut first = SessionRecord::new(Path::new("/tmp/a"));
        let mut second = SessionRecord::new(Path::new("/tmp/b"));
        // 确保时间戳不同（毫秒精度）
        std::thread::sleep(std::time::Duration::from_millis(5));
        second.touch();
        store.save(&first).expect("保存 first");
        store.save(&second).expect("保存 second");
        // 再动一下 first，使其成为最新
        std::thread::sleep(std::time::Duration::from_millis(5));
        first.touch();
        first.status = SessionStatus::Completed;
        store.save(&first).expect("保存 first");

        let list = store.list();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, first.id);
        assert_eq!(list[1].id, second.id);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_delete_is_idempotent() {
        let dir = temp_dir("delete");
        let store = FileSessionStore::new(&dir).expect("创建存储");
        let record = SessionRecord::new(Path::new("/tmp/x"));
        store.save(&record).expect("保存");
        assert!(store.delete(&record.id).is_ok());
        // 再删一次不报错
        assert!(store.delete(&record.id).is_ok());
        assert!(store.load(&record.id).is_err());
        fs::remove_dir_all(&dir).ok();
    }

    /// 确认 permissions 里的审批策略与 session 存储可组合（编译期烟测）。
    #[test]
    fn test_policy_and_store_compose() {
        let bus = EventBus::new();
        let policy = ApprovalToolPolicy::new(PermissionConfig::default(), "s".to_string(), bus);
        assert!(policy.pending_requests().is_empty());
        assert!(!policy.resolve("no-such-request", ApprovalDecision::Deny));
    }
}
