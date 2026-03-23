//! 记忆工具模块。
//!
//! 提供长期记忆存储和检索功能。

use agentkit_core::{
    error::ToolError,
    memory::{Memory, MemoryItem, MemoryQuery},
    tool::{Tool, ToolCategory},
};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Arc;

use crate::memory::InMemoryMemory;

/// 记忆存储工具：存储信息到长期记忆。
///
/// 让 Agent 将事实、偏好或笔记存储到长期记忆中。
/// 支持不同类别：core（永久）、daily（会话）、conversation（对话）。
///
/// 输入格式：
/// ```json
/// {
///   "key": "user_lang",
///   "content": "用户偏好 Rust 语言",
///   "category": "core"
/// }
/// ```
pub struct MemoryStoreTool {
    memory: Arc<dyn Memory>,
}

impl MemoryStoreTool {
    /// 创建一个新的 MemoryStoreTool 实例。
    pub fn new() -> Self {
        Self::from_memory(Arc::new(InMemoryMemory::new()))
    }

    pub fn from_memory(memory: Arc<dyn Memory>) -> Self {
        Self { memory }
    }
}

impl Default for MemoryStoreTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for MemoryStoreTool {
    /// 返回工具名称。
    fn name(&self) -> &str {
        "memory_store"
    }

    /// 返回工具描述。
    fn description(&self) -> Option<&str> {
        Some(
            "存储事实、偏好或笔记到长期记忆。使用 category 'core' 表示永久记忆，'daily' 表示会话笔记，'conversation' 表示对话上下文",
        )
    }

    /// 返回工具分类。
    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::Memory]
    }

    /// 返回输入参数的 JSON Schema。
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "key": {
                    "type": "string",
                    "description": "记忆的唯一键（如 'user_lang', 'project_stack'）"
                },
                "content": {
                    "type": "string",
                    "description": "要记忆的信息"
                },
                "category": {
                    "type": "string",
                    "description": "记忆类别: 'core' (永久), 'daily' (会话), 'conversation' (对话), 或自定义类别。默认为 'core'"
                }
            },
            "required": ["key", "content"]
        })
    }

    /// 执行记忆存储。
    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let key = input
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Message("缺少必需的 'key' 字段".to_string()))?;

        let content = input
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Message("缺少必需的 'content' 字段".to_string()))?;

        let category = input
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("core");

        // 存储
        let full_key = format!("{}:{}", category, key);
        self.memory
            .add(MemoryItem {
                id: full_key,
                content: content.to_string(),
                metadata: None,
            })
            .await
            .map_err(|e| ToolError::Message(e.to_string()))?;

        Ok(json!({
            "success": true,
            "key": key,
            "category": category,
            "message": format!("已存储记忆: {}", key)
        }))
    }
}

/// 记忆回忆工具：从长期记忆中检索信息。
///
/// 根据键从长期记忆中检索存储的信息。
///
/// 输入格式：
/// ```json
/// {
///   "key": "user_lang",
///   "category": "core"
/// }
/// ```
pub struct MemoryRecallTool {
    memory: Arc<dyn Memory>,
}

impl MemoryRecallTool {
    /// 创建一个新的 MemoryRecallTool 实例。
    pub fn new() -> Self {
        Self::from_memory(Arc::new(InMemoryMemory::new()))
    }

    /// 从现有 MemoryStoreTool 创建实例以共享存储。
    pub fn from_store(store: &MemoryStoreTool) -> Self {
        Self {
            memory: store.memory.clone(),
        }
    }

    pub fn from_memory(memory: Arc<dyn Memory>) -> Self {
        Self { memory }
    }
}

impl Default for MemoryRecallTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for MemoryRecallTool {
    /// 返回工具名称。
    fn name(&self) -> &str {
        "memory_recall"
    }

    /// 返回工具描述。
    fn description(&self) -> Option<&str> {
        Some("从长期记忆中检索存储的信息")
    }

    /// 返回工具分类。
    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::Memory]
    }

    /// 返回输入参数的 JSON Schema。
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "key": {
                    "type": "string",
                    "description": "要检索的记忆键"
                },
                "category": {
                    "type": "string",
                    "description": "记忆类别，默认为 'core'"
                }
            },
            "required": ["key"]
        })
    }

    /// 执行记忆检索。
    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let key = input
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Message("缺少必需的 'key' 字段".to_string()))?;

        let category = input
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("core");

        // 检索
        let full_key = format!("{}:{}", category, key);
        let mut results = self
            .memory
            .query(MemoryQuery {
                text: full_key,
                limit: 1,
            })
            .await
            .map_err(|e| ToolError::Message(e.to_string()))?;

        if let Some(item) = results.pop() {
            Ok(json!({
                "found": true,
                "key": key,
                "category": category,
                "content": item.content
            }))
        } else {
            Ok(json!({
                "found": false,
                "key": key,
                "category": category,
                "content": null
            }))
        }
    }
}
