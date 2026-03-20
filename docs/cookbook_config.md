# Cookbook：统一配置系统（env + file + profile）

本项目在 `agentkit::config` 提供了一个最小可用的统一配置加载方式，目标是：

- 用一份配置文件描述 provider 选择与默认模型
- 支持按 profile（dev/prod）切换
- 支持用环境变量做覆盖（便于 CI / 生产环境）

## 1. 配置文件路径与 profile

- `AGENTKIT_CONFIG`：配置文件路径（支持 `.yaml/.yml/.toml`）
- `AGENTKIT_PROFILE`：profile 名称（默认 `default`）

加载逻辑：

- 优先读取 `profiles[AGENTKIT_PROFILE]`
- 若不存在则回退 `profiles.default`
- 若仍不存在则使用空配置（默认值）
- 然后再应用 `AGENTKIT_*` 环境变量覆盖

## 2. YAML 示例

```yaml
profiles:
  default:
    provider:
      kind: ollama
      model: llama3

  prod:
    provider:
      kind: openai
      model: gpt-4o-mini
```

## 3. TOML 示例

```toml
[profiles.default.provider]
kind = "ollama"
model = "llama3"

[profiles.prod.provider]
kind = "openai"
model = "gpt-4o-mini"
```

## 4. 环境变量覆盖（最小覆盖集）

- `AGENTKIT_PROVIDER_KIND`：`openai/ollama/router`
- `AGENTKIT_MODEL`

OpenAI：

- `AGENTKIT_OPENAI_API_KEY`（或使用 `OPENAI_API_KEY`）
- `AGENTKIT_OPENAI_BASE_URL`（或使用 `OPENAI_BASE_URL`）
- `AGENTKIT_OPENAI_MODEL`

Ollama：

- `AGENTKIT_OLLAMA_BASE_URL`（或使用 `OLLAMA_BASE_URL`）
- `AGENTKIT_OLLAMA_MODEL`

## 5. 示例：从配置构建 provider

在代码中使用：

- `AgentkitConfig::load()` 读取 profile 配置
- `AgentkitConfig::build_provider(&profile_cfg)` 构建 `RouterProvider`

接下来你可以将该 provider 交给 runtime 的 agent（例如 streaming agent）来运行。
