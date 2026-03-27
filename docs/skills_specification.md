# AgentKit Skills 开发规范

> 基于 [AgentSkills.io](https://agentskills.io) 规范编写

## 第零部分：Skills 与 AI 运行流程

### 0.1 整体架构图

```
┌─────────────────────────────────────────────────────────────┐
│                      AI Agent                                │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐   │
│  │   用户输入   │ ──→ │  LLM 决策    │ ──→ │ Skill 调度器  │   │
│  └─────────────┘     └─────────────┘     └─────────────┘   │
│                           ↑                                       │
│                           │ 工具描述                              │
│                    ┌──────┴──────┐                               │
│                    │  SKILL.md   │                               │
│                    │  描述和     │                               │
│                    │  input_schema│                              │
│                    └─────────────┘                               │
└─────────────────────────────────────────────────────────────┘
                           │
                           │ 调用
                           ↓
┌─────────────────────────────────────────────────────────────┐
│                      Skill 执行层                             │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐   │
│  │  输入验证    │ ──→ │  脚本执行    │ ──→ │  输出处理    │   │
│  │  (Schema)   │     │ (Py/JS/Sh)  │     │  (Schema)   │   │
│  └─────────────┘     └─────────────┘     └─────────────┘   │
└─────────────────────────────────────────────────────────────┘
                           │
                           ↓
┌─────────────────────────────────────────────────────────────┐
│                      外部资源层                               │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐   │
│  │   APIs      │     │  文件系统    │     │  数据库      │   │
│  └─────────────┘     └─────────────┘     └─────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### 0.2 完整运行流程

#### 阶段 1: Skill 注册与发现

```
1. 扫描 Skills 目录
   ↓
2. 解析 SKILL.md
   ├─ 读取 name 和 description
   ├─ 解析 input_schema
   └─ 解析 output_schema
   ↓
3. 转换为 LLM 可理解的工具描述
   ↓
4. 注册到 Agent 工具列表
```

**示例流程：**

```python
# 1. 扫描目录
skills_dir = "skills/"
skill_folders = list_dirs(skills_dir)  # ['weather', 'calculator', 'search']

# 2. 解析每个 Skill
for folder in skill_folders:
    skill_md = parse_md(f"{folder}/SKILL.md")
    
    # 提取关键信息
    tool_definition = {
        "name": skill_md["name"],
        "description": skill_md["description"],
        "input_schema": skill_md["input_schema"]
    }
    
    # 3. 注册到 Agent
    agent.register_tool(tool_definition)
```

#### 阶段 2: AI 决策与调用

```
用户请求
   ↓
LLM 分析请求
   ├─ 识别意图
   ├─ 匹配可用 Skills
   └─ 生成调用参数
   ↓
调用选定的 Skill
   ↓
执行并返回结果
   ↓
LLM 整合结果
   ↓
回复用户
```

**示例对话流程：**

```
用户：北京今天天气怎么样？

[AI 思考过程]
1. 识别意图：查询天气
2. 匹配 Skills: weather-query
3. 提取参数：city="北京"
4. 生成调用：weather_query(city="北京")

[Skill 执行]
输入：{"city": "北京"}
执行：调用天气 API
输出：{"success": true, "temperature": 25, "condition": "晴朗"}

[AI 整合回复]
北京今天天气晴朗，气温 25 摄氏度。
```

#### 阶段 3: 数据流转

```
┌──────────────┐
│   用户输入    │
│ "北京天气"    │
└──────┬───────┘
       │
       ↓
┌──────────────┐
│  LLM 理解     │
│ 意图：查天气  │
│ 参数：北京    │
└──────┬───────┘
       │
       ↓ (转换为 Skill 输入)
┌──────────────┐
│ Skill 输入    │
│ {"city":     │
│  "北京"}     │
└──────┬───────┘
       │
       ↓ (执行脚本)
┌──────────────┐
│  脚本执行     │
│ 调用 API     │
│ 处理结果     │
└──────┬───────┘
       │
       ↓ (Skill 输出)
┌──────────────┐
│ Skill 输出    │
│ {"success":  │
│  true,       │
│  "temp": 25} │
└──────┬───────┘
       │
       ↓ (LLM 整合)
┌──────────────┐
│   AI 回复      │
│ "北京今天     │
│  25 度，晴朗"  │
└──────────────┘
```

### 0.3 LLM 如何理解 Skills

#### 工具描述格式

LLM 通过以下格式理解 Skills：

```json
{
  "name": "weather-query",
  "description": "查询指定城市的当前天气情况",
  "input_schema": {
    "type": "object",
    "properties": {
      "city": {
        "type": "string",
        "description": "城市名称"
      }
    },
    "required": ["city"]
  }
}
```

#### LLM 决策过程

```
1. 接收用户请求
   ↓
2. 分析请求内容
   - 识别关键词："天气"、"北京"
   ↓
3. 匹配可用工具
   - weather-query: 匹配度 95%
   - news-search: 匹配度 10%
   ↓
4. 选择最佳工具
   - 选择：weather-query
   ↓
5. 提取参数
   - city: "北京"
   ↓
6. 生成工具调用
   - weather_query(city="北京")
```

### 0.4 Skill 执行流程

#### 详细执行步骤

```
┌─────────────────────────────────────────────────────────────┐
│ 步骤 1: 输入验证                                             │
├─────────────────────────────────────────────────────────────┤
│ 1.1 检查 JSON 格式是否有效                                    │
│ 1.2 验证必需字段是否存在                                     │
│ 1.3 验证字段类型是否正确                                     │
│ 1.4 验证约束条件（长度、范围等）                              │
│                                                              │
│ 失败 → 返回错误：{"success": false, "error": "..."}         │
│ 成功 → 进入步骤 2                                            │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│ 步骤 2: 脚本执行                                             │
├─────────────────────────────────────────────────────────────┤
│ 2.1 启动脚本进程 (Python/Node/Bash)                         │
│ 2.2 通过 stdin 传递输入数据                                   │
│ 2.3 等待脚本执行完成                                         │
│ 2.4 监控超时（如果超过 timeout）                             │
│                                                              │
│ 失败 → 返回错误：执行失败/超时                               │
│ 成功 → 进入步骤 3                                            │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│ 步骤 3: 输出处理                                             │
├─────────────────────────────────────────────────────────────┤
│ 3.1 读取脚本 stdout 输出                                      │
│ 3.2 解析 JSON 结果                                            │
│ 3.3 验证输出格式是否符合 output_schema                       │
│ 3.4 处理错误情况（success=false）                            │
│                                                              │
│ 失败 → 返回错误：输出格式错误                                │
│ 成功 → 返回给 LLM                                            │
└─────────────────────────────────────────────────────────────┘
```

#### 执行时序图

```
用户     Agent      Skill 脚本     外部 API
 │          │            │            │
 │──请求───>│            │            │
 │          │            │            │
 │          │[1. 解析请求]│            │
 │          │[2. 匹配 Skill]           │
 │          │            │            │
 │          │──stdin────>│            │
 │          │  {"city":  │            │
 │          │   "北京"}  │            │
 │          │            │            │
 │          │            │[3. 验证输入]│
 │          │            │[4. 调用 API]│
 │          │            │───────────>│
 │          │            │            │
 │          │            │<───────────│
 │          │            │  {"temp":  │
 │          │            │   25}      │
 │          │            │            │
 │          │<─stdout────│            │
 │          │  {"success":│           │
 │          │   true}    │            │
 │          │            │            │
 │          │[5. 整合结果]│            │
 │<─回复────│            │            │
 │"北京 25 度，│            │            │
 │ 晴朗"     │            │            │
 │          │            │            │
```

### 0.5 错误处理流程

#### 错误类型与处理

```
┌─────────────────────────────────────────────────────────────┐
│ 错误类型 1: 输入验证错误                                     │
├─────────────────────────────────────────────────────────────┤
│ 原因：用户输入不符合 input_schema                           │
│ 处理：直接返回错误，不执行 Skill                            │
│ 示例：{"error": "缺少必需字段：city"}                       │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ 错误类型 2: 脚本执行错误                                     │
├─────────────────────────────────────────────────────────────┤
│ 原因：脚本运行失败（语法错误、依赖缺失等）                   │
│ 处理：捕获异常，返回错误信息                                │
│ 示例：{"error": "脚本执行失败：ModuleNotFoundError"}        │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ 错误类型 3: 外部资源错误                                     │
├─────────────────────────────────────────────────────────────┤
│ 原因：API 调用失败、网络超时等                               │
│ 处理：重试机制，返回友好错误                                │
│ 示例：{"error": "天气 API 暂时不可用"}                      │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ 错误类型 4: 输出验证错误                                     │
├─────────────────────────────────────────────────────────────┤
│ 原因：Skill 输出不符合 output_schema                        │
│ 处理：记录日志，返回通用错误                                │
│ 示例：{"error": "Skill 返回数据格式错误"}                   │
└─────────────────────────────────────────────────────────────┘
```

### 0.6 性能优化

#### 缓存策略

```
用户请求：北京天气
   ↓
检查缓存
   ├─ 缓存命中 → 直接返回（<100ms）
   └─ 缓存未命中 → 执行 Skill → 更新缓存
```

**缓存实现示例：**

```python
from functools import lru_cache
import time

@lru_cache(maxsize=100)
def get_weather_cached(city, timestamp_hour):
    """按小时缓存天气数据"""
    return call_weather_api(city)

# 每小时更新一次缓存
def get_weather(city):
    hour_key = int(time.time() / 3600)
    return get_weather_cached(city, hour_key)
```

#### 并发处理

```
并行执行多个 Skills:
┌──────────────┐
│ 用户请求      │
│ "北京和上海  │
│  的天气"     │
└──────┬───────┘
       │
       ↓ (并行调用)
┌──────┴───────┐
│              │
↓              ↓
weather_query  weather_query
city="北京"    city="上海"
│              │
↓              ↓
结果 1         结果 2
│              │
└──────┬───────┘
       │
       ↓ (整合结果)
┌──────────────┐
│ AI 回复        │
│ "北京 25 度，  │
│  上海 28 度"   │
└──────────────┘
```

## 第一部分：快速开始

### 1.1 什么是 Skills

Skills 是可重用的功能模块，使 AI Agent 能够执行特定任务。每个 Skill 包含：
- **定义文件** (SKILL.md) - 描述技能的功能、输入输出
- **实现文件** (可选) - Python/JavaScript/Shell 脚本实现

### 1.2 创建第一个 Skill

**目录结构：**
```
my-skill/
└── SKILL.md
```

**SKILL.md 内容：**
```markdown
---
name: hello
description: 简单的问候技能
---

# Hello Skill

返回友好的问候语。

## 输入

```json
{
  "name": "用户名称"
}
```

## 输出

```json
{
  "greeting": "你好，用户名称！"
}
```
```

### 1.3 Skill 生命周期

```
创建 → 测试 → 优化 → 发布 → 使用
  ↓      ↓      ↓      ↓      ↓
SKILL.md 本地测试 改进描述 版本管理 集成到 Agent
```

## 第二部分：SKILL.md 规范

### 2.1 文件结构

SKILL.md 由三部分组成：

```markdown
---
# 1. YAML Frontmatter (元数据)
name: skill-name
description: 简短描述
---

# 2. Markdown 文档
详细说明...

# 3. 示例代码块 (可选)
```json
{"input": "example"}
```
```

### 2.2 必需字段

| 字段 | 类型 | 说明 | 示例 |
|------|------|------|------|
| `name` | string | 技能唯一标识 | `weather-query` |
| `description` | string | 一句话描述功能 | `查询指定城市的天气` |

### 2.3 推荐字段

| 字段 | 类型 | 说明 |
|------|------|------|
| `version` | string | 版本号 (默认 1.0.0) |
| `author` | string | 作者信息 |
| `tags` | array | 分类标签 |
| `input_schema` | object | 输入 JSON Schema |
| `output_schema` | object | 输出 JSON Schema |
| `timeout` | integer | 超时时间 (秒) |

### 2.4 完整示例

```markdown
---
name: weather-query
description: 查询指定城市的当前天气情况
version: 1.0.0
author: Your Name
tags:
  - weather
  - api
  - data
timeout: 30
input_schema:
  type: object
  properties:
    city:
      type: string
      description: 城市名称
      example: "北京"
    unit:
      type: string
      enum: [celsius, fahrenheit]
      default: celsius
  required:
    - city
output_schema:
  type: object
  properties:
    success:
      type: boolean
    temperature:
      type: number
    condition:
      type: string
    city:
      type: string
---

# 天气查询技能

查询指定城市的当前天气情况，支持摄氏度和华氏度。

## 功能说明

- 实时天气数据
- 支持全球主要城市
- 可选温度单位

## 使用示例

### 基本用法

输入：
```json
{"city": "北京"}
```

输出：
```json
{
  "success": true,
  "temperature": 25,
  "condition": "晴朗",
  "city": "北京"
}
```

### 指定单位

输入：
```json
{"city": "上海", "unit": "fahrenheit"}
```

输出：
```json
{
  "success": true,
  "temperature": 77,
  "condition": "多云",
  "city": "上海"
}
```

## 错误处理

### 城市不存在

```json
{
  "success": false,
  "error": "未找到该城市"
}
```

### API 超时

```json
{
  "success": false,
  "error": "请求超时"
}
```

## 依赖

- 网络连接
- 天气 API 访问权限

## 相关技能

- [air-quality](./air-quality) - 空气质量查询
- [forecast](./forecast) - 天气预报
```

## 第三部分：编写优秀的描述

### 3.1 Description 最佳实践

**好的描述：**
```yaml
description: 查询指定城市的当前天气情况
```

**不好的描述：**
```yaml
description: 天气技能  # 太模糊
description: 这是一个可以用来查询天气的技能，用户可以通过这个技能获取...  # 太长
```

### 3.2 Description 指南

| 原则 | 说明 |
|------|------|
| **简洁** | 50 字以内 |
| **具体** | 说明具体功能 |
| **动作导向** | 使用动词开头 |
| **避免技术术语** | 使用用户语言 |

### 3.3 对比示例

| 技能 | 不好的描述 | 好的描述 |
|------|-----------|---------|
| 天气 | `天气相关` | `查询指定城市的当前天气` |
| 计算 | `计算东西` | `执行数学表达式计算` |
| 搜索 | `搜索功能` | `在互联网上搜索相关信息` |
| 邮件 | `邮件` | `发送电子邮件到指定地址` |

### 3.4 Name 命名规范

**规则：**
- 小写字母
- 使用连字符分隔单词
- 见名知意
- 避免缩写

```
✅ 推荐：weather-query, send-email, calculate-expression
❌ 避免：weather, email, calc, wq
```

## 第四部分：输入输出 Schema

### 4.1 Input Schema

**基本结构：**
```yaml
input_schema:
  type: object
  properties:
    字段名:
      type: 类型
      description: 描述
  required:
    - 必需字段
```

**完整示例：**
```yaml
input_schema:
  type: object
  properties:
    city:
      type: string
      description: 城市名称
      example: "北京"
    unit:
      type: string
      enum: [celsius, fahrenheit]
      default: celsius
      description: 温度单位
    include_forecast:
      type: boolean
      default: false
      description: 是否包含预报
  required:
    - city
```

### 4.2 Output Schema

**基本结构：**
```yaml
output_schema:
  type: object
  properties:
    success:
      type: boolean
    data:
      type: object
    error:
      type: string
```

**完整示例：**
```yaml
output_schema:
  type: object
  properties:
    success:
      type: boolean
      description: 是否成功
    temperature:
      type: number
      description: 当前温度
    condition:
      type: string
      description: 天气状况
    city:
      type: string
      description: 城市名称
    timestamp:
      type: string
      format: date-time
      description: 数据时间戳
```

### 4.3 类型参考

| 类型 | 说明 | 示例 |
|------|------|------|
| `string` | 字符串 | `"hello"` |
| `number` | 数字 | `42`, `3.14` |
| `integer` | 整数 | `42` |
| `boolean` | 布尔值 | `true`, `false` |
| `array` | 数组 | `[1, 2, 3]` |
| `object` | 对象 | `{"key": "value"}` |

### 4.4 约束条件

```yaml
properties:
  name:
    type: string
    minLength: 1
    maxLength: 100
    pattern: "^[a-zA-Z]+$"
  
  age:
    type: integer
    minimum: 0
    maximum: 150
  
  email:
    type: string
    format: email
  
  tags:
    type: array
    minItems: 1
    maxItems: 10
    items:
      type: string
```

## 第五部分：使用脚本实现

### 5.1 脚本类型

| 类型 | 文件 | 适用场景 |
|------|------|---------|
| Python | SKILL.py | 数据处理、ML、复杂逻辑 |
| JavaScript | SKILL.js | Web API、快速原型 |
| Shell | SKILL.sh | 系统操作、命令行工具 |

### 5.2 Python 脚本模板

```python
#!/usr/bin/env python3
"""
Skill: skill-name
Description: 技能描述
"""

import sys
import json

def main():
    # 1. 读取输入
    try:
        input_data = json.loads(sys.stdin.read())
    except json.JSONDecodeError as e:
        print(json.dumps({
            "success": False,
            "error": f"输入格式错误：{e}"
        }))
        sys.exit(1)
    
    # 2. 验证输入
    if "required_field" not in input_data:
        print(json.dumps({
            "success": False,
            "error": "缺少必需字段：required_field"
        }))
        sys.exit(1)
    
    # 3. 执行逻辑
    try:
        result = execute(input_data)
        print(json.dumps({
            "success": True,
            **result
        }))
    except Exception as e:
        print(json.dumps({
            "success": False,
            "error": str(e)
        }))
        sys.exit(1)

def execute(input_data):
    """
    执行技能逻辑
    
    Args:
        input_data: 输入数据字典
    
    Returns:
        结果字典
    """
    # 实现你的逻辑
    return {"result": "value"}

if __name__ == "__main__":
    main()
```

### 5.3 JavaScript 脚本模板

```javascript
#!/usr/bin/env node
/**
 * Skill: skill-name
 * Description: 技能描述
 */

async function main() {
    // 1. 读取输入
    let inputData;
    try {
        const input = await readStdin();
        inputData = JSON.parse(input);
    } catch (e) {
        console.log(JSON.stringify({
            success: false,
            error: `输入格式错误：${e.message}`
        }));
        process.exit(1);
    }
    
    // 2. 验证输入
    if (!inputData.required_field) {
        console.log(JSON.stringify({
            success: false,
            error: '缺少必需字段：required_field'
        }));
        process.exit(1);
    }
    
    // 3. 执行逻辑
    try {
        const result = await execute(inputData);
        console.log(JSON.stringify({
            success: true,
            ...result
        }));
    } catch (e) {
        console.log(JSON.stringify({
            success: false,
            error: e.message
        }));
        process.exit(1);
    }
}

async function execute(inputData) {
    // 实现你的逻辑
    return { result: 'value' };
}

function readStdin() {
    return new Promise((resolve) => {
        let data = '';
        process.stdin.on('data', chunk => data += chunk);
        process.stdin.on('end', () => resolve(data));
    });
}

main();
```

### 5.4 Shell 脚本模板

```bash
#!/bin/bash
# Skill: skill-name
# Description: 技能描述

# 1. 读取输入
input=$(cat)

# 2. 解析 JSON (需要 jq)
required_field=$(echo "$input" | jq -r '.required_field // empty')

# 3. 验证输入
if [ -z "$required_field" ]; then
    echo '{"success": false, "error": "缺少必需字段"}'
    exit 1
fi

# 4. 执行逻辑
result="value"

# 5. 输出结果
echo "{\"success\": true, \"result\": \"$result\"}"
exit 0
```

### 5.5 错误处理最佳实践

**Python 示例：**
```python
def execute(input_data):
    # 输入验证错误 - 返回 400 级别错误
    if not input_data.get("city"):
        raise ValueError("缺少必需字段：city")
    
    # 业务逻辑错误 - 返回具体错误信息
    try:
        result = call_weather_api(input_data["city"])
    except requests.Timeout:
        raise TimeoutError("天气 API 请求超时")
    except requests.ConnectionError:
        raise ConnectionError("无法连接到天气 API")
    
    return result
```

## 第六部分：评估 Skills

### 6.1 评估维度

| 维度 | 说明 | 评估方法 |
|------|------|---------|
| **准确性** | 输出是否正确 | 单元测试 |
| **可靠性** | 是否稳定运行 | 压力测试 |
| **性能** | 响应时间 | 基准测试 |
| **安全性** | 是否有漏洞 | 安全审计 |
| **可用性** | 是否易用 | 用户测试 |

### 6.2 测试用例设计

**测试文件结构：**
```
tests/
├── basic/
│   ├── test_01_valid_input.json
│   ├── test_01_expected_output.json
│   ├── test_02_missing_field.json
│   └── test_02_expected_output.json
├── edge_cases/
│   ├── test_01_empty_string.json
│   └── test_01_expected_output.json
└── performance/
    └── test_01_large_input.json
```

**测试输入示例：**
```json
{
  "name": "有效输入",
  "description": "测试正常输入情况",
  "input": {
    "city": "北京"
  },
  "expected": {
    "success": true,
    "city": "北京"
  }
}
```

### 6.3 评估清单

**发布前检查：**

- [ ] SKILL.md 格式正确
- [ ] name 和 description 已填写
- [ ] input_schema 定义完整
- [ ] output_schema 定义完整
- [ ] 包含使用示例
- [ ] 错误处理完善
- [ ] 通过所有测试用例
- [ ] 文档清晰易懂

### 6.4 性能基准

| 指标 | 目标 | 测量方法 |
|------|------|---------|
| 响应时间 | < 3 秒 | 平均响应时间 |
| 成功率 | > 99% | 1000 次请求 |
| 并发支持 | > 10 QPS | 并发测试 |

## 第七部分：最佳实践

### 7.1 设计原则

**单一职责：**
```
✅ 推荐：weather-query (只查询天气)
❌ 避免：weather-and-news (混合多个功能)
```

**明确边界：**
```
✅ 推荐：明确说明技能能做什么和不能做什么
❌ 避免：模糊的功能描述
```

**错误友好：**
```
✅ 推荐：{"error": "城市不存在，请检查城市名称"}
❌ 避免：{"error": "API 错误"}
```

### 7.2 命名规范

**Skill 名称：**
```bash
# 使用小写和连字符
✅ weather-query
✅ send-email
❌ WeatherQuery
❌ send_email
```

**字段名称：**
```yaml
# 使用有意义的名称
✅ city: "北京"
✅ temperature: 25
❌ c: "北京"
❌ t: 25
```

### 7.3 文档规范

**结构清晰：**
```markdown
# 技能名称

简短描述

## 功能说明

详细介绍

## 使用示例

输入输出示例

## 错误处理

可能的错误情况

## 依赖

外部依赖说明
```

**示例完整：**
```markdown
## 使用示例

### 基本用法

输入：
```json
{"city": "北京"}
```

输出：
```json
{"success": true, "temperature": 25}
```

### 高级用法

输入：
```json
{"city": "北京", "unit": "fahrenheit"}
```

输出：
```json
{"success": true, "temperature": 77, "unit": "fahrenheit"}
```
```

### 7.4 安全实践

**输入验证：**
```python
def validate_input(data):
    # 类型检查
    if not isinstance(data.get("city"), str):
        raise ValueError("city 必须是字符串")
    
    # 长度限制
    if len(data["city"]) > 100:
        raise ValueError("city 长度不能超过 100")
    
    # 字符限制
    if not re.match(r'^[\u4e00-\u9fa5a-zA-Z]+$', data["city"]):
        raise ValueError("city 只能包含中英文字母")
```

**敏感信息：**
```python
# ❌ 错误做法
API_KEY = "sk-1234567890"

# ✅ 正确做法
import os
API_KEY = os.environ.get("WEATHER_API_KEY")
```

**命令注入防护：**
```bash
# ❌ 危险做法
curl "https://api.example.com/$USER_INPUT"

# ✅ 安全做法
SANITIZED_INPUT=$(echo "$USER_INPUT" | sed 's/[^a-zA-Z0-9]//g')
curl "https://api.example.com/$SANITIZED_INPUT"
```

### 7.5 性能优化

**缓存结果：**
```python
from functools import lru_cache

@lru_cache(maxsize=100)
def get_weather(city):
    return call_weather_api(city)
```

**超时控制：**
```python
import signal

def timeout_handler(signum, frame):
    raise TimeoutError("技能执行超时")

signal.signal(signal.SIGALRM, timeout_handler)
signal.alarm(30)  # 30 秒超时

try:
    result = execute()
    signal.alarm(0)
except TimeoutError:
    return {"success": False, "error": "执行超时"}
```

## 第八部分：常见问题

### Q1: 如何处理可选参数？

```yaml
input_schema:
  type: object
  properties:
    city:
      type: string
      description: 城市名称
    unit:
      type: string
      default: celsius  # 设置默认值
      description: 温度单位
  required:
    - city  # 只标记必需字段
```

### Q2: 如何返回复杂数据结构？

```yaml
output_schema:
  type: object
  properties:
    success:
      type: boolean
    data:
      type: object
      properties:
        current:
          type: object
          properties:
            temperature:
              type: number
            condition:
              type: string
        forecast:
          type: array
          items:
            type: object
            properties:
              date:
                type: string
              temperature:
                type: number
```

### Q3: 如何处理长时间运行的任务？

```yaml
# SKILL.md
---
name: long-task
timeout: 300  # 5 分钟超时
---
```

```python
# 实现进度返回
def execute(input_data):
    for i, step in enumerate(steps):
        yield {
            "progress": (i + 1) / len(steps),
            "current_step": step
        }
```

### Q4: 如何调试 Skill？

**本地测试：**
```bash
# 直接运行
echo '{"city": "北京"}' | python skills/weather/SKILL.py

# 查看输出
echo '{"city": "北京"}' | python skills/weather/SKILL.py 2>&1 | jq
```

**日志记录：**
```python
import logging

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

def execute(input_data):
    logger.info(f"执行技能，输入：{input_data}")
    result = do_something()
    logger.info(f"执行结果：{result}")
    return result
```

## 第九部分：参考资料

- [AgentSkills 官方规范](https://agentskills.io/specification)
- [Skill 创建快速开始](https://agentskills.io/skill-creation/quickstart)
- [最佳实践](https://agentskills.io/skill-creation/best-practices)
- [优化描述](https://agentskills.io/skill-creation/optimizing-descriptions)
- [评估 Skills](https://agentskills.io/skill-creation/evaluating-skills)
- [使用脚本](https://agentskills.io/skill-creation/using-scripts)
- [客户端实现](https://agentskills.io/client-implementation/adding-skills-support)

## 附录：完整示例

### 天气查询 Skill

```markdown
---
name: weather-query
description: 查询指定城市的当前天气情况
version: 1.0.0
author: Your Name
tags:
  - weather
  - api
timeout: 30
input_schema:
  type: object
  properties:
    city:
      type: string
      description: 城市名称
      example: "北京"
    unit:
      type: string
      enum: [celsius, fahrenheit]
      default: celsius
  required:
    - city
output_schema:
  type: object
  properties:
    success:
      type: boolean
    temperature:
      type: number
    condition:
      type: string
    city:
      type: string
---

# 天气查询技能

查询指定城市的当前天气情况。

## 使用示例

输入：
```json
{"city": "北京", "unit": "celsius"}
```

输出：
```json
{
  "success": true,
  "temperature": 25,
  "condition": "晴朗",
  "city": "北京"
}
```

## 错误情况

- 城市不存在
- API 超时
- 网络连接失败

## 依赖

- 网络连接
- 天气 API 访问权限
```

### Python 实现

```python
#!/usr/bin/env python3
"""
Skill: weather-query
Description: 查询指定城市的当前天气情况
"""

import sys
import json
import requests

API_URL = "https://api.weather.com"

def main():
    try:
        # 读取输入
        input_data = json.loads(sys.stdin.read())
        
        # 验证输入
        if "city" not in input_data:
            raise ValueError("缺少必需字段：city")
        
        # 执行查询
        result = get_weather(
            input_data["city"],
            input_data.get("unit", "celsius")
        )
        
        # 输出结果
        print(json.dumps(result))
        
    except Exception as e:
        print(json.dumps({
            "success": False,
            "error": str(e)
        }))
        sys.exit(1)

def get_weather(city, unit):
    try:
        response = requests.get(
            f"{API_URL}/{city}",
            params={"unit": unit},
            timeout=30
        )
        response.raise_for_status()
        data = response.json()
        
        return {
            "success": True,
            "temperature": data["temperature"],
            "condition": data["condition"],
            "city": city
        }
    except requests.Timeout:
        raise TimeoutError("天气 API 请求超时")
    except requests.ConnectionError:
        raise ConnectionError("无法连接到天气 API")

if __name__ == "__main__":
    main()
```
