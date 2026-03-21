//! agentkit 统一配置系统
//!
//! # 概述
//!
//! 本模块提供 agentkit 的统一配置系统，支持：
//! - 从配置文件（YAML/TOML）加载
//! - 从环境变量加载
//! - Profile 切换（dev/prod）
//! - 环境变量覆盖配置
//!
//! # 设计目标
//!
//! - 支持从文件（YAML/TOML）与环境变量加载
//! - 支持 profile（dev/prod）覆盖
//! - 尽量只描述"怎么组装 agentkit"，不强绑定具体业务
//!
//! # 环境变量
//!
//! | 变量名 | 说明 | 示例 |
//! |--------|------|------|
//! | `AGENTKIT_CONFIG` | 配置文件路径 | `config.yaml` |
//! | `AGENTKIT_PROFILE` | Profile 名称 | `prod` |
//! | `AGENTKIT_PROVIDER_KIND` | Provider 类型 | `openai` |
//! | `AGENTKIT_MODEL` | 默认模型 | `gpt-4o-mini` |
//! | `AGENTKIT_OPENAI_API_KEY` | OpenAI API Key | `sk-...` |
//! | `AGENTKIT_OPENAI_BASE_URL` | OpenAI Base URL | `https://api.openai.com/v1` |
//! | `AGENTKIT_OPENAI_MODEL` | OpenAI 模型 | `gpt-4o` |
//! | `AGENTKIT_OLLAMA_BASE_URL` | Ollama Base URL | `http://localhost:11434` |
//! | `AGENTKIT_OLLAMA_MODEL` | Ollama 模型 | `llama3` |
//!
//! # 配置文件格式
//!
//! ## YAML 示例
//!
//! ```yaml
//! profiles:
//!   default:
//!     provider:
//!       kind: ollama
//!       model: llama3
//!   prod:
//!     provider:
//!       kind: openai
//!       model: gpt-4o-mini
//!       openai:
//!         api_key: "${OPENAI_API_KEY}"
//! ```
//!
//! ## TOML 示例
//!
//! ```toml
//! [profiles.default]
//! [profiles.default.provider]
//! kind = "ollama"
//! model = "llama3"
//!
//! [profiles.prod]
//! [profiles.prod.provider]
//! kind = "openai"
//! model = "gpt-4o-mini"
//! ```
//!
//! # 使用示例
//!
//! ## 基本使用
//!
//! ```rust,no_run
//! use agentkit::config::AgentkitConfig;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // 从环境变量和配置文件加载
//! let profile = AgentkitConfig::load().await?;
//!
//! // 构建 provider
//! let provider = AgentkitConfig::build_provider(&profile)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## 设置环境变量
//!
//! ```bash
//! # 设置配置文件路径
//! export AGENTKIT_CONFIG=config.yaml
//!
//! # 设置 Profile
//! export AGENTKIT_PROFILE=prod
//!
//! # 设置 OpenAI API Key
//! export AGENTKIT_OPENAI_API_KEY=sk-...
//!
//! # 运行程序
//! cargo run
//! ```
//!
//! ## 从指定文件加载 Profile
//!
//! ```rust,no_run
//! use agentkit::config::AgentkitConfig;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // 从 YAML 文件加载 default profile
//! let profile = AgentkitConfig::load_profile_from_file("config.yaml", "default").await?;
//!
//! // 从 TOML 文件加载 prod profile
//! let profile = AgentkitConfig::load_profile_from_file("config.toml", "prod").await?;
//! # Ok(())
//! # }
//! ```
//!
//! # 配置结构
//!
//! ```text
//! AgentkitConfig
//! ├── profile: Option<String>         - 当前选中的 profile
//! └── profiles: HashMap<String, ProfileConfig>
//!     └── ProfileConfig
//!         └── provider: ProviderConfig
//!             ├── kind: String        - provider 类型（openai/ollama/router）
//!             ├── model: Option<String>
//!             ├── openai: Option<OpenAiConfig>
//!             ├── ollama: Option<OllamaConfig>
//!             └── router: Option<RouterConfig>
//! ```
//!
//! # Provider 类型
//!
//! ## openai
//!
//! 使用 OpenAI API 兼容服务：
//!
//! ```yaml
//! provider:
//!   kind: openai
//!   model: gpt-4o-mini
//!   openai:
//!     api_key: "sk-..."
//!     base_url: "https://api.openai.com/v1"
//! ```
//!
//! ## ollama
//!
//! 使用 Ollama 本地模型：
//!
//! ```yaml
//! provider:
//!   kind: ollama
//!   model: llama3
//!   ollama:
//!     base_url: "http://localhost:11434"
//! ```
//!
//! ## router
//!
//! 使用多 Provider 路由：
//!
//! ```yaml
//! provider:
//!   kind: router
//!   router:
//!     default_provider: ollama
//!     ollama:
//!       - base_url: "http://localhost:11434"
//!     openai:
//!       - api_key: "sk-..."
//! ```

