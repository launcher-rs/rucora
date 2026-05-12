# rucora Prompt 方案设计（简化版）

## 核心思路

1. **内置经典模板** - 内置几种常用的 prompt，零配置直接用
2. **用户自定义** - 通过路径加载自己的 prompt 文件
3. **灵活指定** - 通过名称或路径选择使用哪个 prompt

---

## 设计方案

### 1. 简洁的 API 设计

```rust
use rucora::deep_research::{ResearchEngine, Strategy};

// 方式1: 使用内置默认 prompt
let engine = DefaultResearchEngine::new(StandardStrategy::new());

// 方式2: 指定内置 prompt 名称
let engine = DefaultResearchEngine::new(
    StandardStrategy::with_prompt("academic")
);

// 方式3: 使用自定义 prompt 文件
let engine = DefaultResearchEngine::new(
    StandardStrategy::with_prompt_file("config/my_prompt.toml")
);
```

### 2. 内置 Prompt 枚举

```rust
// rucora-core/src/research/prompt.rs

/// 内置 Prompt 模板
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BuiltInPrompt {
    /// 默认模板 - 平衡模式
    #[default]
    Default,
    /// 简洁模式 - 快速获取结果
    Concise,
    /// 详细模式 - 深度分析
    Detailed,
    /// 学术模式 - 注重引用和来源
    Academic,
    /// Agentic 模式 - 自主决策
    Agentic,
}

impl BuiltInPrompt {
    /// 获取 system prompt
    pub fn system_prompt(&self) -> &'static str {
        match self {
            BuiltInPrompt::Default => r#"
你是一名专业研究助手，负责对给定主题进行深入研究。
请遵循以下原则：
1. 基于可靠的信息源
2. 提供有据可查的分析
3. 保持客观中立
4. 结构化输出结果
"#,
            BuiltInPrompt::Concise => r#"
你是研究助手。请简洁地回答用户问题，突出重点。
"#,
            BuiltInPrompt::Detailed => r#"
你是资深研究专家。请进行深入全面的分析：
- 多角度审视问题
- 提供详实的案例和数据
- 识别潜在风险和机会
- 给出具体的建议
"#,
            BuiltInPrompt::Academic => r#"
你是学术研究助手。请以学术规范进行分析：
- 引用权威来源
- 标注参考出处
- 使用专业术语
- 保持客观严谨
"#,
            BuiltInPrompt::Agentic => r#"
你是自主研究Agent。你可以根据研究进展自主决策：
1. 决定搜索策略和方向
2. 判断信息是否足够
3. 选择下一步行动
4. 在适当时机终止研究
"#,
        }
    }

    /// 获取默认 user prompt 模板
    pub fn user_template(&self) -> &'static str {
        match self {
            BuiltInPrompt::Default => r#"
## 研究主题
{{topic}}

## 要求
请进行深入研究，提供有价值的分析。

## 输出
请按以下结构输出：
1. 核心发现
2. 详细分析
3. 参考来源
"#,
            BuiltInPrompt::Concise => r#"
主题：{{topic}}
简明扼要地回答。
"#,
            BuiltInPrompt::Detailed => r#"
## 研究主题
{{topic}}

## 研究背景
{{context}}

## 分析要求
请进行全面深入的分析，包括：
- 背景和现状
- 主要趋势和变化
- 关键因素分析
- 未来展望
- 建议和结论

## 参考信息
{{collected_info}}
"#,
            BuiltInPrompt::Academic => r#"
## 学术研究主题
{{topic}}

## 要求
1. 搜索学术文献和专业资料
2. 引用权威来源，标注出处
3. 使用学术语言和专业术语

## 参考
{{collected_info}}
"#,
            BuiltInPrompt::Agentic => r#"
## 研究任务
{{topic}}

## 当前研究状态
- 已收集信息：{{collected_info}}
- 研究轮次：{{iteration}}
- 置信度：{{confidence}}

请自主决定下一步行动。
"#,
        }
    }
}

/// Prompt 配置来源
#[derive(Debug, Clone)]
pub enum PromptSource {
    /// 内置 prompt
    BuiltIn(BuiltInPrompt),
    /// 自定义 prompt 文件路径
    CustomFile(PathBuf),
    /// 内联自定义 prompt 字符串
    Inline { system: String, user: String },
}

impl Default for PromptSource {
    fn default() -> Self {
        Self::BuiltIn(BuiltInPrompt::Default)
    }
}
```

### 3. 策略的 Prompt 配置

