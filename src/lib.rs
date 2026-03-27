//! AgentKit - LLM 应用开发框架
//!
//! # 概述
//!
//! AgentKit 是一个用于构建生产级 LLM 应用的框架，提供：
//! - Skills（技能）系统 - 可扩展的功能模块
//! - Agent（智能体） - 决策和执行单元
//! - Provider（模型提供商） - LLM 连接层
//! - Runtime（运行时） - 编排和执行引擎
//!
//! # 快速开始
//!
//! ## 使用 Skills
//!
//! ```rust,no_run
//! use agentkit::skills::{SkillLoader, SkillExecutor};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // 1. 加载 Skills
//!     let mut loader = SkillLoader::new("skills/");
//!     loader.load_from_dir().await?;
//!
//!     // 2. 执行 Skill
//!     let executor = SkillExecutor::new();
//!     let skill = loader.get_skill("weather-query").unwrap();
//!     let result = executor.execute(
//!         skill,
//!         std::path::Path::new("skills/weather"),
//!         &serde_json::json!({"city": "Beijing"})
//!     ).await?;
//!
//!     println!("结果：{:?}", result);
//!     Ok(())
//! }
//! ```
//!
//! ## 与 Agent 集成
//!
//! ```rust,no_run
//! use agentkit::agent::DefaultAgent;
//! use agentkit::provider::OpenAiProvider;
//! use agentkit::skills::SkillLoader;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // 1. 加载 Skills
//!     let mut loader = SkillLoader::new("skills/");
//!     loader.load_from_dir().await?;
//!
//!     // 2. 创建 Agent
//!     let provider = OpenAiProvider::from_env()?;
//!     let agent = DefaultAgent::builder()
//!         .provider(provider)
//!         .model("gpt-4")
//!         .system_prompt("你是有用的助手")
//!         .build();
//!
//!     // 3. 使用 Agent
//!     let output = agent.run("北京天气怎么样？").await?;
//!     println!("{}", output.text().unwrap());
//!
//!     Ok(())
//! }
//! ```

// Skills 模块
pub mod skills;

// 重新导出常用类型
pub use skills::{SkillLoader, SkillExecutor, SkillDefinition, SkillResult};