use std::collections::HashMap;
use std::path::Path;

use agentkit_core::error::ProviderError;
use serde::{Deserialize, Serialize};

use crate::provider::{OllamaProvider, OpenAiProvider, RouterProvider};

/// agentkit 统一配置
///
/// # 字段说明
///
/// - `profile`: 选中的 profile（只在 load 时使用）
/// - `profiles`: 多 profile 配置
///
/// # 示例
///
/// ```rust
/// use agentkit::config::AgentkitConfig;
/// use std::collections::HashMap;
///
/// let config = AgentkitConfig {
///     profile: Some("default".to_string()),
///     profiles: HashMap::new(),
/// };
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentkitConfig {
    /// 选中的 profile（只在 load 时使用；序列化时可为空）
    #[serde(default)]
    pub profile: Option<String>,

    /// 多 profile 配置
    ///
    /// # 文件格式示例
    ///
    /// ```yaml
    /// profiles:
    ///   default:
    ///     provider:
    ///       kind: ollama
    ///       model: llama3
    ///   prod:
    ///     provider:
    ///       kind: openai
    ///       model: gpt-4o-mini
    /// ```
    #[serde(default)]
    pub profiles: HashMap<String, ProfileConfig>,
}

/// Profile 配置
///
/// 包含单个 profile 的所有配置项。
///
/// # 示例
///
/// ```rust
/// use agentkit::config::ProfileConfig;
///
/// let config = ProfileConfig {
///     provider: Default::default(),
/// };
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProfileConfig {
    /// Provider 配置
    #[serde(default)]
    pub provider: ProviderConfig,
}

/// Provider 配置
///
/// 支持三种 provider 类型：
/// - `openai`: OpenAI API 兼容服务
/// - `ollama`: Ollama 本地模型
/// - `router`: 多 Provider 路由
///
/// # 示例
///
/// ```rust
/// use agentkit::config::ProviderConfig;
///
/// // OpenAI 配置
/// let config = ProviderConfig {
///     kind: "openai".to_string(),
///     model: Some("gpt-4o-mini".to_string()),
///     openai: None,
///     ollama: None,
///     router: None,
/// };
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// provider 类型：openai/ollama/router
    #[serde(default)]
    pub kind: String,

    /// 默认模型（会写入 request.model 或 provider 的默认模型）
    #[serde(default)]
    pub model: Option<String>,

    /// OpenAI 配置
    #[serde(default)]
    pub openai: Option<OpenAiConfig>,

    /// Ollama 配置
    #[serde(default)]
    pub ollama: Option<OllamaConfig>,

    /// Router 配置
    #[serde(default)]
    pub router: Option<RouterConfig>,
}

/// OpenAI 配置
///
/// # 字段说明
///
/// - `base_url`: OpenAI API 地址（默认 `https://api.openai.com/v1`）
/// - `api_key`: OpenAI API Key
/// - `model`: 默认模型名称
///
/// # 示例
///
/// ```rust
/// use agentkit::config::OpenAiConfig;
///
/// let config = OpenAiConfig {
///     base_url: Some("https://api.openai.com/v1".to_string()),
///     api_key: Some("sk-...".to_string()),
///     model: Some("gpt-4o-mini".to_string()),
/// };
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpenAiConfig {
    /// API 地址
    #[serde(default)]
    pub base_url: Option<String>,
    /// API Key
    #[serde(default)]
    pub api_key: Option<String>,
    /// 模型名称
    #[serde(default)]
    pub model: Option<String>,
}

/// Ollama 配置
///
/// # 字段说明
///
/// - `base_url`: Ollama API 地址（默认 `http://localhost:11434`）
/// - `model`: 默认模型名称
///
/// # 示例
///
/// ```rust
/// use agentkit::config::OllamaConfig;
///
/// let config = OllamaConfig {
///     base_url: Some("http://localhost:11434".to_string()),
///     model: Some("llama3".to_string()),
/// };
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OllamaConfig {
    /// API 地址
    #[serde(default)]
    pub base_url: Option<String>,
    /// 模型名称
    #[serde(default)]
    pub model: Option<String>,
}

