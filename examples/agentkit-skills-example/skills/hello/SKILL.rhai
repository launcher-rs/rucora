// Hello Skill - 简单的问候技能
// 演示如何使用 Rhai 脚本创建技能

let name = (ctx.input["name"] ?? "朋友").to_string();

#{
    success: true,
    greeting: "你好，" + name + "！欢迎使用 AgentKit Skills 系统！",
    name: name
}
