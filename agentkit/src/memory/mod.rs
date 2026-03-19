use std::sync::Arc;

use agentkit_core::memory::Memory;

pub mod file;
pub mod in_memory;

pub use file::FileMemory;
pub use in_memory::InMemoryMemory;

pub type DynMemory = Arc<dyn Memory>;
