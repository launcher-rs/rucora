use std::collections::HashMap;
use std::path::Path;

use agentkit_core::error::ProviderError;
use serde::{Deserialize, Serialize};

use crate::provider::{OllamaProvider, OpenAiProvider, RouterProvider};

/// agentkit 的统一配置模型。
///
/// 设计目标：
/// - 支持从文件（YAML/TOML）与环境变量加载。
/// - 支持 profile（dev/prod）覆盖。
/// - 尽量只描述“怎么组装 agentkit”，不强绑定具体业务。
///
/// 约定：
/// - `AGENTKIT_CONFIG`：配置文件路径（可选）。
/// - `AGENTKIT_PROFILE`：profile 名称（可选，默认 `default`）。
/// - 其他 `AGENTKIT_*` env：用于覆盖部分字段（目前提供最小覆盖集）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentkitConfig {
    /// 选中的 profile（只在 load 时使用；序列化时可为空）。
    #[serde(default)]
    pub profile: Option<String>,

    /// 多 profile 配置。
    ///
    /// 文件格式示例：
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProfileConfig {
    #[serde(default)]
    pub provider: ProviderConfig,
}

/// Provider 配置。
///
/// 说明：当前先覆盖最常用的 provider 组装：
/// - `openai`
/// - `ollama`
/// - `router`（内部再引用 `openai/ollama` 子配置）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// provider 类型：openai/ollama/router。
    #[serde(default)]
    pub kind: String,

    /// 默认模型（会写入 request.model 或 provider 的默认模型）。
    #[serde(default)]
    pub model: Option<String>,

    /// OpenAI 配置。
    #[serde(default)]
    pub openai: Option<OpenAiConfig>,

    /// Ollama 配置。
    #[serde(default)]
    pub ollama: Option<OllamaConfig>,

    /// Router 配置。
    #[serde(default)]
    pub router: Option<RouterConfig>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpenAiConfig {
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OllamaConfig {
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RouterConfig {
    /// router 默认 provider 名称。
    #[serde(default)]
    pub default_provider: Option<String>,

    /// openai provider 的别名列表（可选）。
    #[serde(default)]
    pub openai: Vec<OpenAiConfig>,

    /// ollama provider 的别名列表（可选）。
    #[serde(default)]
    pub ollama: Vec<OllamaConfig>,
}

impl AgentkitConfig {
    /// 从 env + file 加载配置。
    ///
    /// 合并优先级（后者覆盖前者）：
    /// 1) 文件中的 `profiles[profile]`
    /// 2) env 覆盖（AGENTKIT_*）
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

    /// 从配置文件读取某个 profile。
    ///
    /// 说明：
    /// - 若 `profile` 不存在，则回退 `default`。
    /// - 若仍不存在，则返回空配置（default）。
    ///
    /// 该函数主要用于测试与复用（避免暴露内部解析实现细节）。
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

    fn apply_env_overrides(cfg: &mut ProfileConfig) {
        // 仅提供最小覆盖集，先把 DX 闭环跑通：
        // - provider kind/model
        // - openai api_key/base_url/model
        // - ollama base_url/model
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

    /// 根据配置构建 provider。
    ///
    /// 说明：
    /// - 该函数不会自动创建 tools/policies（这些通常更业务相关）。
    /// - 如果配置不完整，会返回可读错误。
    pub fn build_provider(cfg: &ProfileConfig) -> Result<RouterProvider, ProviderError> {
        let kind = cfg.provider.kind.trim().to_ascii_lowercase();

        // 为了让上层统一使用 RouterProvider，这里无论 openai/ollama 都包一层 router。
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
