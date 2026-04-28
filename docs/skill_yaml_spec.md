# Skill 配置文件规范

> rucora Skills 系统配置文件标准格式

## 快速开始

### 最小配置

```yaml
skill:
  name: my-skill        # 技能名称（必需）
  description: 我的技能描述  # 技能描述（必需）
```

### 完整配置

```yaml
# 基本信息
skill:
  name: weather-query
  description: 查询指定城市的当前天气情况
  version: 1.0.0
  author: rucora Team
  tags: [weather, api, utility]

# 输入输出定义
input_schema:
  type: object
  properties:
    city:
      type: string
      description: 城市名称
  required: [city]

output_schema:
  type: object
  properties:
    success: { type: boolean }
    weather: { type: string }

# 执行配置
execution:
  timeout: 10           # 超时时间（秒）
  retries: 2            # 重试次数
  cache: false          # 是否缓存结果

# 触发器（用于自动触发）
triggers: [天气，天气查询，北京天气]

# 权限配置
permissions:
  network: true
  allowed_domains: [wttr.in]

# 依赖配置
dependencies:
  bins: [python3]
  env: [API_KEY]
```

## 配置字段详解

### 基本信息 (skill)

| 字段 | 类型 | 必需 | 说明 | 示例 |
|------|------|------|------|------|
| name | string | ✅ | 技能唯一标识 | `weather-query` |
| description | string | ✅ | 技能用途描述 | `查询城市天气` |
| version | string | ❌ | 语义化版本号 | `1.0.0` |
| author | string | ❌ | 作者信息 | `team@email.com` |
| tags | string[] | ❌ | 分类标签 | `[weather, api]` |

**命名规范**：
- `name`: 3-50 字符，小写字母 + 数字 + 连字符
- `description`: 10-500 字符，清晰说明用途
- `version`: 遵循 SemVer (主版本。次版本。修订号)

### 输入输出 Schema

使用 JSON Schema 格式定义：

```yaml
input_schema:
  type: object
  properties:
    city:
      type: string
      description: 城市名称
      examples: [Beijing, Shanghai]
    format:
      type: string
      enum: [simple, full]
      default: simple
  required: [city]

output_schema:
  type: object
  properties:
    success:
      type: boolean
    data:
      type: object
```

### 执行配置 (execution)

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| timeout | number | 30 | 执行超时（秒），范围 1-300 |
| retries | number | 0 | 失败重试次数 |
| cache | boolean | false | 是否缓存执行结果 |
| work_dir | string | - | 工作目录路径 |

### 触发器 (triggers)

用于技能自动触发的关键词：

```yaml
triggers:
  - 天气
  - 天气查询
  - 今天天气
  - 北京天气
```

### 权限配置 (permissions)

| 字段 | 类型 | 默认 | 说明 |
|------|------|------|------|
| network | boolean | false | 是否允许网络访问 |
| filesystem | boolean | false | 是否允许文件访问 |
| commands | string[] | [] | 允许执行的命令 |
| allowed_domains | string[] | [] | 域名白名单 |
| denied_domains | string[] | [] | 域名黑名单 |

示例：
```yaml
permissions:
  network: true
  filesystem: false
  commands: [curl, wget]
  allowed_domains:
    - wttr.in
    - api.example.com
```

### 依赖配置 (dependencies)

```yaml
dependencies:
  bins: [python3, node]           # 需要的二进制文件
  env: [API_KEY, BASE_URL]        # 需要的环境变量
  python_packages: [requests]     # 需要的 Python 包
```

## 场景示例

### 天气查询技能

```yaml
skill:
  name: weather-query
  description: 查询指定城市的当前天气情况
  version: 1.0.0
  tags: [weather, api]

input_schema:
  type: object
  properties:
    city:
      type: string
      description: 城市名称（英文或拼音）
    format:
      type: string
      enum: [simple, full, json]
      default: simple
  required: [city]

execution:
  timeout: 10

triggers: [天气，天气查询]

permissions:
  network: true
  allowed_domains: [wttr.in, open-meteo.com]

dependencies:
  bins: [python3]
```

### 计算器技能

```yaml
skill:
  name: calculator
  description: 执行数学表达式计算
  version: 1.0.0
  tags: [math, utility]

input_schema:
  type: object
  properties:
    expression:
      type: string
      description: 数学表达式
  required: [expression]

execution:
  timeout: 5

triggers: [计算，算一下]

permissions:
  network: false
  filesystem: false
```

### 新闻聚合技能

