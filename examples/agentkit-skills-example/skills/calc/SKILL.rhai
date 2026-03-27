// Calc Skill - 计算器技能
// 演示如何调用工具执行计算

let expression = (ctx.input["expression"] ?? "1+1").to_string();

// 使用 cmd_exec 执行 bc 计算器（如果可用）
let cmd = "echo '" + expression + "' | bc";
let res = call_tool("cmd_exec", #{ command: cmd });

#{
    success: true,
    expression: expression,
    command: cmd,
    result: res
}
