//! Memory（记忆）实现。
//!
//! 该模块提供 `agentkit-core::memory::Memory` 的一些开箱即用实现：
//! - `InMemoryMemory`：进程内内存实现，适合测试/演示
//! - `FileMemory`：基于 JSON 文件的持久化实现，适合本地小规模使用
//!
use std::sync::Arc;

use agentkit_core::memory::Memory;

pub mod file;
pub mod in_memory;

pub use file::FileMemory;
pub use in_memory::InMemoryMemory;

/// 动态记忆对象类型别名。
///
/// 便于在 agent 组装时以 trait object 的形式传递 Memory。
pub type DynMemory = Arc<dyn Memory>;
