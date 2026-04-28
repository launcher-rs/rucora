---
name: weather-query
description: 查询指定城市的当前天气情况
version: 1.0.0
author: rucora Team
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
      description: 城市名称（英文或拼音，如 Beijing、Shanghai）
      example: "Beijing"
    format:
      type: string
      description: 输出格式
      enum: [simple, full, json]
      default: simple
    unit:
      type: string
      description: 温度单位
      enum: [metric, uscs]
      default: metric
  required:
    - city
output_schema:
  type: object
  properties:
    success:
      type: boolean
      description: 是否成功
    city:
      type: string
      description: 城市名称
    temperature:
      type: string
      description: 温度
    condition:
      type: string
      description: 天气状况
    humidity:
      type: string
      description: 湿度
    wind:
      type: string
      description: 风速
    raw:
      type: string
      description: 原始响应（full 模式）
---

# 天气查询技能

查询指定城市的当前天气情况，支持多种输出格式。

## 功能说明

- ✅ 实时天气数据
- ✅ 支持全球主要城市
- ✅ 可选输出格式
- ✅ 无需 API Key

## 使用示例

### 基本用法

**输入：**
```json
{"city": "Beijing"}
```

**输出：**
```json
{
  "success": true,
  "city": "Beijing",
  "temperature": "+25°C",
  "condition": "晴朗"
}
```

### 完整格式

**输入：**
```json
{"city": "Shanghai", "format": "full"}
```

**输出：**
```json
{
  "success": true,
  "city": "Shanghai",
  "temperature": "+28°C",
  "condition": "多云",
  "humidity": "71%",
  "wind": "→ 12km/h",
  "raw": "Shanghai: ⛅ +28°C 71% → 12km/h"
}
```

### JSON 格式

**输入：**
```json
{"city": "Guangzhou", "format": "json"}
```

**输出：**
```json
{
  "success": true,
  "city": "Guangzhou",
  "temperature": 30,
  "condition": "Sunny",
  "humidity": 65,
  "wind_speed": 15
}
```

## 错误处理

### 城市不存在

**输入：**
```json
{"city": "InvalidCity123"}
```

**输出：**
```json
{
  "success": false,
  "error": "未找到该城市，请检查城市名称"
}
```

### 网络超时

**输出：**
```json
{
  "success": false,
  "error": "天气服务暂时不可用，请稍后重试"
}
```

## 实现说明

### 数据源

- **主要**: [wttr.in](https://wttr.in) - 免费天气查询服务
- **备用**: [Open-Meteo](https://open-meteo.com) - JSON 格式天气 API

### 格式说明

| 格式 | 说明 | 示例输出 |
|------|------|---------|
| `simple` | 简洁模式 | `Beijing: ⛅ +25°C` |
| `full` | 完整模式 | 包含湿度、风速等 |
| `json` | JSON 格式 | 结构化数据 |

### 城市名称格式

- ✅ 英文：`Beijing`, `Shanghai`, `New York`
- ✅ 拼音：`Beijing`, `Shanghai`
- ✅ 机场代码：`PEK`, `JFK`, `LAX`
- ❌ 中文：`北京`, `上海`（可能不识别）

## 相关技能

- [air-quality](../air-quality) - 空气质量查询
- [weather-forecast](../weather-forecast) - 天气预报
- [travel-advisor](../travel-advisor) - 旅行建议

## 依赖

- 网络连接
- `curl` 命令（可选，用于备用方案）
