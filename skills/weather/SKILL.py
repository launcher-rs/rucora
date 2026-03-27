#!/usr/bin/env python3
"""
Skill: weather-query
Description: 查询指定城市的当前天气情况
Version: 1.0.0
"""

import sys
import json
import urllib.request
import urllib.error
from typing import Dict, Any

# API 端点
WTTR_IN_URL = "https://wttr.in/{}?format={}"
OPEN_METEO_URL = "https://api.open-meteo.com/v1/forecast"

def main():
    """主函数"""
    try:
        # 1. 读取输入
        try:
            input_data = json.loads(sys.stdin.read())
        except json.JSONDecodeError as e:
            output_error(f"输入格式错误：{e}")
            sys.exit(1)
        
        # 2. 验证输入
        if "city" not in input_data:
            output_error("缺少必需字段：city")
            sys.exit(1)
        
        city = input_data.get("city", "")
        format_type = input_data.get("format", "simple")
        unit = input_data.get("unit", "metric")
        
        # 3. 执行天气查询
        try:
            if format_type == "json":
                result = get_weather_json(city, unit)
            else:
                result = get_weather_wttr(city, format_type, unit)
            
            # 4. 输出结果
            print(json.dumps({
                "success": True,
                **result
            }, ensure_ascii=False))
            
        except TimeoutError:
            output_error("天气服务响应超时")
            sys.exit(1)
        except ConnectionError:
            output_error("无法连接到天气服务")
            sys.exit(1)
        except Exception as e:
            output_error(str(e))
            sys.exit(1)
            
    except SystemExit:
        raise
    except Exception as e:
        output_error(f"未预期的错误：{e}")
        sys.exit(1)


def get_weather_wttr(city: str, format_type: str, unit: str) -> Dict[str, Any]:
    """
    使用 wttr.in 查询天气
    
    Args:
        city: 城市名称
        format_type: 输出格式 (simple/full)
        unit: 单位 (metric/uscs)
    
    Returns:
        天气数据字典
    """
    # 格式化城市名称
    city_formatted = city.replace(" ", "+")
    
    # 选择格式
    if format_type == "full":
        format_code = "%l:+%c+%t+%h+%w"
    else:
        format_code = "3"
    
    # 单位参数
    unit_param = "m" if unit == "metric" else "u"
    
    # 构建 URL
    url = WTTR_IN_URL.format(city_formatted, format_code)
    if unit_param:
        url += f"&{unit_param}"
    
    # 发送请求
    try:
        with urllib.request.urlopen(url, timeout=10) as response:
            raw_data = response.read().decode("utf-8").strip()
    except urllib.error.URLError as e:
        raise ConnectionError(f"天气服务不可用：{e}")
    except TimeoutError:
        raise TimeoutError("请求超时")
    
    # 解析结果
    if format_type == "full":
        # 解析完整格式：London: ⛅ +8°C 71% → 12km/h
        return parse_full_format(raw_data, city)
    else:
        # 简洁格式：London: ⛅ +8°C
        return parse_simple_format(raw_data, city)


def get_weather_json(city: str, unit: str) -> Dict[str, Any]:
    """
    使用 Open-Meteo 查询天气（JSON 格式）
    
    Args:
        city: 城市名称
        unit: 单位 (metric/uscs)
    
    Returns:
        天气数据字典
    """
    # 注意：Open-Meteo 需要经纬度，这里简化处理
    # 实际使用需要先将城市名转换为经纬度
    
    # 示例坐标（北京）
    coordinates = {
        "beijing": (39.9042, 116.4074),
        "shanghai": (31.2304, 121.4737),
        "guangzhou": (23.1291, 113.2644),
        "shenzhen": (22.5431, 114.0579),
    }
    
    city_lower = city.lower()
    if city_lower in coordinates:
        lat, lon = coordinates[city_lower]
    else:
        # 默认使用北京坐标
        lat, lon = 39.9042, 116.4074
    
    # 构建 URL
    url = (
        f"{OPEN_METEO_URL}?"
        f"latitude={lat}&longitude={lon}"
        f"&current_weather=true"
    )
    
    # 发送请求
    try:
        with urllib.request.urlopen(url, timeout=10) as response:
            data = json.loads(response.read().decode("utf-8"))
    except urllib.error.URLError as e:
        raise ConnectionError(f"天气服务不可用：{e}")
    except TimeoutError:
        raise TimeoutError("请求超时")
    
    # 解析 JSON 结果
    current = data.get("current_weather", {})
    
    return {
        "city": city,
        "temperature": current.get("temperature", "N/A"),
        "condition": get_weather_condition(current.get("weathercode", 0)),
        "wind_speed": current.get("windspeed", "N/A"),
        "wind_direction": current.get("winddirection", "N/A"),
    }


def parse_simple_format(raw_data: str, city: str) -> Dict[str, Any]:
    """解析简洁格式"""
    # 格式：London: ⛅ +8°C
    parts = raw_data.split(":")
    
    condition = "Unknown"
    temperature = "N/A"
    
    if len(parts) >= 2:
        # 提取天气符号和温度
        rest = ":".join(parts[1:]).strip()
        tokens = rest.split()
        
        if len(tokens) >= 2:
            condition = tokens[0]  # 天气符号
            temperature = " ".join(tokens[1:])  # 温度
    
    return {
        "city": city,
        "temperature": temperature,
        "condition": condition,
    }


def parse_full_format(raw_data: str, city: str) -> Dict[str, Any]:
    """解析完整格式"""
    # 格式：London: ⛅ +8°C 71% → 12km/h
    parts = raw_data.split(":")
    
    result = {
        "city": city,
        "temperature": "N/A",
        "condition": "Unknown",
        "humidity": "N/A",
        "wind": "N/A",
        "raw": raw_data,
    }
    
    if len(parts) >= 2:
        rest = ":".join(parts[1:]).strip()
        tokens = rest.split()
        
        if len(tokens) >= 2:
            result["condition"] = tokens[0]
            result["temperature"] = tokens[1]
        
        if len(tokens) >= 3:
            result["humidity"] = tokens[2]
        
        if len(tokens) >= 4:
            result["wind"] = " ".join(tokens[3:])
    
    return result


def get_weather_condition(code: int) -> str:
    """
    将天气代码转换为描述
    
    Args:
        code: WMO 天气代码
    
    Returns:
        天气描述
    """
    conditions = {
        0: "晴朗",
        1: "主要晴朗",
        2: "多云",
        3: "阴天",
        45: "雾",
        48: "雾凇",
        51: "小毛毛雨",
        53: "中毛毛雨",
        55: "大毛毛雨",
        61: "小雨",
        63: "中雨",
        65: "大雨",
        71: "小雪",
        73: "中雪",
        75: "大雪",
        95: "雷雨",
    }
    return conditions.get(code, f"未知 ({code})")


def output_error(message: str):
    """输出错误信息"""
    print(json.dumps({
        "success": False,
        "error": message
    }, ensure_ascii=False))


if __name__ == "__main__":
    main()
