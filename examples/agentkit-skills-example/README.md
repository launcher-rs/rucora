# AgentKit Skills 示例

展示如何将 Skills 系统与 Agent 集成使用。

## 快速开始

```bash
# 1. 进入项目目录
cd D:\Desktop\ocr\agentkit

# 2. 设置环境变量
export OPENAI_API_KEY=sk-your-key
# 或使用 Ollama（推荐）
export OPENAI_BASE_URL=http://localhost:11434
export MODEL_NAME=qwen3.5:9b

# 3. 运行示例
cargo run -p agentkit-skills-example
```

## 功能演示

### 1. Skills 加载

从 `skills/` 目录自动加载所有 Skills：

```
1. 加载 Skills...
   Skills 目录：skills/
✓ 加载了 3 个 Skills

已加载的 Skills:
  - calculator: 执行数学表达式计算
  - datetime: 获取当前日期和时间信息
  - weather-query: 查询指定城市的当前天气情况
```

### 2. Skills 转 Tools

自动将 Skills 转换为 Agent 可以使用的 Tools：

```
3. 注册 Skills 为 Tools...
  ✓ 注册技能：calculator
  ✓ 注册技能：datetime
  ✓ 注册技能：weather-query
```

### 3. Agent 自动调用 Skills

Agent 根据用户问题自动选择合适的 Skill 执行：

```
测试：现在几点了？ (测试时间技能)
  助手：现在是 2024 年 03 月 27 日 15 时 30 分 45 秒。

测试：计算 10 + 20 * 3 (测试计算器技能)
  助手：计算结果是 70。

测试：北京天气怎么样？ (测试天气查询技能)
  助手：北京的天气：⛅ +25°C
```

## 项目结构

```
agentkit-skills-example/
├── Cargo.toml              # 项目配置
├── README.md               # 本文档
├── src/
│   └── main.rs             # 示例代码
└── skills/                 # Skills 目录
    ├── calculator/         # 计算器技能
    │   ├── SKILL.md        # 技能说明
    │   └── SKILL.py        # Python 实现
    ├── datetime/           # 日期时间技能
    │   ├── SKILL.md
    │   └── SKILL.py
    └── weather/            # 天气查询技能
        ├── SKILL.md
        └── SKILL.py
```

## Skill 格式

每个 Skill 包含：

### SKILL.md (必需)

```markdown
---
name: skill-name
description: 技能描述
version: 1.0.0
author: Author Name
tags:
  - tag1
  - tag2
input_schema:
  type: object
  properties:
    param1:
      type: string
      description: 参数说明
output_schema:
  type: object
  properties:
    result:
      type: string
---

# 技能详细说明

## 使用示例
...
```

### SKILL.py (可选)

```python
#!/usr/bin/env python3
"""
Skill: skill-name
Description: 技能描述
"""

import sys
import json

def main():
    input_data = json.loads(sys.stdin.read())
    # 处理输入
    result = {"success": True, "output": "result"}
    print(json.dumps(result))

if __name__ == "__main__":
    main()
```

## 运行输出示例

```
╔════════════════════════════════════════╗
║   AgentKit Skills 示例                ║
╚════════════════════════════════════════╝

1. 加载 Skills...
   Skills 目录：skills/
✓ 加载了 3 个 Skills

已加载的 Skills:
  - calculator: 执行数学表达式计算
  - datetime: 获取当前日期和时间信息
  - weather-query: 查询指定城市的当前天气情况

2. 创建 Skill Executor...
✓ Skill Executor 创建成功

3. 注册 Skills 为 Tools...
  ✓ 注册技能：calculator
  ✓ 注册技能：datetime
  ✓ 注册技能：weather-query

4. 创建带 Skills 的 Agent...
✓ Agent 创建成功

═══════════════════════════════════════
测试 Skills
═══════════════════════════════════════

测试：现在几点了？ (测试时间技能)
  助手：现在是 2024 年 03 月 27 日 15 时 30 分 45 秒。

测试：计算 10 + 20 * 3 (测试计算器技能)
  助手：计算结果是 70。

测试：北京天气怎么样？ (测试天气查询技能)
  助手：北京的天气：⛅ +25°C

═══════════════════════════════════════
示例完成！
═══════════════════════════════════════

📝 Skills 使用总结：

1. Skills 加载:
   - 使用 SkillLoader 从目录加载
   - 自动识别 SKILL.md 配置文件

2. Skills 转 Tools:
   - 使用 SkillExecutor 执行技能
   - 注册到 ToolRegistry

3. Agent 使用:
   - Agent 自动选择合适的技能
   - 支持多步推理和技能组合
```

## 依赖要求

### 必需
- Rust 1.70+
- OPENAI_API_KEY 环境变量
- 或 Ollama 服务

### 可选（用于执行 Python Skills）
- Python 3.8+
- 网络连接（天气查询需要）

## 环境变量

| 变量 | 说明 | 示例 |
|------|------|------|
| `OPENAI_API_KEY` | OpenAI API Key | `sk-xxx` |
| `OPENAI_BASE_URL` | OpenAI 兼容服务地址 | `http://localhost:11434` |
| `MODEL_NAME` | 使用的模型名称 | `qwen3.5:9b` |

## 添加新 Skill

1. 在 `skills/` 目录创建新文件夹
2. 添加 `SKILL.md` 定义技能
3. （可选）添加 `SKILL.py` 实现技能
4. 重新运行示例

示例：

```bash
mkdir skills/currency-converter
# 创建 SKILL.md 和 SKILL.py
```

## 故障排除

### Skills 未加载

**问题**: 显示 "⚠ 没有加载到 Skills"

**解决**:
```bash
# 检查 skills 目录是否存在
ls skills/

# 确保每个 skill 都有 SKILL.md
ls skills/*/SKILL.md
```

### Python Skills 执行失败

**问题**: "Permission denied" 或 "No such file or directory"

**解决**:
```bash
# 添加执行权限
chmod +x skills/*/SKILL.py

# 或确保 Python 已安装
python3 --version
```

### API 调用失败

**问题**: "OPENAI_API_KEY 未设置"

**解决**:
```bash
# 设置 API Key
export OPENAI_API_KEY=sk-your-key

# 或使用 Ollama
export OPENAI_BASE_URL=http://localhost:11434
```

## 相关文档

- [AgentKit 用户指南](../../docs/user_guide.md)
- [Skills 系统设计](../../docs/skills_design.md)
- [自定义 Skills](../../docs/custom_skills.md)

## License

MIT License - See [LICENSE](../../LICENSE) for details.
