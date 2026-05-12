# rucora Prompt 配置文件实现方案

## 设计目标

- 支持所有模块的 prompt 配置（Deep Research、Agent、工具调用）
- 使用 TOML 格式（Rust 原生支持）
- 运行时热加载，修改无需重启

## 目录结构

```
rucora/
├── config/
│   └── prompts/
│       ├── default.toml          # 默认配置
│       ├── research/
│       │   ├── standard.toml    # 标准研究策略
│       │   ├── fast.toml        # 快速研究策略
│       │   ├── agentic.toml     # Agentic 策略
│       │   └── academic.toml    # 学术研究策略
│       ├── agent/
│       │   ├── simple.toml      # SimpleAgent
│       │   ├── chat.toml        # ChatAgent
│       │   ├── tool.toml        # ToolAgent
│       │   └── react.toml       # ReActAgent
│       └── tools/
│           ├── search.toml      # 搜索工具
│           ├── browse.toml     # 浏览工具
│           └── analysis.toml   # 分析工具
```

## TOML 配置格式

### 示例：Deep Research 标准策略

```toml
# config/prompts/research/standard.toml

[system]
content = """
你是一名专业研究助手，负责对主题进行深度研究。
请遵循以下原则：
1. 只引用可靠来源
2. 提供具体的案例和数据
3. 保持客观中立的态度
"""

[user.templates.default]
template = """
## 研究主题
{{topic}}

## 研究要求
- 搜索最新的相关信息
- 识别关键趋势和变化
- 提供专业分析

## 输出格式
请按以下结构输出：
1. 核心发现
2. 详细分析
3. 参考来源
"""

[user.templates.iteration]
template = """
## 第 {{iteration}} 轮研究

## 前一轮结果
{{previous_summary}}

## 本轮任务
基于前一轮结果，进行更深入的分析：
- 补充新的信息和角度
- 验证之前结论的准确性
- 识别潜在的争议和不同观点

## 研究主题
{{topic}}
"""

[user.templates.completion]
template = """
研究已完成，请生成最终报告。

## 收集的信息
{{collected_info}}

## 已有的总结
{{existing_summary}}

请生成完整的分析报告。
"""
```

### 示例：ToolAgent

```toml
# config/prompts/agent/tool.toml

[system]
content = """
你是一个智能助手，可以通过工具来完成用户请求的任务。
在执行任务时：
1. 先理解用户意图
2. 合理选择和调用工具
3. 评估工具返回的结果
4. 根据结果决定下一步行动
"""

[tools.search]
description = "搜索网络信息"
input_schema = '''
{
  "type": "object",
  "properties": {
    "query": {"type": "string", "description": "搜索关键词"},
    "limit": {"type": "integer", "description": "返回结果数量"}
  },
  "required": ["query"]
}
'''

[tools.browse]
description = "浏览网页内容"
input_schema = '''
{
  "type": "object",
  "properties": {
    "url": {"type": "string", "description": "要浏览的网址"},
    "max_length": {"type": "integer", "description": "最大读取长度"}
  },
  "required": ["url"]
}
'''
```

## 核心实现

### 1. Prompt 配置结构

```rust
// rucora-core/src/config/prompt.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Prompt 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptConfig {
    /// 配置名称
    pub name: String,
    /// 配置描述
    pub description: Option<String>,
    /// System prompt
    pub system: PromptSection,
    /// User prompt 模板
    pub user: HashMap<String, PromptTemplate>,
}

/// Prompt 部分
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptSection {
    /// 角色描述
    pub role: Option<String>,
    /// Prompt 内容
    pub content: String,
}

/// User prompt 模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    /// 模板名称
    pub name: Option<String>,
    /// 模板描述
    pub description: Option<String>,
    /// 模板内容，支持 {{variable}} 变量替换
    pub template: String,
    /// 变量定义（用于验证和文档）
    pub variables: Option<HashMap<String, VariableSchema>>,
}

/// 变量 schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableSchema {
    /// 变量类型
    pub var_type: String,
    /// 变量描述
    pub description: Option<String>,
    /// 是否必需
    pub required: Option<bool>,
    /// 默认值
    pub default: Option<String>,
}
```

### 2. Prompt 加载器

```rust
// rucora-core/src/config/prompt_loader.rs

use crate::config::prompt::PromptConfig;
use std::path::Path;
use std::sync::RwLock;
use notify::{Watcher, RecommendedWatcher, RecursiveMode, Config};

/// Prompt 加载器，支持热加载
pub struct PromptLoader {
    configs: RwLock<HashMap<String, PromptConfig>>,
    path: PathBuf,
    watcher: Option<RecommendedWatcher>,
}

impl PromptLoader {
    /// 从目录加载所有配置
    pub fn load_dir(path: impl Into<PathBuf>) -> Result<Self, Box<dyn std::error::Error>> {
        let path = path.into();
        let mut loader = Self {
            configs: RwLock::new(HashMap::new()),
            path: path.clone(),
            watcher: None,
        };
        loader.reload_all()?;
        Ok(loader)
    }

    /// 重新加载所有配置
    pub fn reload_all(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut configs = self.configs.write().unwrap();
        configs.clear();

        // 遍历目录加载所有 .toml 文件
        for entry in walkdir::WalkDir::new(&self.path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "toml"))
        {
            let toml_content = std::fs::read_to_string(entry.path())?;
            let config: PromptConfig = toml::from_str(&toml_content)?;
            let name = entry.path()
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            configs.insert(name, config);
        }
        Ok(())
    }

    /// 启动热加载监控
    pub fn watch(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let path = self.path.clone();
        let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, _>| {
            if let Ok(event) = res {
                if event.kind.is_modify() {
                    // 触发重载
                    println!("[PromptLoader] 检测到配置变化，准备重载...");
                    // 实际实现中需要通知所有使用者
                }
            }
        })?;

        watcher.watch(&path, RecursiveMode::Recursive)?;
        self.watcher = Some(watcher);
        Ok(())
    }

    /// 获取配置
    pub fn get(&self, name: &str) -> Option<PromptConfig> {
        self.configs.read().unwrap().get(name).cloned()
    }

    /// 渲染模板
    pub fn render(&self, name: &str, template_name: &str, vars: &HashMap<String, String>) -> Option<String> {
        let config = self.get(name)?;
        let template = config.user.get(template_name)?.template.clone();
        
        // 简单的变量替换
        let mut result = template;
        for (key, value) in vars {
            result = result.replace(&format!("{{{{{}}}}}", key), value);
        }
        Some(result)
    }
}
```

