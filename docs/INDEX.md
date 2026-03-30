# AgentKit 文档索引

> 最新文档列表 - 2026 年 3 月 30 日更新

## 📚 核心文档

### 入门指南
- ✅ [快速开始](quick_start.md) - 5 分钟上手
- ✅ [用户指南](user_guide.md) - 完整功能说明
- ✅ [常见问题](faq.md) - FAQ

### 示例代码
- ✅ [示例集合](cookbook.md) - 实际使用示例

### 架构设计
- ✅ [设计文档](design.md) - 系统设计理念
- ✅ [Agent 与 Runtime](agent_runtime_relationship.md) - 核心架构说明
- ✅ [快速参考](QUICK_REFERENCE.md) - API 快速查询

### 技能系统
- ✅ [Skill 配置规范](skill_yaml_spec.md) - 配置文件完整说明 **NEW**
- ✅ [Skill 配置示例](skill_yaml_examples.md) - 实际使用示例 **NEW**

### 开发指南
- ✅ [对话设计](conversation_guide.md) - 对话系统指南
- ✅ [Provider 设计](provider_default_model.md) - LLM Provider 实现说明
- ✅ [运行时设计](runtime_agent_model_design.md) - Runtime 实现细节

## 📝 已清理的过时文档

以下文档已删除（内容已过时或已合并）：

- ❌ ~~skills_specification.md~~ - 已合并到 skill_yaml_spec.md
- ❌ ~~MERGE_COMPLETE.md~~ - 临时文档
- ❌ ~~cookbook_config.md~~ - 已合并到 cookbook.md
- ❌ ~~SKILL_CONFIG_OPTIMIZATION.md~~ - 已合并到 skill_yaml_spec.md

## 🎯 推荐学习路径

### 新手
1. [快速开始](quick_start.md)
2. [用户指南](user_guide.md)
3. [示例集合](cookbook.md)
4. [常见问题](faq.md)

### 开发者
1. [设计文档](design.md)
2. [Agent 与 Runtime](agent_runtime_relationship.md)
3. [快速参考](QUICK_REFERENCE.md)
4. [Skill 配置规范](skill_yaml_spec.md)

### 技能开发者
1. [Skill 配置规范](skill_yaml_spec.md)
2. [Skill 配置示例](skill_yaml_examples.md)
3. [示例集合](cookbook.md)

## 📦 项目结构

```
agentkit/
├── docs/                      # 文档目录
│   ├── README.md              # 文档导航（本文件）
│   ├── quick_start.md         # 快速开始
│   ├── user_guide.md          # 用户指南
│   ├── cookbook.md            # 示例集合
│   ├── faq.md                 # 常见问题
│   ├── design.md              # 设计文档
│   ├── QUICK_REFERENCE.md     # 快速参考
│   ├── skill_yaml_spec.md     # Skill 配置规范 ✨
│   └── skill_yaml_examples.md # Skill 配置示例 ✨
├── agentkit/                  # 主库
├── agentkit-core/             # 核心抽象
├── agentkit-runtime/          # 运行时
└── examples/                  # 示例代码
```

## 🔄 文档更新记录

### 2026-03-30
- ✨ 新增 `skill_yaml_spec.md` - 完整的 Skill 配置规范
- ✨ 新增 `skill_yaml_examples.md` - Skill 配置使用示例
- 🗑️ 删除 `skills_specification.md` - 内容已过时
- 🗑️ 删除其他临时文档

### 2026-03-27
- 🔄 更新 Skills 相关文档

### 2026-03-24
- 🔄 更新 Provider 相关文档
- 🔄 更新对话设计文档

## 📞 获取帮助

- 📖 查看 [常见问题](faq.md)
- 💬 查看 [示例集合](cookbook.md)
- 🐛 提交 Issue
