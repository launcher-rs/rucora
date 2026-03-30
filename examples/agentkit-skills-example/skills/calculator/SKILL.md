---
name: calculator
description: 执行数学表达式计算
version: 1.0.0
author: AgentKit Team
tags:
  - utility
  - math
timeout: 10
input_schema:
  type: object
  properties:
    expression:
      type: string
      description: 数学表达式（支持 +, -, *, /, ^, sqrt 等）
  required:
    - expression
output_schema:
  type: object
  properties:
    success:
      type: boolean
    result:
      type: number
    expression:
      type: string
---

# 计算器技能

执行数学表达式计算，支持基本运算和科学计算。

## 使用示例

输入：
```json
{"expression": "10 + 20 * 3"}
```

输出：
```json
{
  "success": true,
  "result": 70,
  "expression": "10 + 20 * 3"
}
```

## 支持的运算

- 基本运算：`+`, `-`, `*`, `/`
- 幂运算：`^` 或 `**`
- 平方根：`sqrt()`
- 三角函数：`sin()`, `cos()`, `tan()`
- 对数：`log()`, `ln()`
- 常数：`pi`, `e`

## 安全说明

- 只允许数学表达式
- 不允许执行代码
- 表达式长度限制 1000 字符
