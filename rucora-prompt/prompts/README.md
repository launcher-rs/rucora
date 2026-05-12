// rucora-prompt prompts 配置文件
// 分类存放，每个目录代表一个类别

/*
目录结构：
prompts/
├── agent/
│   ├── simple.toml      # 简单问答
│   ├── chat.toml        # 对话
│   ├── tool.toml        # 工具调用
│   ├── react.toml       # ReAct 推理
│   └── reflect.toml     # 反思
├── tool/
│   ├── search.toml      # 搜索
│   ├── summarize.toml   # 总结
│   ├── translate.toml   # 翻译
│   ├── browse.toml      # 浏览
│   ├── code.toml       # 代码
│   └── file.toml        # 文件
├── research/
│   ├── default.toml     # 默认研究
│   ├── fast.toml        # 快速研究
│   ├── academic.toml    # 学术研究
│   └── agentic.toml     # 自主研究
└── filter/
    ├── classify.toml    # 分类
    └── extract.toml     # 提取
*/

// 使用方式：
// 1. 内置: prompt("agent/tool")
// 2. 文件: prompt("prompts/agent/tool.toml")
// 3. 自定义: prompt("path/to/custom.toml")