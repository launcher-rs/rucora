---
name: jsdate
description: JavaScript 日期技能 - 使用 JavaScript 处理日期和时间
input_schema:
  type: object
  properties:
    action:
      type: string
      description: 操作类型：now(当前时间), format(格式化), parse(解析)
      enum: [now, format, parse]
    date:
      type: string
      description: 日期字符串（format/parse 时需要）
    format:
      type: string
      description: 格式化模板
  required:
    - action
output_schema:
  type: object
  properties:
    success:
      type: boolean
    action:
      type: string
    result:
      type: string
---

这是一个使用 JavaScript 脚本的技能示例。

技能逻辑：
1. 接收操作类型和日期参数
2. 使用 JavaScript Date 对象处理
3. 返回结果

示例输入：
```json
{
  "action": "now"
}
```

示例输出：
```json
{
  "success": true,
  "action": "now",
  "result": "2024-01-01 12:00:00"
}
```
