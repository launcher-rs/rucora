# NVIDIA API 使用指南

## 配置说明

NVIDIA API 需要特殊的 model 名称格式，必须包含 `nvidia/` 前缀。

### 正确的配置方式

```rust
use agentkit::provider::OpenAiProvider;

// NVIDIA API 配置
let provider = OpenAiProvider::new(
    "https://integrate.api.nvidia.com/v1",
    "nvapi-YOUR_API_KEY"  // 替换为你的 API Key
)
.with_default_model("nvidia/nemotron-3-super-120b-a12b");  // 注意：需要 nvidia/ 前缀
```

### 常见的 Model 名称

| Model | 完整名称 |
|-------|---------|
| Nemotron-3 Super | `nvidia/nemotron-3-super-120b-a12b` |
| Llama 3.1 | `meta/llama-3.1-405b-instruct` |
| Llama 3 | `meta/llama3-70b-instruct` |
| Mistral Large | `mistralai/mistral-large` |
| Gemma 2 | `google/gemma-2-27b-it` |

## 使用 agentkit-deep-research 配置

运行配置向导：

```bash
cargo run -p agentkit-deep-research
```

配置步骤：

1. **选择 Provider**: 选择 `OpenAI`
2. **输入 API Key**: 输入你的 NVIDIA API Key（以 `nvapi-` 开头）
3. **输入模型名称**: 输入完整的 model 名称，如 `nvidia/nemotron-3-super-120b-a12b`
4. **输入 Base URL**: 输入 `https://integrate.api.nvidia.com/v1`

## 完整配置示例

```toml
# ~/.agentkit/config.toml
provider = "OpenAI"
api_key = "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"
model = "nvidia/nemotron-3-super-120b-a12b"
base_url = "https://integrate.api.nvidia.com/v1"
```

## 测试连接

```bash
cargo run -p agentkit-deep-research

# 输入研究主题
> 美国对伊朗的战争对中国有什么影响

# 如果配置正确，应该能看到研究正常进行
```

## 常见错误

### 404 Not Found

**错误信息**:
```
OpenAI 请求失败：status=404 Not Found
```

**原因**: Model 名称不正确，缺少前缀

**解决方案**: 使用完整的 model 名称，如 `nvidia/nemotron-3-super-120b-a12b`

### 401 Unauthorized

**错误信息**:
```
OpenAI 请求失败：status=401 Unauthorized
```

**原因**: API Key 无效或过期

**解决方案**: 检查 API Key 是否正确，确保以 `nvapi-` 开头

### 429 Rate Limit

**错误信息**:
```
OpenAI 请求失败：status=429 Too Many Requests
```

**原因**: 超出 API 配额

**解决方案**: 等待配额重置或升级套餐

## 获取 API Key

1. 访问 [NVIDIA API Catalog](https://build.nvidia.com/)
2. 注册/登录账号
3. 创建 API Key
4. 复制 Key 并保存到配置中

## 参考资料

- [NVIDIA API Catalog](https://build.nvidia.com/)
- [NVIDIA API 文档](https://docs.api.nvidia.com/)
- [支持的 Models](https://build.nvidia.com/models)
