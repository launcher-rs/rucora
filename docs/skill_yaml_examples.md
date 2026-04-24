# Skill 配置使用示例

> 实际场景中的配置文件使用方法

## 目录

- [加载配置](#加载配置)
- [LLM 调用场景](#llm-调用场景)
- [技能执行场景](#技能执行场景)
- [技能搜索场景](#技能搜索场景)
- [配置验证](#配置验证)
- [配置合并](#配置合并)
- [性能优化](#性能优化)

## 加载配置

### 基础加载

```rust
use agentkit::skills::config::SkillConfig;
use std::path::Path;

// 从目录加载配置
let config = SkillConfig::from_dir(
    &Path::new("skills/weather-query")
).expect("Failed to load config");

println!("技能：{}", config.skill.name);
println!("描述：{}", config.skill.description);
```

### 按需加载

```rust
use agentkit::skills::config::{SkillConfig, ConfigLoadOptions};

// LLM 调用场景 - 只加载必要字段
let options = ConfigLoadOptions::for_llm();
let config = SkillConfig::from_dir_with_options(&path, &options)?;

// 技能执行场景 - 加载执行配置
let options = ConfigLoadOptions::for_execution();
let config = SkillConfig::from_dir_with_options(&path, &options)?;

// 技能搜索场景 - 只加载搜索信息
let options = ConfigLoadOptions::for_search();
let config = SkillConfig::from_dir_with_options(&path, &options)?;
```

## LLM 调用场景

构建工具描述供 LLM 使用：

```rust
use agentkit::skills::config::{SkillConfig, ConfigLoadOptions};
use serde_json::json;

fn build_tool_description(path: &Path) -> Option<serde_json::Value> {
    let options = ConfigLoadOptions::for_llm();
    let config = SkillConfig::from_dir_with_options(path, &options)?;
    
    Some(json!({
        "name": config.skill.name,
        "description": config.skill.description,
        "parameters": config.input_schema
    }))
}

// 使用示例
if let Some(tool_desc) = build_tool_description(&Path::new("skills/weather-query")) {
    println!("工具描述：{}", tool_desc);
}
```

### 批量构建工具列表

```rust
fn build_all_tools(skills_dir: &Path) -> Vec<serde_json::Value> {
    let options = ConfigLoadOptions::for_llm();
    let mut tools = Vec::new();
    
    for entry in std::fs::read_dir(skills_dir).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(config) = SkillConfig::from_dir_with_options(&path, &options) {
                tools.push(json!({
                    "name": config.skill.name,
                    "description": config.skill.description,
                    "parameters": config.input_schema
                }));
            }
        }
    }
    
    tools
}
```

## 技能执行场景

### 检查权限

```rust
use agentkit::skills::config::{SkillConfig, ConfigLoadOptions};

fn check_permissions(path: &Path) -> Result<(), String> {
    let options = ConfigLoadOptions::for_execution();
    let config = SkillConfig::from_dir_with_options(path, &options)
        .ok_or("Failed to load config")?;
    
    // 检查网络权限
    if let Some(perm) = &config.permissions {
        if perm.network {
            if !perm.allowed_domains.is_empty() {
                println!("网络访问限制：{:?}", perm.allowed_domains);
            } else {
                return Err("网络访问未限制域名".to_string());
            }
        }
        
        // 检查命令权限
        if !perm.commands.is_empty() {
            println!("允许的命令：{:?}", perm.commands);
        }
    }
    
    Ok(())
}
```

### 获取执行配置

```rust
fn get_execution_config(path: &Path) -> Result<(u64, u32), String> {
    let options = ConfigLoadOptions::for_execution();
    let config = SkillConfig::from_dir_with_options(path, &options)?;
    
    let timeout = config.execution
        .as_ref()
        .map(|e| e.timeout)
        .unwrap_or(30);
    
    let retries = config.execution
        .as_ref()
        .map(|e| e.retries)
        .unwrap_or(0);
    
    Ok((timeout, retries))
}
```

## 技能搜索场景

### 关键词搜索

```rust
use agentkit::skills::config::{SkillConfig, ConfigLoadOptions};

fn search_skills(skills_dir: &Path, keyword: &str) -> Vec<String> {
    let options = ConfigLoadOptions::for_search();
    let mut results = Vec::new();
    
    for entry in std::fs::read_dir(skills_dir).ok().into_iter().flatten() {
        let entry = entry.ok()?;
        let path = entry.path();
        
        if path.is_dir() {
            if let Some(config) = SkillConfig::from_dir_with_options(&path, &options) {
                // 搜索描述
                if config.skill.description.contains(keyword) {
                    results.push(config.skill.name);
                    continue;
                }
                
                // 搜索标签
                if config.skill.tags.iter().any(|t| t.contains(keyword)) {
                    results.push(config.skill.name);
                    continue;
                }
                
                // 搜索触发器
                if config.matches_trigger(keyword) {
                    results.push(config.skill.name);
                }
            }
        }
    }
    
    results
}

// 使用示例
let weather_skills = search_skills(&Path::new("skills"), "天气");
println!("找到天气相关技能：{:?}", weather_skills);
```

### 按标签筛选

```rust
fn filter_by_tag(skills_dir: &Path, tag: &str) -> Vec<String> {
    let options = ConfigLoadOptions::for_search();
    let mut results = Vec::new();
    
    for entry in std::fs::read_dir(skills_dir).ok().into_iter().flatten() {
        let path = entry.ok()?.path();
        if let Some(config) = SkillConfig::from_dir_with_options(&path, &options) {
            if config.has_tag(tag) {
                results.push(config.skill.name);
            }
        }
    }
    
    results
}

// 使用示例
let api_skills = filter_by_tag(&Path::new("skills"), "api");
println!("API 技能：{:?}", api_skills);
```

## 配置验证

### 基础验证

```rust
use agentkit::skills::config::SkillConfig;

fn validate_config(path: &Path) -> Result<(), String> {
    let config = SkillConfig::from_dir(path)
        .ok_or("Failed to load config")?;
    
    config.validate()
        .map_err(|errors| {
            errors.iter()
                .map(|e| format!("{}: {}", e.field, e.message))
                .collect::<Vec<_>>()
                .join("\n")
        })?;
    
    println!("✓ 配置有效");
    Ok(())
}
```

### 自定义验证

```rust
fn validate_with_custom_rules(config: &SkillConfig) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    
    // 检查是否有标签
    if config.skill.tags.is_empty() {
        errors.push("至少需要一个标签".to_string());
    }
    
    // 检查触发器
    if config.triggers.is_empty() {
        errors.push("至少需要一个触发器".to_string());
    }
    
    // 检查网络权限
    if let Some(perm) = &config.permissions {
        if perm.network && perm.allowed_domains.is_empty() {
            errors.push("网络访问需要设置域名白名单".to_string());
        }
    }
    
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
```

## 配置合并

### 基础配置 + 自定义配置

```rust
use agentkit::skills::config::SkillConfig;

fn merge_configs(base_path: &Path, custom_path: &Path) -> Option<SkillConfig> {
    let base = SkillConfig::from_dir(base_path)?;
    let custom = SkillConfig::from_dir(custom_path)?;
    
    // 合并配置（custom 优先）
    Some(base.merge(&custom))
}

// 使用示例
let base = Path::new("skills/base/weather-query");
let custom = Path::new("skills/custom/weather-query");

if let Some(merged) = merge_configs(&base, &custom) {
    println!("合并后的技能：{}", merged.skill.name);
    println!("合并后的标签：{:?}", merged.skill.tags);
}
```

### 批量合并配置

```rust
fn merge_all_skills(base_dir: &Path, custom_dir: &Path) -> Vec<SkillConfig> {
    let mut skills = Vec::new();
    
    // 加载所有基础技能
    for entry in std::fs::read_dir(base_dir).ok().into_iter().flatten() {
        let base_path = entry.ok()?.path();
        let custom_path = custom_dir.join(base_path.file_name()?);
        
        let base = SkillConfig::from_dir(&base_path)?;
        
        if custom_path.exists() {
            if let Some(custom) = SkillConfig::from_dir(&custom_path) {
                skills.push(base.merge(&custom));
            } else {
                skills.push(base);
            }
        } else {
            skills.push(base);
        }
    }
    
    skills
}
```

## 性能优化

### 批量加载（只加载必要字段）

```rust
use agentkit::skills::config::{SkillConfig, ConfigLoadOptions};
use std::time::Instant;

fn benchmark_loading(skills_dir: &Path) {
    // 完整加载
    let start = Instant::now();
    let options = ConfigLoadOptions::full();
    let mut full_count = 0;
    for entry in std::fs::read_dir(skills_dir).unwrap() {
        let path = entry.unwrap().path();
        if SkillConfig::from_dir_with_options(&path, &options).is_some() {
            full_count += 1;
        }
    }
    println!("完整加载：{:?} ({} skills)", start.elapsed(), full_count);
    
    // 搜索加载
    let start = Instant::now();
    let options = ConfigLoadOptions::for_search();
    let mut search_count = 0;
    for entry in std::fs::read_dir(skills_dir).unwrap() {
        let path = entry.unwrap().path();
        if SkillConfig::from_dir_with_options(&path, &options).is_some() {
            search_count += 1;
        }
    }
    println!("搜索加载：{:?} ({} skills)", start.elapsed(), search_count);
}
```

### 配置缓存

```rust
use std::collections::HashMap;
use agentkit::skills::config::{SkillConfig, ConfigLoadOptions};

struct SkillConfigCache {
    cache: HashMap<String, SkillConfig>,
    options: ConfigLoadOptions,
}

impl SkillConfigCache {
    fn new(options: ConfigLoadOptions) -> Self {
        Self {
            cache: HashMap::new(),
            options,
        }
    }
    
    fn get_or_load(&mut self, name: &str, dir: &Path) -> Option<&SkillConfig> {
        use std::collections::hash_map::Entry;
        
        match self.cache.entry(name.to_string()) {
            Entry::Occupied(entry) => Some(entry.into_mut()),
            Entry::Vacant(entry) => {
                let path = dir.join(name);
                let config = SkillConfig::from_dir_with_options(&path, &self.options)?;
                Some(entry.insert(config))
            }
        }
    }
}

// 使用示例
let mut cache = SkillConfigCache::new(ConfigLoadOptions::for_search());
if let Some(config) = cache.get_or_load("weather-query", &Path::new("skills")) {
    println!("技能：{}", config.skill.name);
}
```

## 完整示例

### 技能管理器

```rust
use agentkit::skills::config::{SkillConfig, ConfigLoadOptions};
use std::path::{Path, PathBuf};

pub struct SkillManager {
    skills_dir: PathBuf,
    configs: HashMap<String, SkillConfig>,
}

impl SkillManager {
    pub fn new(skills_dir: PathBuf) -> Self {
        Self {
            skills_dir,
            configs: HashMap::new(),
        }
    }
    
    /// 加载所有技能配置
    pub fn load_all(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let options = ConfigLoadOptions::for_registration();
        
        for entry in std::fs::read_dir(&self.skills_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                if let Some(config) = SkillConfig::from_dir_with_options(&path, &options) {
                    // 验证配置
                    if let Err(errors) = config.validate() {
                        eprintln!("配置验证失败 {}: {:?}", path.display(), errors);
                        continue;
                    }
                    
                    self.configs.insert(config.skill.name.clone(), config);
                }
            }
        }
        
        Ok(())
    }
    
    /// 获取技能配置
    pub fn get(&self, name: &str) -> Option<&SkillConfig> {
        self.configs.get(name)
    }
    
    /// 搜索技能
    pub fn search(&self, keyword: &str) -> Vec<&SkillConfig> {
        self.configs.values()
            .filter(|config| {
                config.skill.description.contains(keyword)
                    || config.matches_trigger(keyword)
                    || config.skill.tags.iter().any(|t| t.contains(keyword))
            })
            .collect()
    }
    
    /// 按标签筛选
    pub fn filter_by_tag(&self, tag: &str) -> Vec<&SkillConfig> {
        self.configs.values()
            .filter(|config| config.has_tag(tag))
            .collect()
    }
    
    /// 构建 LLM 工具列表
    pub fn build_tool_list(&self) -> Vec<serde_json::Value> {
        use serde_json::json;
        
        self.configs.values()
            .map(|config| {
                json!({
                    "name": config.skill.name,
                    "description": config.skill.description,
                    "parameters": config.input_schema
                })
            })
            .collect()
    }
}

// 使用示例
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut manager = SkillManager::new(PathBuf::from("skills"));
    manager.load_all()?;
    
    // 搜索技能
    let weather_skills = manager.search("天气");
    println!("天气技能：{} 个", weather_skills.len());
    
    // 按标签筛选
    let api_skills = manager.filter_by_tag("api");
    println!("API 技能：{} 个", api_skills.len());
    
    // 构建工具列表
    let tools = manager.build_tool_list();
    println!("工具总数：{} 个", tools.len());
    
    Ok(())
}
```

## 相关文档

- [配置规范](skill_yaml_spec.md) - 完整的字段说明
- [Skill 配置规范](skill_yaml_spec.md) - 完整格式说明