```rust
// rucora/src/deep_research/strategies.rs

/// 标准研究策略
pub struct StandardStrategy {
    config: ResearchConfig,
    prompt_source: PromptSource,
}

impl StandardStrategy {
    /// 使用默认 prompt
    pub fn new() -> Self {
        Self {
            config: ResearchConfig::default(),
            prompt_source: PromptSource::default(),
        }
    }

    /// 使用指定的内置 prompt
    pub fn with_prompt(mut self, name: &str) -> Self {
        let prompt = match name {
            "concise" => BuiltInPrompt::Concise,
            "detailed" => BuiltInPrompt::Detailed,
            "academic" => BuiltInPrompt::Academic,
            "agentic" => BuiltInPrompt::Agentic,
            _ => BuiltInPrompt::Default,
        };
        self.prompt_source = PromptSource::BuiltIn(prompt);
        self
    }

    /// 使用自定义 prompt 文件
    pub fn with_prompt_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.prompt_source = PromptSource::CustomFile(path.into());
        self
    }

    /// 使用内联自定义 prompt
    pub fn with_custom_prompt(mut self, system: &str, user: &str) -> Self {
        self.prompt_source = PromptSource::Inline {
            system: system.to_string(),
            user: user.to_string(),
        };
        self
    }

    /// 获取当前 system prompt
    fn get_system_prompt(&self) -> String {
        match &self.prompt_source {
            PromptSource::BuiltIn(p) => p.system_prompt().to_string(),
            PromptSource::CustomFile(path) => {
                // 从文件加载
                std::fs::read_to_string(path)
                    .ok()
                    .and_then(|c| parse_toml_prompt(&c))
                    .map(|(s, _)| s)
                    .unwrap_or_else(|| BuiltInPrompt::Default.system_prompt().to_string())
            }
            PromptSource::Inline { system, .. } => system.clone(),
        }
    }

    /// 渲染 user prompt
    fn render_user_prompt(&self, vars: &HashMap<String, String>) -> String {
        let template = match &self.prompt_source {
            PromptSource::BuiltIn(p) => p.user_template(),
            PromptSource::CustomFile(path) => {
                // 从文件加载模板
                return load_and_render_template(path, vars);
            }
            PromptSource::Inline { user, .. } => user,
        };

        // 简单变量替换
        let mut result = template.to_string();
        for (k, v) in vars {
            result = result.replace(&format!("{{{{{}}}}}", k), v);
        }
        result
    }
}

/// 解析 TOML prompt 文件
fn parse_toml_prompt(content: &str) -> Option<(String, String)> {
    // 简单解析 [system] content = "..." 和 [user] template = "..."
    // 实际使用 serde 或手动解析
    None // TODO: 实现
}
```

### 4. 使用示例

```rust
// 示例 1: 使用默认
let strategy = StandardStrategy::new();
let engine = DefaultResearchEngine::new(Box::new(strategy));

// 示例 2: 选择内置 academic 风格
let strategy = StandardStrategy::new().with_prompt("academic");
let engine = DefaultResearchEngine::new(Box::new(strategy));

// 示例 3: 使用自定义文件
let strategy = StandardStrategy::new()
    .with_prompt_file("config/my_research_prompt.toml");

// 示例 4: 内联自定义
let strategy = StandardStrategy::new()
    .with_custom_prompt(
        "你是金融分析师...",
        "分析 {{topic}} 的投资机会..."
    );
```

### 5. 自定义 Prompt 文件格式 (TOML)

```toml
# config/my_research_prompt.toml

[system]
content = """
你是金融领域专业分析师。
请从投资角度分析以下主题：
- 市场趋势
- 风险评估
- 投资建议
"""

[user]
template = """
## 分析主题
{{topic}}

## 投资相关分析
请从以下角度分析：
1. 市场现状
2. 发展趋势
3. 风险因素
4. 投资建议

## 已收集信息
{{collected_info}}
"""
```

---

## 总结

| 方式 | 用法 | 适用场景 |
|------|------|----------|
| **默认** | `StandardStrategy::new()` | 直接使用，无需配置 |
| **内置选择** | `.with_prompt("academic")` | 快速切换风格 |
| **自定义文件** | `.with_prompt_file("path/to/file.toml")` | 复杂自定义 |
| **内联** | `.with_custom_prompt(system, user)` | 临时测试 |

这个方案：
- **简单** - 核心 API 只有几个方法
- **灵活** - 支持内置/文件/内联三种方式
- **渐进** - 从默认开始，逐步自定义
- **熟悉** - 类似其他配置的方式