```yaml
skill:
  name: ai_news
  description: AI 新闻聚合 - 抓取科技媒体内容
  version: 1.0.0
  tags: [news, ai, web]

input_schema:
  type: object
  properties:
    limit:
      type: integer
      description: 返回数量
      default: 10
    source:
      type: string
      enum: [netease, 36kr, all]
      default: all

execution:
  timeout: 30
  retries: 1

triggers: [AI 新闻，AI 动态，AI 资讯]

permissions:
  network: true
  allowed_domains: [tech.163.com, 36kr.com]

dependencies:
  bins: [python3]
  python_packages: [requests, beautifulsoup4]
```

## 配置验证

### 自动验证

```rust
use rucora::skills::config::SkillConfig;

let config = SkillConfig::from_dir(&path)?;

// 验证配置
match config.validate() {
    Ok(()) => println!("✓ 配置有效"),
    Err(errors) => {
        for error in errors {
            eprintln!("✗ {}: {}", error.field, error.message);
        }
    }
}
```

### 验证规则

| 字段 | 规则 |
|------|------|
| name | 必需，3-50 字符，`[a-z0-9-]` |
| description | 必需，< 500 字符 |
| version | 语义化版本 (x.y.z) |
| timeout | 1-300 秒 |

## 按需加载

根据不同场景加载不同字段，优化性能：

```rust
use rucora::skills::config::{SkillConfig, ConfigLoadOptions};

// LLM 调用 - 只加载基本信息和 Schema
let options = ConfigLoadOptions::for_llm();

// 技能执行 - 加载执行配置和权限
let options = ConfigLoadOptions::for_execution();

// 技能搜索 - 只加载基本信息和触发器
let options = ConfigLoadOptions::for_search();

// 加载配置
let config = SkillConfig::from_dir_with_options(&path, &options)?;
```

### 性能对比

| 加载方式 | 加载字段 | 内存占用 |
|----------|----------|----------|
| for_llm() | name, description, schema | ~30% |
| for_execution() | basic, execution, permissions | ~60% |
| for_search() | name, description, triggers | ~20% |
| full() | 所有字段 | 100% |

## 配置合并

合并基础配置和自定义配置：

```rust
let base = SkillConfig::from_dir(&base_path)?;
let custom = SkillConfig::from_dir(&custom_path)?;

// 合并（当前配置优先）
let merged = base.merge(&custom);
```

**合并规则**：
- 基本信息：当前配置优先
- tags: 合并去重
- triggers: 合并去重
- metadata: 合并（当前优先）

## 便捷方法

```rust
// 检查标签
if config.has_tag("weather") {
    println!("这是天气技能");
}

// 匹配触发器
if config.matches_trigger("北京天气") {
    println!("匹配天气查询");
}

// 获取摘要
println!("{}", config.summary());
// 输出：weather-query v1.0.0 - 查询指定城市的当前天气情况

// JSON 序列化
let json = config.to_json()?;
let config = SkillConfig::from_json(&json)?;
```

## 最佳实践

### ✅ 推荐

```yaml
# 1. 始终包含必需字段
skill:
  name: my-skill
  description: 清晰的技能描述

# 2. 使用语义化版本
skill:
  version: 1.0.0

# 3. 添加 2-5 个标签
skill:
  tags: [weather, api, utility]

# 4. 定义输入 Schema
input_schema:
  type: object
  required: [city]

# 5. 设置超时时间
execution:
  timeout: 10

# 6. 权限最小化
permissions:
  network: true
  allowed_domains: [wttr.in]
```

### ❌ 避免

```yaml
# 1. 缺少必需字段
skill:
  name: my-skill
  # 缺少 description

# 2. 描述过长
skill:
  description: |  # 超过 500 字符
    这是一个非常长的描述...

# 3. 权限过大
permissions:
  network: true
  filesystem: true
  # 没有限制域名

# 4. 超时过长
execution:
  timeout: 600  # 超过 300 秒
```

## 常见问题

### Q: 配置文件放在哪里？

A: 放在技能目录中，与脚本文件同级：
```
skills/
└── weather-query/
    ├── skill.yaml      # 配置文件
    └── SKILL.py        # 脚本实现
```

### Q: 支持哪些配置格式？

A: 支持 YAML、TOML、JSON 三种格式，优先级：
`skill.yaml` > `skill.toml` > `skill.json`

### Q: 如何调试配置问题？

A: 使用验证方法查看详细错误：
```rust
if let Err(errors) = config.validate() {
    for error in errors {
        eprintln!("{}: {}", error.field, error.message);
    }
}
```

### Q: 配置加载失败怎么办？

A: 配置加载失败会回退到 SKILL.md 的 YAML Frontmatter：
```markdown
---
name: my-skill
description: 我的技能描述
---
```

## 相关文档

- [使用示例](skill_yaml_examples.md) - 各场景的详细示例
- [配置优化总结](../../SKILL_CONFIG_COMPLETE.md) - 完整优化说明