### 3. 与 Deep Research 集成

```rust
// rucora/src/deep_research/strategies.rs

use rucora_core::config::PromptLoader;

/// 标准研究策略
pub struct StandardStrategy {
    config: ResearchConfig,
    prompt_loader: PromptLoader,
}

impl StandardStrategy {
    pub fn new() -> Self {
        let prompt_loader = PromptLoader::load_dir("config/prompts/research").unwrap();
        
        // 启动热加载
        if let Err(e) = prompt_loader.watch() {
            eprintln!("启动 prompt 热加载失败: {}", e);
        }
        
        Self {
            config: ResearchConfig::default(),
            prompt_loader,
        }
    }

    async fn search(&self, provider: &dyn LlmProvider, topic: &str, context: &mut ResearchContext) -> Result<StrategyResult, ResearchError> {
        // 获取 system prompt
        let system_prompt = self.prompt_loader
            .get("standard")
            .map(|c| c.system.content)
            .unwrap_or_else(|| self.default_system_prompt());

        // 渲染 user prompt
        let mut vars = HashMap::new();
        vars.insert("topic".to_string(), topic.to_string());
        vars.insert("collected_info".to_string(), context.collected_info.iter()
            .map(|i| i.content.clone())
            .collect::<Vec<_>>()
            .join("\n"));

        let user_prompt = self.prompt_loader
            .render("standard", "default", &vars)
            .unwrap_or_else(|| self.default_user_prompt(topic));

        // 构建消息
        let messages = vec![
            Message::system(system_prompt),
            Message::user(user_prompt),
        ];

        // 调用 LLM
        let response = provider.chat(&messages).await?;
        // ...
    }
}
```

### 4. 配置验证

```rust
/// 验证 prompt 配置的完整性
pub fn validate_prompt_config(config: &PromptConfig) -> Vec<String> {
    let mut errors = Vec::new();

    if config.system.content.is_empty() {
        errors.push("system prompt 不能为空".to_string());
    }

    if config.user.is_empty() {
        errors.push("至少需要一个 user prompt 模板".to_string());
    }

    // 检查变量是否存在
    for (name, template) in &config.user {
        if let Some(vars) = &template.variables {
            for var_name in extract_template_vars(&template.template) {
                if !vars.contains_key(&var_name) {
                    errors.push(format!("模板 '{}' 中变量 '{}' 未定义", name, var_name));
                }
            }
        }
    }

    errors
}

fn extract_template_vars(template: &str) -> Vec<String> {
    // 提取 {{variable}} 形式的变量名
    let mut vars = Vec::new();
    let re = regex::Regex::new(r"\{\{(\w+)\}\}").unwrap();
    for cap in re.captures_iter(template) {
        if let Some(var) = cap.get(1) {
            vars.push(var.as_str().to_string());
        }
    }
    vars
}
```

## 使用示例

### 1. 自定义 Deep Research Prompt

创建 `config/prompts/research/custom_strategy.toml`:

```toml
name = "custom_strategy"
description = "自定义研究策略"

[system]
content = """
你是一个领域专家，专注于 {{domain}} 领域的研究。
你的分析风格：
- 注重数据驱动的结论
- 偏好一手 источник (source)
- 善于发现趋势变化
"""

[user.templates.default]
description = "默认研究模板"
template = """
主题：{{topic}}
领域：{{domain}}

请进行 {{depth}} 程度的分析。
{{additional_instructions}}
"""
variables = { topic = { type = "string", required = true }, domain = { type = "string", required = false, default = "通用" }, depth = { type = "string", required = false, default = "标准" }, additional_instructions = { type = "string", required = false } }
```

### 2. 使用自定义配置

```rust
let config = PromptConfig::load("config/prompts/research/custom_strategy.toml")?;

let mut vars = HashMap::new();
vars.insert("topic".to_string(), "人工智能".to_string());
vars.insert("domain".to_string(), "技术".to_string());
vars.insert("depth".to_string(), "深入".to_string());

let user_prompt = config.render("default", &vars);
```

## 文件组织建议

```
rucora/
├── config/                    # 配置目录
│   ├── prompts/              # Prompt 配置
│   │   ├── default.toml      # 默认/示例配置
│   │   ├── research/         # Deep Research
│   │   ├── agent/            # Agent
│   │   └── tools/            # 工具
│   └── rucora.yaml           # 主配置（引用 prompt）
├── rucora/                   # 主 crate
└── rucora-core/             # 核心 crate
    └── src/
        └── config/           # 配置加载模块
```

## 下一步

1. 创建 `rucora-core/src/config/prompt.rs` - 定义配置结构
2. 创建 `rucora-core/src/config/prompt_loader.rs` - 实现加载器
3. 添加 `notify` 依赖用于热加载
4. 在 Deep Research 模块中集成
5. 添加示例配置文件