/// Router 配置
///
/// 用于配置多 Provider 路由。
///
/// # 字段说明
///
/// - `default_provider`: 默认 provider 名称
/// - `openai`: OpenAI provider 列表
/// - `ollama`: Ollama provider 列表
///
/// # 示例
///
/// ```rust
/// use agentkit::config::RouterConfig;
///
/// let config = RouterConfig {
///     default_provider: Some("ollama".to_string()),
///     openai: vec![],
///     ollama: vec![],
/// };
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RouterConfig {
    /// router 默认 provider 名称
    #[serde(default)]
    pub default_provider: Option<String>,

    /// OpenAI provider 列表
    #[serde(default)]
    pub openai: Vec<OpenAiConfig>,

    /// Ollama provider 列表
    #[serde(default)]
    pub ollama: Vec<OllamaConfig>,
}

impl AgentkitConfig {
    /// 从环境变量和配置文件加载配置
    ///
    /// # 加载顺序
    ///
    /// 1. 从文件加载 `profiles[profile]`
    /// 2. 应用环境变量覆盖（`AGENTKIT_*`）
    ///
    /// # 环境变量
    ///
    /// - `AGENTKIT_CONFIG`: 配置文件路径（可选）
    /// - `AGENTKIT_PROFILE`: profile 名称（可选，默认 `default`）
    /// - 其他 `AGENTKIT_*` env: 用于覆盖部分字段
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use agentkit::config::AgentkitConfig;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let profile = AgentkitConfig::load().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn load() -> Result<ProfileConfig, String> {
        let file_path = std::env::var("AGENTKIT_CONFIG").ok();
        let profile = std::env::var("AGENTKIT_PROFILE").unwrap_or_else(|_| "default".to_string());

        let mut base = ProfileConfig::default();

        if let Some(path) = file_path.as_deref() {
            base = Self::load_profile_from_file(path, &profile).await?;
        }

        Self::apply_env_overrides(&mut base);
        Ok(base)
    }

    /// 从配置文件读取某个 profile
    ///
    /// # 说明
    ///
    /// - 若 `profile` 不存在，则回退 `default`
    /// - 若仍不存在，则返回空配置（default）
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use agentkit::config::AgentkitConfig;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let profile = AgentkitConfig::load_profile_from_file("config.yaml", "prod").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn load_profile_from_file(
        path: impl AsRef<Path>,
        profile: &str,
    ) -> Result<ProfileConfig, String> {
        let loaded = Self::load_from_file(path).await?;
        if let Some(p) = loaded.profiles.get(profile).cloned() {
            Ok(p)
        } else if let Some(p) = loaded.profiles.get("default").cloned() {
            Ok(p)
        } else {
            Ok(ProfileConfig::default())
        }
    }

