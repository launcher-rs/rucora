---
name: simple
description: 简单的问候技能 - 只使用 SKILL.md 定义，无需脚本
input_schema:
  type: object
  properties:
    name:
      type: string
      description: 要问候的人名
  required:
    - name
output_schema:
  type: object
  properties:
    greeting:
      type: string
    name:
      type: string
---

这是一个简单的技能示例，只使用 SKILL.md 文件定义。

技能逻辑：
1. 接收输入参数 name
2. 返回问候语

示例输入：
```json
{
  "name": "张三"
}
```

示例输出：
```json
{
  "greeting": "你好，张三！",
  "name": "张三"
}
```
