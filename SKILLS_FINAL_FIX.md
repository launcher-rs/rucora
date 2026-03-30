# Skills 系统最终修复

## 修复的问题

### 1. 天气查询脚本输出为空 ✅
**问题**: Python 脚本没有正确处理输入和输出

**修复**:
- 添加输入为空检查
- 添加 JSON 解析错误处理
- 添加 User-Agent 头（wttr.in 需要）
- 确保输出正确的 JSON 格式

### 2. 系统提示词缺少引导 ✅
**问题**: LLM 直接调用技能工具，没有先读取技能说明

**修复**:
- 在系统提示词中添加详细的使用流程
- 提供完整的示例对话
- 强调"先读取技能说明，再调用技能工具"

## 修改的文件

### 1. weather/SKILL.py
```python
#!/usr/bin/env python3

def main():
    try:
        # 读取输入
        input_str = sys.stdin.read()
        if not input_str.strip():
            print(json.dumps({"success": False, "error": "输入为空"}))
            return
            
        input_data = json.loads(input_str)
        city = input_data.get("city", "Beijing")
        
        # 查询天气
        result = get_weather(city)
        
        # 输出结果
        print(json.dumps(result, ensure_ascii=False))
    except Exception as e:
        print(json.dumps({"success": False, "error": str(e)}))

def get_weather(city: str) -> dict:
    """使用 wttr.in 查询天气"""
    url = f"https://wttr.in/{city}?format=%C+%t"
    
    req = urllib.request.Request(
        url,
        headers={'User-Agent': 'curl/7.64.0'}  # 添加 User-Agent
    )
    with urllib.request.urlopen(req, timeout=10) as response:
        weather = response.read().decode('utf-8').strip()
    
    return {
        "success": True,
        "city": city,
        "weather": weather,
        "message": f"{city} 的天气：{weather}"
    }
```

### 2. main.rs 系统提示词
```rust
let system_prompt = format!(
    "你是智能助手，可以使用工具帮助用户解决问题。\n\n\
     可用技能：\n\
     - calculator: 执行数学表达式计算\n\
     - datetime: 获取当前日期和时间信息\n\
     - weather-query: 查询指定城市的当前天气情况\n\
     - read_skill: 读取技能的详细说明\n\n\
     使用流程：\n\
     1. 分析用户需求，确定需要哪个技能\n\
     2. 如果不清楚技能的使用方法，先调用 read_skill 工具读取该技能的详细说明\n\
     3. 根据技能说明，调用相应的技能工具\n\
     4. 将结果返回给用户\n\n\
     示例：\n\
     用户：北京天气怎么样？\n\
     助手思考：用户想查询天气，需要使用 weather-query 技能。让我先读取该技能的说明。\n\
     助手：[调用 read_skill 工具，参数：skill_name=\"weather-query\"]\n\
     工具返回：weather-query 需要 city 参数...\n\
     助手：[调用 weather-query 工具，参数：city=\"Beijing\"]\n\
     工具返回：北京的天气：Sunny +25°C\n\
     助手：北京现在晴朗，气温 25 摄氏度。\n\n\
     请记住：先读取技能说明，再调用技能工具！"
);
```

## 预期运行流程

```
用户：北京天气怎么样？

助手思考：
  用户想查询天气，需要使用 weather-query 技能。
  让我先读取该技能的说明。

助手：[调用 read_skill 工具]
  参数：{"skill_name": "weather-query"}

工具返回：
  weather-query 技能说明：
  - 需要 city 参数（城市名称）
  - 返回天气信息

助手：[调用 weather-query 工具]
  参数：{"city": "Beijing"}

工具返回：
  {"success": true, "weather": "Sunny +25°C"}

助手：北京现在晴朗，气温 25 摄氏度。
```

## 验证结果

```bash
cargo check --workspace
# ✅ Finished
```

## 下一步

- [ ] 运行完整测试
- [ ] 验证天气查询功能
- [ ] 验证 read_skill 引导流程
