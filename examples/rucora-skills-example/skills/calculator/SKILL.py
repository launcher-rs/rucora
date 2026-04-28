#!/usr/bin/env python3
"""
Skill: calculator
Description: 执行数学表达式计算
"""

import sys
import json
import re
import math

def safe_eval(expression: str) -> float:
    """安全地计算数学表达式"""
    # 只允许数学字符
    allowed_pattern = r'^[\d+\-*/().\s^sqrtincolgpie]+$'
    if not re.match(allowed_pattern, expression):
        raise ValueError("无效的表达式")
    
    # 长度限制
    if len(expression) > 1000:
        raise ValueError("表达式过长")
    
    # 替换友好语法
    expr = expression.replace('^', '**').replace('sqrt', 'math.sqrt')
    expr = expr.replace('sin', 'math.sin').replace('cos', 'math.cos').replace('tan', 'math.tan')
    expr = expr.replace('log', 'math.log10').replace('ln', 'math.log')
    expr = expr.replace('pi', 'math.pi').replace('e', 'math.e')
    
    # 计算
    return eval(expr, {"__builtins__": {}, "math": math})

def calculate(expression: str) -> dict:
    """计算数学表达式"""
    try:
        result = safe_eval(expression)
        return {
            "success": True,
            "result": result,
            "expression": expression
        }
    except Exception as e:
        return {
            "success": False,
            "error": str(e)
        }

if __name__ == "__main__":
    try:
        input_data = json.loads(sys.stdin.read())
        expression = input_data.get("expression", "0")
        
        result = calculate(expression)
        print(json.dumps(result))
    except Exception as e:
        print(json.dumps({
            "success": False,
            "error": str(e)
        }))
        sys.exit(1)
