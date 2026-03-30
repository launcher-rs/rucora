#!/usr/bin/env python3
"""
Skill: weather-query
Description: 查询指定城市的当前天气情况
"""

import sys
import json
import urllib.request

def main():
    try:
        # 读取输入
        input_str = sys.stdin.read()
        if not input_str.strip():
            print(json.dumps({
                "success": False,
                "error": "输入为空"
            }, ensure_ascii=False))
            return
            
        input_data = json.loads(input_str)
        city = input_data.get("city", "Beijing")
        
        # 查询天气
        result = get_weather(city)
        
        # 输出结果
        print(json.dumps(result, ensure_ascii=False))
    except json.JSONDecodeError as e:
        print(json.dumps({
            "success": False,
            "error": f"JSON 解析错误：{e}"
        }, ensure_ascii=False))
    except Exception as e:
        print(json.dumps({
            "success": False,
            "error": f"查询失败：{e}"
        }, ensure_ascii=False))

def get_weather(city: str) -> dict:
    """使用 wttr.in 查询天气"""
    url = f"https://wttr.in/{city}?format=%C+%t"
    
    try:
        req = urllib.request.Request(
            url,
            headers={'User-Agent': 'curl/7.64.0'}
        )
        with urllib.request.urlopen(req, timeout=10) as response:
            weather = response.read().decode('utf-8').strip()
        
        return {
            "success": True,
            "city": city,
            "weather": weather,
            "message": f"{city} 的天气：{weather}"
        }
    except Exception as e:
        return {
            "success": False,
            "error": f"查询失败：{e}"
        }

if __name__ == "__main__":
    main()
