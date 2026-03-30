---
name: datetime
description: 获取当前日期和时间信息
version: 1.0.0
author: AgentKit Team
tags:
  - utility
  - time
timeout: 10
input_schema:
  type: object
  properties:
    format:
      type: string
      description: 时间格式（strftime 格式）
      default: "%Y-%m-%d %H:%M:%S"
    timezone:
      type: string
      description: 时区（如 Asia/Shanghai）
      default: "UTC"
output_schema:
  type: object
  properties:
    success:
      type: boolean
    datetime:
      type: string
    timestamp:
      type: integer
---

# 日期时间技能

获取当前日期和时间信息，支持自定义格式和时区。

## 使用示例

输入：
```json
{"format": "%Y年%m月%d日 %H时%M分%S秒"}
```

输出：
```json
{
  "success": true,
  "datetime": "2024 年 03 月 27 日 15 时 30 分 45 秒",
  "timestamp": 1711554645
}
```

## 支持的格式

- `%Y` - 四位年份
- `%m` - 两位月份
- `%d` - 两位日期
- `%H` - 24 小时制小时
- `%M` - 分钟
- `%S` - 秒
- `%A` - 星期全称
- `%b` - 月份简称