    /// 从文件加载配置
    async fn load_from_file(path: impl AsRef<Path>) -> Result<AgentkitConfig, String> {
        let path = path.as_ref();
        let raw = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| format!("read config file failed: {} (path={})", e, path.display()))?;

        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        if ext == "yaml" || ext == "yml" {
            serde_yaml::from_str::<AgentkitConfig>(&raw)
                .map_err(|e| format!("parse yaml config failed: {}", e))
        } else if ext == "toml" {
            toml::from_str::<AgentkitConfig>(&raw)
                .map_err(|e| format!("parse toml config failed: {}", e))
        } else {
            Err(format!(
                "unsupported config file extension: {} (path={})",
                ext,
                path.display()
            ))
        }
    }

    /// 应用环境变量覆盖
    fn apply_env_overrides(cfg: &mut ProfileConfig) {
        // 提供最小覆盖集
        if let Ok(kind) = std::env::var("AGENTKIT_PROVIDER_KIND") {
            cfg.provider.kind = kind;
        }
        if let Ok(model) = std::env::var("AGENTKIT_MODEL") {
            cfg.provider.model = Some(model);
        }

        if let Ok(v) = std::env::var("AGENTKIT_OPENAI_API_KEY") {
            cfg.provider
                .openai
                .get_or_insert_with(Default::default)
                .api_key = Some(v);
        }
        if let Ok(v) = std::env::var("AGENTKIT_OPENAI_BASE_URL") {
            cfg.provider
                .openai
                .get_or_insert_with(Default::default)
                .base_url = Some(v);
        }
        if let Ok(v) = std::env::var("AGENTKIT_OPENAI_MODEL") {
            cfg.provider
                .openai
                .get_or_insert_with(Default::default)
                .model = Some(v);
        }

        if let Ok(v) = std::env::var("AGENTKIT_OLLAMA_BASE_URL") {
            cfg.provider
                .ollama
                .get_or_insert_with(Default::default)
                .base_url = Some(v);
        }
        if let Ok(v) = std::env::var("AGENTKIT_OLLAMA_MODEL") {
            cfg.provider
                .ollama
                .get_or_insert_with(Default::default)
                .model = Some(v);
        }
    }

    /// 根据配置构建 provider
    ///
    /// # 说明
    ///
    /// - 该函数不会自动创建 tools/policies（这些通常更业务相关）
    /// - 如果配置不完整，会返回可读错误
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use agentkit::config::{AgentkitConfig, ProfileConfig};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let profile = AgentkitConfig::load().await?;
    /// let provider = AgentkitConfig::build_provider(&profile)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn build_provider(cfg: &ProfileConfig) -> Result<RouterProvider, ProviderError> {
        let kind = cfg.provider.kind.trim().to_ascii_lowercase();

        // 为了让上层统一使用 RouterProvider，这里无论 openai/ollama 都包一层 router
        match kind.as_str() {
            "openai" => {
                let openai_cfg = cfg.provider.openai.clone().unwrap_or_default();
                let api_key = openai_cfg
                    .api_key
                    .or_else(|| std::env::var("OPENAI_API_KEY").ok())
                    .ok_or_else(|| {
                        ProviderError::Message(
                            "缺少 OpenAI api_key（AGENTKIT_OPENAI_API_KEY 或 OPENAI_API_KEY）"
                                .to_string(),
                        )
                    })?;
                let base_url = openai_cfg
                    .base_url
                    .or_else(|| std::env::var("OPENAI_BASE_URL").ok())
                    .unwrap_or_else(|| "https://api.openai.com/v1".to_string());

                let model = openai_cfg.model.or_else(|| cfg.provider.model.clone());
                let mut p = OpenAiProvider::new(base_url, api_key);
                if let Some(m) = model {
                    p = p.with_default_model(m);
                }
                Ok(RouterProvider::new("openai").register("openai", p))
            }
            "ollama" => {
                let ollama_cfg = cfg.provider.ollama.clone().unwrap_or_default();
                let base_url = ollama_cfg
                    .base_url
                    .or_else(|| std::env::var("OLLAMA_BASE_URL").ok())
                    .unwrap_or_else(|| "http://localhost:11434".to_string());

                let model = ollama_cfg.model.or_else(|| cfg.provider.model.clone());
                let mut p = OllamaProvider::new(base_url);
                if let Some(m) = model {
                    p = p.with_default_model(m);
                }
                Ok(RouterProvider::new("ollama").register("ollama", p))
            }
            "router" => {
                let router_cfg = cfg.provider.router.clone().unwrap_or_default();
                let default_provider = router_cfg
                    .default_provider
                    .unwrap_or_else(|| "ollama".to_string());

                let mut router = RouterProvider::new(default_provider.clone());

                for c in router_cfg.ollama {
                    let base_url = c
                        .base_url
                        .or_else(|| std::env::var("OLLAMA_BASE_URL").ok())
                        .unwrap_or_else(|| "http://localhost:11434".to_string());
                    let mut p = OllamaProvider::new(base_url);
                    let model = c.model.or_else(|| cfg.provider.model.clone());
                    if let Some(m) = model {
                        p = p.with_default_model(m);
                    }
                    router = router.register("ollama", p);
                }

                for c in router_cfg.openai {
                    let api_key = c
                        .api_key
                        .or_else(|| std::env::var("OPENAI_API_KEY").ok())
                        .ok_or_else(|| {
                            ProviderError::Message(
                                "缺少 OpenAI api_key（OPENAI_API_KEY）".to_string(),
                            )
                        })?;
                    let base_url = c
                        .base_url
                        .or_else(|| std::env::var("OPENAI_BASE_URL").ok())
                        .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
                    let mut p = OpenAiProvider::new(base_url, api_key);
                    let model = c.model.or_else(|| cfg.provider.model.clone());
                    if let Some(m) = model {
                        p = p.with_default_model(m);
                    }
                    router = router.register("openai", p);
                }

                Ok(router)
            }
            _ => Err(ProviderError::Message(format!(
                "unknown provider kind: {} (expected: openai/ollama/router)",
                cfg.provider.kind
            ))),
        }
    }
}
