//! Skills锛堟妧鑳斤級妯″潡
//!
//! # 姒傝堪
//!
//! 鏈ā鍧楁彁渚?Skills 鐨勫姞杞姐€佹墽琛屽拰涓?Agent 鐨勯泦鎴愬姛鑳姐€?//!
//! 鍙傝€?zeroclaw 椤圭洰鐨勮璁★細
//! - 鏀寔澶氱閰嶇疆鏂囦欢鏍煎紡锛圱OML/YAML/JSON锛?//! - 鏀寔澶氱鎻愮ず璇嶆敞鍏ユā寮忥紙Full/Compact锛?//! - 鎻愪緵 read_skill 宸ュ叿璇诲彇 skill 璇︾粏淇℃伅
//! - 鏍规嵁 skill 妯″紡鏋勫缓涓嶅悓鐨勭郴缁熸彁绀鸿瘝

pub mod config;
pub mod loader;
pub mod integrator;
pub mod tool_adapter;
pub mod cache;

pub use config::{SkillConfig, SkillMeta};
pub use loader::{SkillLoader, SkillExecutor, SkillImplementation};
pub use integrator::SkillsAutoIntegrator;
pub use tool_adapter::{SkillTool, skills_to_tools, skills_to_prompt_with_mode, read_skill, ReadSkillTool};
pub use cache::{SkillCache, CachedSkillLoader};
pub use agentkit_core::skill::{SkillDefinition, SkillResult, SkillContext};

/// Skills 鎻愮ず璇嶆敞鍏ユā寮?///
/// 鍙傝€?zeroclaw 鐨?SkillsPromptInjectionMode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillsPromptMode {
    /// 瀹屾暣妯″紡锛氬寘鍚墍鏈?skill 鐨勮缁嗚鏄庡拰宸ュ叿
    Full,
    /// 绠€娲佹ā寮忥細鍙寘鍚?skill 鎽樿锛岃缁嗕俊鎭€氳繃 read_skill 宸ュ叿鑾峰彇
    Compact,
}

impl Default for SkillsPromptMode {
    fn default() -> Self {
        Self::Compact
    }
}
