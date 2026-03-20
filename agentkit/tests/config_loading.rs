use std::path::PathBuf;

use agentkit::config::AgentkitConfig;

fn unique_temp_file(prefix: &str, ext: &str) -> PathBuf {
    let mut base = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    base.push(format!("agentkit-{}-{}.{}", prefix, nanos, ext));
    base
}

#[tokio::test]
async fn config_should_load_profile_from_yaml() {
    let path = unique_temp_file("cfg", "yaml");

    let yaml = r#"
profiles:
  default:
    provider:
      kind: ollama
      model: llama3
  prod:
    provider:
      kind: openai
      model: gpt-4o-mini
"#;

    tokio::fs::write(&path, yaml).await.unwrap();

    let p = AgentkitConfig::load_profile_from_file(&path, "prod")
        .await
        .unwrap();
    assert_eq!(p.provider.kind, "openai");
    assert_eq!(p.provider.model.as_deref(), Some("gpt-4o-mini"));

    let _ = tokio::fs::remove_file(path).await;
}

#[tokio::test]
async fn config_should_load_profile_from_toml() {
    let path = unique_temp_file("cfg", "toml");

    let toml = r#"
[profiles.default.provider]
kind = "ollama"
model = "llama3"

[profiles.prod.provider]
kind = "openai"
model = "gpt-4o-mini"
"#;

    tokio::fs::write(&path, toml).await.unwrap();

    let p = AgentkitConfig::load_profile_from_file(&path, "default")
        .await
        .unwrap();
    assert_eq!(p.provider.kind, "ollama");
    assert_eq!(p.provider.model.as_deref(), Some("llama3"));

    let _ = tokio::fs::remove_file(path).await;
}
