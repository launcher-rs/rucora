#!/usr/bin/env python3
"""
Skill: datetime
Description: 获取当前日期和时间信息
"""

import sys
import json
from datetime import datetime, timezone

def get_datetime(format_str: str = "%Y-%m-%d %H:%M:%S", timezone_str: str = "UTC") -> dict:
    """获取当前日期时间"""
    try:
        # 获取当前 UTC 时间
        now = datetime.now(timezone.utc)
        
        # 转换时区（简化处理，实际应该使用 pytz）
        if timezone_str != "UTC":
            # 这里简化处理，只支持 UTC
            pass
        
        # 格式化时间
        formatted = now.strftime(format_str)
        timestamp = int(now.timestamp())
        
        return {
            "success": True,
            "datetime": formatted,
            "timestamp": timestamp
        }
    except Exception as e:
        return {
            "success": False,
            "error": str(e)
        }

if __name__ == "__main__":
    try:
        input_data = json.loads(sys.stdin.read())
        format_str = input_data.get("format", "%Y-%m-%d %H:%M:%S")
        timezone_str = input_data.get("timezone", "UTC")
        
        result = get_datetime(format_str, timezone_str)
        print(json.dumps(result))
    except Exception as e:
        print(json.dumps({
            "success": False,
            "error": str(e)
        }))
        sys.exit(1)
