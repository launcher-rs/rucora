# Claude Code 源码分析报告

## 可借鉴到 AgentKit 的设计思路与方法

> 基于 Claude Code v2.1.88 源码分析  
> 分析日期：2026 年 4 月 1 日  
> 目标：提取可集成到 AgentKit Rust 项目的设计思路

---

## 目录

1. [执行摘要](#执行摘要)
2. [核心架构设计](#核心架构设计)
3. [工具系统设计](#工具系统设计)
4. [权限管理系统](#权限管理系统)
5. [钩子 (Hook) 系统](#钩子系统)
6. [任务与状态管理](#任务与状态管理)
7. [多 Agent 协作](#多 agent 协作)
8. [技能 (Skill) 系统](#技能系统)
9. [记忆与上下文管理](#记忆与上下文管理)
10. [遥测与可观测性](#遥测与可观测性)
11. [推荐集成优先级](#推荐集成优先级)

---

## 执行摘要

### 关键发现

| 领域 | Claude Code 设计 | AgentKit 现状 | 集成建议 |
|------|-----------------|--------------|---------|
| **工具系统** | 40+ 工具，统一接口，支持进度追踪 | 12+ 工具，基础接口 | ⭐⭐⭐ 高优先级 |
| **权限管理** | 6 层渐进式权限，规则引擎 | 基础策略检查 | ⭐⭐⭐ 高优先级 |
| **钩子系统** | 异步钩子注册表，超时处理 | 中间件系统 | ⭐⭐ 中优先级 |
| **任务管理** | 任务类型化，状态机，输出持久化 | 基础执行循环 | ⭐⭐⭐ 高优先级 |
| **多 Agent** | 协调器模式，队友管理，权限同步 | Supervisor Agent | ⭐⭐ 中优先级 |
| **技能系统** | 技能发现，加载，版本管理 | 基础技能系统 | ⭐⭐ 中优先级 |
| **记忆系统** | 记忆形状遥测，版本化 | 基础记忆存储 | ⭐ 低优先级 |
| **遥测** | 全链路追踪，BigQuery 导出 | 基础日志 | ⭐ 低优先级 |

### 推荐集成路线图

```
Phase 1 (立即): 任务状态机 + 工具进度追踪
Phase 2 (短期): 权限规则引擎 + 渐进式权限
Phase 3 (中期): 钩子系统 + 多 Agent 协调器
Phase 4 (长期): 技能市场 + 遥测系统
```

---

## 核心架构设计

### 1. 渐进式架构 (Progressive Harness)

**设计思路**：
Claude Code 采用 12 层渐进式机制，从简单对话到完全自主 Agent：

```
Level 1: 基础对话 (Chat)
Level 2: 工具调用 (Tools)
Level 3: 权限检查 (Permissions)
Level 4: 钩子执行 (Hooks)
Level 5: 任务管理 (Tasks)
Level 6: 子 Agent (Sub-agents)
Level 7: 协调器 (Coordinator)
Level 8: 记忆系统 (Memory)
Level 9: 技能系统 (Skills)
Level 10: 工作流 (Workflows)
Level 11: 团队协作 (Team/Swarm)
Level 12: 完全自主 (KAIROS)
```

**AgentKit 集成建议**：

```rust
// 当前 AgentKit 主要在 Level 2-3
// 建议逐步添加更高级功能

pub enum AgentCapabilityLevel {
    Chat,              // Level 1
    ToolUse,           // Level 2
    PermissionCheck,   // Level 3
    HookExecution,     // Level 4
    TaskManagement,    // Level 5
    SubAgent,          // Level 6
    Coordinator,       // Level 7
    Memory,            // Level 8
    Skills,            // Level 9
    Workflows,         // Level 10
    TeamCollaboration, // Level 11
    Autonomous,        // Level 12
}

impl ToolAgent {
    pub fn capability_level(&self) -> AgentCapabilityLevel {
        // 根据配置的工具和特性返回当前能力等级
        AgentCapabilityLevel::ToolUse
    }
}
```

### 2. 查询引擎架构 (Query Engine)

**设计思路**：
```
用户输入 → 查询引擎 → 消息处理 → 工具调用 → 结果聚合
                ↓
            状态管理
                ↓
            钩子触发
```

**AgentKit 集成建议**：

```rust
// 当前 DefaultExecution 可以扩展为 QueryEngine

pub struct QueryEngine<P: LlmProvider> {
    provider: Arc<P>,
    state: Arc<RwLock<EngineState>>,
    hooks: Arc<HookRegistry>,
    tools: Arc<ToolRegistry>,
}

impl<P: LlmProvider> QueryEngine<P> {
    pub async fn process(&self, input: AgentInput) -> Result<AgentOutput> {
        // 1. 预处理钩子
        self.hooks.run_pre_hooks(&input).await?;
        
        // 2. 执行核心逻辑
        let output = self.execute_core(input).await?;
        
        // 3. 后处理钩子
        self.hooks.run_post_hooks(&output).await?;
        
        Ok(output)
    }
}
```

---

## 工具系统设计

### 1. 统一工具接口

**Claude Code 设计**：
```typescript
interface Tool {
  name: string;
  description: string;
  inputSchema: ToolInputJSONSchema;
  execute(context: ToolContext): Promise<ToolResult>;
  getPermissionRequirements(): PermissionRequirement[];
  getProgressData?(toolUseId: string): ToolProgressData;
}
```

**关键特性**：
- 所有工具统一接口
- 内置权限需求声明
- 进度追踪支持
- 结果类型化

**AgentKit 集成建议**：

```rust
// 扩展当前 Tool trait

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> Option<&str>;
    fn categories(&self) -> &[ToolCategory];
    fn input_schema(&self) -> Value;
    
    // 新增：权限需求声明
    fn permission_requirements(&self) -> Vec<PermissionRequirement> {
        Vec::new() // 默认无特殊权限需求
    }
    
    // 新增：进度追踪
    fn get_progress(&self, _tool_use_id: &str) -> Option<ToolProgress> {
        None // 默认不支持进度追踪
    }
    
    async fn call(&self, input: Value) -> Result<Value, ToolError>;
}

// 新增类型
pub struct PermissionRequirement {
    pub resource_type: String,      // "file", "shell", "network"
    pub action: String,             // "read", "write", "execute"
    pub scope: PermissionScope,     // Global, PathPattern, UrlPattern
}

pub struct ToolProgress {
    pub current_step: u32,
    pub total_steps: u32,
    pub description: String,
    pub metadata: Option<Value>,
}
```

### 2. 工具进度追踪

**Claude Code 设计**：
```typescript
// 工具执行时发送进度更新
type ToolProgressData = 
  | { type: 'bash'; command: string; pid?: number }
  | { type: 'agent'; subAgentId: string; status: string }
  | { type: 'mcp'; serverName: string; toolName: string }
  | { type: 'skill'; skillName: string; phase: string }
```

**AgentKit 集成建议**：

```rust
// 添加进度事件通道

pub struct ToolExecutionContext {
    pub tool_use_id: String,
    pub progress_tx: Option<mpsc::Sender<ToolProgressEvent>>,
}

pub enum ToolProgressEvent {
    Started { description: String },
    Step { step: u32, total: u32, description: String },
    SubAgentSpawned { agent_id: String, task: String },
    SubAgentCompleted { agent_id: String, result: String },
    Completed { result: Value },
    Failed { error: String },
}

// 在 ToolAgent 中使用
impl<P> ToolAgent<P> {
    async fn execute_tool_call(
        &self,
        call: &ToolCall,
    ) -> Result<ToolResult> {
        let (progress_tx, mut progress_rx) = mpsc::channel(100);
        
        let context = ToolExecutionContext {
            tool_use_id: call.id.clone(),
            progress_tx: Some(progress_tx),
        };
        
        // 启动工具执行
        let handle = tokio::spawn(async move {
            tool.call_with_context(input, context).await
        });
        
        // 转发进度事件
        while let Some(event) = progress_rx.recv().await {
            self.observer.on_tool_progress(call.id.clone(), event).await;
        }
        
        handle.await?
    }
}
```

### 3. 工具分类系统

**Claude Code 设计**：
```
工具分类：
├── Core (核心工具)
│   ├── BashTool
│   ├── EditTool
│   └── GlobTool
├── MCP (外部服务)
│   ├── MCPServerTool
│   └── MCPResourceTool
├── Agent (子 Agent)
│   ├── LocalAgentTool
│   └── RemoteAgentTool
├── Skills (技能)
│   ├── SkillTool
│   └── SkillDiscoveryTool
└── Special (特殊工具)
    ├── OverflowTestTool
    └── TungstenTool (性能监控)
```

**AgentKit 集成建议**：

```rust
// 扩展现有 ToolCategory

#[derive(Debug, Clone, PartialEq)]
pub enum ToolCategory {
    // 现有分类
    Basic,
    System,
    Network,
    Memory,
    External,
    
    // 新增分类
    Core,           // 核心工具 (Bash, Edit, Glob)
    MCP,            // MCP 服务工具
    Agent,          // 子 Agent 工具
    Skill,          // 技能工具
    Monitoring,     // 监控工具 (性能、遥测)
    Workflow,       // 工作流工具
}

impl ToolCategory {
    pub fn requires_permission(&self) -> bool {
        matches!(
            self,
            ToolCategory::System | ToolCategory::Core | ToolCategory::Agent
        )
    }
}
```

---

## 权限管理系统

### 1. 渐进式权限层级

**Claude Code 设计**：
```
6 层渐进式权限：

Level 1: Default (默认)
  - 安全操作自动允许
  - 危险操作需要确认

Level 2: Auto-approve (自动批准)
  - 基于规则自动允许特定操作
  - 可配置路径白名单

Level 3: Bypass (绕过)
  - 临时绕过权限检查
  - 需要用户确认

Level 4: Plan Mode (计划模式)
  - 先制定计划，再执行
  - 每步需要确认

Level 5: Restricted (限制)
  - 所有操作需要确认
  - 部分操作被禁止

Level 6: ReadOnly (只读)
  - 只允许读取操作
  - 禁止任何修改
```

**AgentKit 集成建议**：

```rust
// 添加权限模式枚举

#[derive(Debug, Clone, PartialEq, Default)]
pub enum PermissionMode {
    #[default]
    Default,        // 默认模式
    AutoApprove,    // 自动批准（基于规则）
    Bypass,         // 绕过权限（临时）
    PlanMode,       // 计划模式
    Restricted,     // 限制模式
    ReadOnly,       // 只读模式
}

impl PermissionMode {
    pub fn requires_confirmation(&self, tool: &dyn Tool) -> bool {
        match self {
            PermissionMode::ReadOnly => true,
            PermissionMode::Restricted => true,
            PermissionMode::PlanMode => true,
            PermissionMode::Default => tool.categories().iter().any(|c| c.requires_permission()),
            PermissionMode::AutoApprove => false,
            PermissionMode::Bypass => false,
        }
    }
    
    pub fn can_execute(&self, tool: &dyn Tool) -> bool {
        match self {
            PermissionMode::ReadOnly => {
                // 只允许读取类工具
                !tool.categories().iter().any(|c| c.is_write_operation())
            }
            _ => true,
        }
    }
}
```

### 2. 权限规则引擎

**Claude Code 设计**：
```typescript
interface PermissionRule {
  toolName: string;
  ruleContent?: string;  // 工具特定的规则内容
  behavior: 'allow' | 'deny' | 'ask';
  source: 'global' | 'project' | 'user';
  pattern?: string;      // 路径/URL 模式
}

// 规则匹配优先级
// 1. Project rules (项目规则)
// 2. User rules (用户规则)
// 3. Global rules (全局规则)
```

**AgentKit 集成建议**：

```rust
// 添加权限规则系统

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRule {
    pub tool_name: String,
    pub behavior: PermissionBehavior,
    pub source: PermissionSource,
    pub pattern: Option<String>,  // glob 模式或正则
    pub rule_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionBehavior {
    Allow,
    Deny,
    Ask,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionSource {
    Global,    // ~/.agentkit/permissions.toml
    Project,   // .agentkit/permissions.toml
    User,      // 用户临时设置
}

pub struct PermissionEngine {
    rules: Vec<PermissionRule>,
    mode: PermissionMode,
}

impl PermissionEngine {
    pub fn check(&self, tool: &dyn Tool, input: &Value) -> PermissionDecision {
        // 1. 检查模式限制
        if !self.mode.can_execute(tool) {
            return PermissionDecision::Deny { reason: "模式限制" };
        }
        
        // 2. 匹配规则（优先级：Project > User > Global）
        let matching_rules = self.rules.iter()
            .filter(|r| r.tool_name == tool.name())
            .filter(|r| self.matches_pattern(r, input))
            .collect::<Vec<_>>();
        
        // 3. 应用最高优先级规则
        if let Some(rule) = matching_rules.first() {
            match rule.behavior {
                PermissionBehavior::Allow => PermissionDecision::Allow,
                PermissionBehavior::Deny => PermissionDecision::Deny { reason: "规则禁止" },
                PermissionBehavior::Ask => PermissionDecision::Ask,
            }
        } else {
            // 4. 无匹配规则，根据模式决定
            if self.mode.requires_confirmation(tool) {
                PermissionDecision::Ask
            } else {
                PermissionDecision::Allow
            }
        }
    }
}
```

### 3. 危险命令分类器

**Claude Code 设计**：
```typescript
// 预定义的危险命令模式
const dangerousPatterns = [
  'rm -rf',
  'dd if=',
  'chmod 777',
  'curl | bash',
  'sudo rm',
  // ... 100+ patterns
];

// 使用 ML 模型分类（可选）
async function classifyCommand(command: string): Promise<{
  riskLevel: 'low' | 'medium' | 'high';
  explanations: string[];
}> {
  // 基于模式匹配 + 轻量 ML 模型
}
```

**AgentKit 集成建议**：

```rust
// 添加危险命令检测

pub struct CommandClassifier {
    patterns: Vec<CommandPattern>,
}

#[derive(Debug, Clone)]
pub struct CommandPattern {
    pub pattern: String,      // 正则或 glob
    pub risk_level: RiskLevel,
    pub description: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RiskLevel {
    Safe,
    Low,
    Medium,
    High,
    Critical,
}

impl CommandClassifier {
    pub fn new() -> Self {
        Self {
            patterns: vec![
                // 危险文件操作
                CommandPattern {
                    pattern: r"rm\s+(-[rf]+\s+)?/".to_string(),
                    risk_level: RiskLevel::Critical,
                    description: "删除根目录文件".to_string(),
                    suggestion: Some("请确认路径是否正确".to_string()),
                },
                // 危险权限修改
                CommandPattern {
                    pattern: r"chmod\s+777".to_string(),
                    risk_level: RiskLevel::High,
                    description: "设置完全开放权限".to_string(),
                    suggestion: Some("考虑使用更严格的权限".to_string()),
                },
                // 管道执行
                CommandPattern {
                    pattern: r"curl.*\|\s*(ba)?sh".to_string(),
                    risk_level: RiskLevel::Critical,
                    description: "下载并执行远程脚本".to_string(),
                    suggestion: Some("先审查脚本内容".to_string()),
                },
                // ... 更多模式
            ],
        }
    }
    
    pub fn classify(&self, command: &str) -> CommandClassification {
        for pattern in &self.patterns {
            if let Ok(re) = regex::Regex::new(&pattern.pattern) {
                if re.is_match(command) {
                    return CommandClassification {
                        risk_level: pattern.risk_level.clone(),
                        matched_pattern: pattern.pattern.clone(),
                        description: pattern.description.clone(),
                        suggestion: pattern.suggestion.clone(),
                    };
                }
            }
        }
        
        CommandClassification {
            risk_level: RiskLevel::Safe,
            matched_pattern: String::new(),
            description: String::new(),
            suggestion: None,
        }
    }
}
```

---

## 钩子 (Hook) 系统

### 1. 异步钩子注册表

**Claude Code 设计**：
```typescript
// 钩子类型
type HookEvent = 
  | 'command_complete'
  | 'file_changed'
  | 'git_event'
  | 'session_start'
  | 'session_end';

// 异步钩子注册表
class AsyncHookRegistry {
  private pendingHooks = new Map<string, PendingAsyncHook>();
  
  register(hook: AsyncHook): void;
  checkForResponses(): Promise<HookResponse[]>;
  timeout(id: string): void;
}

// 钩子执行流程
command → 触发钩子 → 异步执行 → 超时处理 → 结果聚合
```

**AgentKit 集成建议**：

```rust
// 添加钩子系统

pub struct HookRegistry {
    sync_hooks: Vec<Arc<dyn SyncHook>>,
    async_hooks: RwLock<HashMap<String, PendingAsyncHook>>,
}

#[async_trait]
pub trait SyncHook: Send + Sync {
    fn name(&self) -> &str;
    fn event(&self) -> HookEvent;
    
    async fn execute(&self, context: &HookContext) -> Result<HookResponse>;
}

pub struct HookContext {
    pub event: HookEvent,
    pub data: Value,
    pub timeout: Duration,
}

#[derive(Debug, Clone)]
pub enum HookEvent {
    CommandComplete { command: String, exit_code: i32 },
    FileChanged { path: PathBuf, operation: FileOperation },
    GitEvent { event_type: String, details: Value },
    SessionStart { session_id: String },
    SessionEnd { session_id: String, duration: Duration },
    ToolCall { tool_name: String, input: Value },
    ToolComplete { tool_name: String, output: Value },
}

impl HookRegistry {
    pub fn new() -> Self {
        Self {
            sync_hooks: Vec::new(),
            async_hooks: RwLock::new(HashMap::new()),
        }
    }
    
    pub fn register_sync(&mut self, hook: Arc<dyn SyncHook>) {
        self.sync_hooks.push(hook);
    }
    
    pub async fn trigger(&self, event: HookEvent) -> Result<Vec<HookResponse>> {
        let context = HookContext {
            event: event.clone(),
            data: serde_json::to_value(&event)?,
            timeout: Duration::from_secs(15),
        };
        
        // 执行所有匹配的同步钩子
        let mut responses = Vec::new();
        for hook in &self.sync_hooks {
            if hook.event() == event {
                match tokio::time::timeout(context.timeout, hook.execute(&context)).await {
                    Ok(Ok(response)) => responses.push(response),
                    Ok(Err(e)) => tracing::warn!("钩子执行失败：{}", e),
                    Err(_) => tracing::warn!("钩子执行超时"),
                }
            }
        }
        
        Ok(responses)
    }
}
```

### 2. 钩子类型示例

**Claude Code 设计**：
```
内置钩子：
├── .claude/hooks/post-command.sh    # 命令执行后
├── .claude/hooks/file-changed.sh    # 文件变更
├── .claude/hooks/git-event.sh       # Git 事件
└── .claude/hooks/session-start.sh   # 会话开始
```

**AgentKit 集成建议**：

```rust
// 内置钩子实现

pub struct PostCommandHook;

#[async_trait]
impl SyncHook for PostCommandHook {
    fn name(&self) -> &str { "post_command" }
    fn event(&self) -> HookEvent { 
        HookEvent::CommandComplete { command: String::new(), exit_code: 0 }
    }
    
    async fn execute(&self, context: &HookContext) -> Result<HookResponse> {
        // 1. 记录命令历史
        self.log_command(&context.data).await?;
        
        // 2. 更新环境变量缓存
        self.update_env_cache(&context.data).await?;
        
        // 3. 触发文件状态变更检测
        self.detect_file_changes().await?;
        
        Ok(HookResponse::Success { data: None })
    }
}

pub struct FileChangedHook;

#[async_trait]
impl SyncHook for FileChangedHook {
    fn name(&self) -> &str { "file_changed" }
    fn event(&self) -> HookEvent {
        HookEvent::FileChanged { path: PathBuf::new(), operation: FileOperation::Write }
    }
    
    async fn execute(&self, context: &HookContext) -> Result<HookResponse> {
        // 1. 更新文件状态缓存
        // 2. 检查是否是配置文件变更
        // 3. 触发重新加载
        
        Ok(HookResponse::Success { data: None })
    }
}
```

---

## 任务与状态管理

### 1. 任务类型系统

**Claude Code 设计**：
```typescript
type TaskType =
  | 'local_bash'       // 本地 shell 命令
  | 'local_agent'      // 本地子 Agent
  | 'remote_agent'     // 远程 Agent
  | 'in_process_teammate'  // 进程内队友
  | 'local_workflow'   // 本地工作流
  | 'monitor_mcp'      // MCP 监控
  | 'dream';           // 后台任务

type TaskStatus =
  | 'pending'
  | 'running'
  | 'completed'
  | 'failed'
  | 'killed';

interface TaskState {
  id: string;
  type: TaskType;
  status: TaskStatus;
  description: string;
  startTime: number;
  endTime?: number;
  outputFile: string;  // 输出持久化路径
  outputOffset: number;  // 读取偏移量
}
```

**AgentKit 集成建议**：

```rust
// 添加任务类型系统

#[derive(Debug, Clone, PartialEq)]
pub enum TaskType {
    LocalBash,        // 本地 shell 命令
    LocalAgent,       // 本地子 Agent
    RemoteAgent,      // 远程 Agent
    LocalWorkflow,    // 本地工作流
    MonitorMcp,       // MCP 监控
    Background,       // 后台任务
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed { error: String },
    Killed,
}

impl TaskStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, TaskStatus::Completed | TaskStatus::Failed { .. } | TaskStatus::Killed)
    }
}

#[derive(Debug, Clone)]
pub struct TaskState {
    pub id: String,
    pub task_type: TaskType,
    pub status: TaskStatus,
    pub description: String,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub output_file: Option<PathBuf>,  // 输出持久化
    pub output_offset: usize,
    pub total_paused_ms: Option<u64>,
}

pub struct TaskManager {
    tasks: RwLock<HashMap<String, TaskState>>,
    output_dir: PathBuf,
}

impl TaskManager {
    pub fn create_task(
        &self,
        task_type: TaskType,
        description: String,
    ) -> Result<String> {
        let id = Self::generate_task_id(&task_type);
        let state = TaskState {
            id: id.clone(),
            task_type,
            status: TaskStatus::Pending,
            description,
            start_time: timestamp(),
            end_time: None,
            output_file: Some(self.output_dir.join(format!("{}.log", id))),
            output_offset: 0,
            total_paused_ms: None,
        };
        
        self.tasks.write().unwrap().insert(id.clone(), state);
        Ok(id)
    }
    
    fn generate_task_id(task_type: &TaskType) -> String {
        let prefix = match task_type {
            TaskType::LocalBash => "b",
            TaskType::LocalAgent => "a",
            TaskType::RemoteAgent => "r",
            TaskType::LocalWorkflow => "w",
            TaskType::MonitorMcp => "m",
            TaskType::Background => "d",
        };
        
        format!("{}{}", prefix, random_alphanumeric(8))
    }
}
```

### 2. 任务输出持久化

**Claude Code 设计**：
```typescript
// 任务输出写入磁盘
const outputDir = path.join(os.homedir(), '.claude', 'tasks');

async function writeTaskOutput(taskId: string, output: string): Promise<void> {
  const filePath = path.join(outputDir, `${taskId}.output`);
  await fs.appendFile(filePath, output);
}

// 支持增量读取
async function readTaskOutput(taskId: string, offset: number): Promise<string> {
  const filePath = path.join(outputDir, `${taskId}.output`);
  const stream = fs.createReadStream(filePath, { start: offset });
  return readStream(stream);
}
```

**AgentKit 集成建议**：

```rust
// 添加任务输出持久化

pub struct TaskOutputWriter {
    output_dir: PathBuf,
    writers: RwLock<HashMap<String, BufWriter<File>>>,
}

impl TaskOutputWriter {
    pub fn new(output_dir: PathBuf) -> Self {
        Self {
            output_dir,
            writers: RwLock::new(HashMap::new()),
        }
    }
    
    pub async fn write(&self, task_id: &str, output: &[u8]) -> Result<usize> {
        let mut writers = self.writers.write().unwrap();
        
        let writer = writers.entry(task_id.to_string())
            .or_insert_with(|| {
                let path = self.output_dir.join(format!("{}.log", task_id));
                let file = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                    .unwrap();
                BufWriter::new(file)
            });
        
        writer.write(output)
    }
    
    pub async fn read(&self, task_id: &str, offset: usize) -> Result<String> {
        let path = self.output_dir.join(format!("{}.log", task_id));
        let mut file = tokio::fs::File::open(path).await?;
        
        let mut buffer = Vec::new();
        file.seek(SeekFrom::Start(offset as u64)).await?;
        file.read_to_end(&mut buffer).await?;
        
        Ok(String::from_utf8_lossy(&buffer).to_string())
    }
}
```

---

## 多 Agent 协作

### 1. 协调器模式 (Coordinator)

**Claude Code 设计**：
```typescript
// 协调器架构
Coordinator
├── Worker Agents (工作 Agent)
│   ├── Agent 1 (编码)
│   ├── Agent 2 (测试)
│   └── Agent 3 (审查)
├── Task Queue (任务队列)
├── Result Aggregator (结果聚合)
└── Permission Sync (权限同步)

// 协调器配置
interface CoordinatorConfig {
  workerCount: number;
  taskTimeout: number;
  resultAggregationStrategy: 'first' | 'majority' | 'all';
  permissionSyncMode: 'leader' | 'independent';
}
```

**AgentKit 集成建议**：

```rust
// 添加协调器

pub struct Coordinator<P: LlmProvider> {
    workers: Vec<Arc<WorkerAgent<P>>>,
    task_queue: mpsc::Sender<Task>,
    result_aggregator: Arc<ResultAggregator>,
    config: CoordinatorConfig,
}

#[derive(Debug, Clone)]
pub struct CoordinatorConfig {
    pub worker_count: usize,
    pub task_timeout: Duration,
    pub aggregation_strategy: AggregationStrategy,
    pub permission_sync_mode: PermissionSyncMode,
}

#[derive(Debug, Clone)]
pub enum AggregationStrategy {
    First,      // 采用第一个结果
    Majority,   // 多数表决
    All,        // 聚合所有结果
    BestQuality,// 质量评分最高
}

#[derive(Debug, Clone)]
pub enum PermissionSyncMode {
    Leader,       // 领导者权限同步给所有 worker
    Independent,  // 每个 worker 独立权限
}

impl<P: LlmProvider> Coordinator<P> {
    pub async fn spawn(config: CoordinatorConfig, provider: Arc<P>) -> Result<Self> {
        let (task_tx, mut task_rx) = mpsc::channel::<Task>(100);
        
        let mut workers = Vec::new();
        for i in 0..config.worker_count {
            let worker = WorkerAgent::new(
                format!("worker-{}", i),
                provider.clone(),
                task_rx.clone(),
            );
            workers.push(Arc::new(worker));
        }
        
        Ok(Self {
            workers,
            task_queue: task_tx,
            result_aggregator: Arc::new(ResultAggregator::new(config.aggregation_strategy.clone())),
            config,
        })
    }
    
    pub async fn submit_task(&self, task: Task) -> Result<TaskHandle> {
        self.task_queue.send(task).await?;
        Ok(TaskHandle { /* ... */ })
    }
}
```

### 2. 队友管理 (Teammate)

**Claude Code 设计**：
```typescript
// 队友类型
type TeammateMode = 
  | 'in_process'     // 进程内
  | 'tmux'           // tmux 会话
  | 'iterm'          // iTerm2 面板
  | 'remote';        // 远程

interface Teammate {
  id: string;
  mode: TeammateMode;
  status: 'idle' | 'busy' | 'error';
  currentTask?: Task;
  permissions: PermissionState;
}

// 队友布局管理
class TeammateLayoutManager {
  // 自动排列队友窗口
  arrangeLayout(teammates: Teammate[]): void;
  
  // 同步权限状态
  syncPermissions(leader: Teammate, teammates: Teammate[]): void;
}
```

**AgentKit 集成建议**：

```rust
// 添加队友管理

#[derive(Debug, Clone)]
pub struct Teammate {
    pub id: String,
    pub mode: TeammateMode,
    pub status: TeammateStatus,
    pub current_task: Option<Task>,
    pub permissions: PermissionState,
}

#[derive(Debug, Clone)]
pub enum TeammateMode {
    InProcess,    // 进程内
    Tmux,         // tmux 会话
    Remote,       // 远程
}

#[derive(Debug, Clone, PartialEq)]
pub enum TeammateStatus {
    Idle,
    Busy { task_id: String },
    Error { error: String },
}

pub struct TeammateManager {
    teammates: RwLock<HashMap<String, Teammate>>,
    layout_manager: TeammateLayoutManager,
}

impl TeammateManager {
    pub fn spawn_teammate(&self, mode: TeammateMode) -> Result<String> {
        let id = generate_teammate_id();
        let teammate = Teammate {
            id: id.clone(),
            mode,
            status: TeammateStatus::Idle,
            current_task: None,
            permissions: PermissionState::default(),
        };
        
        self.teammates.write().unwrap().insert(id.clone(), teammate);
        self.layout_manager.arrange_layout(&self.teammates.read().unwrap().values().cloned().collect());
        
        Ok(id)
    }
    
    pub fn sync_permissions(&self, leader_id: &str) -> Result<()> {
        let teammates = self.teammates.read().unwrap();
        let leader = teammates.get(leader_id)
            .ok_or_else(|| anyhow!("领导者不存在"))?;
        
        for teammate in teammates.values() {
            if teammate.id != leader_id {
                // 同步权限状态
                self.sync_teammate_permissions(teammate, &leader.permissions)?;
            }
        }
        
        Ok(())
    }
}
```

---

## 技能 (Skill) 系统

### 1. 技能发现与加载

**Claude Code 设计**：
```typescript
// 技能目录结构
~/.claude/skills/
├── skill-name/
│   ├── SKILL.md       # 技能说明
│   ├── SKILL.js       # 技能实现
│   └── SKILL.toml     # 技能配置

// 技能加载
async function loadSkills(): Promise<Skill[]> {
  const skillDirs = await findSkillDirectories();
  const skills = await Promise.all(
    skillDirs.map(dir => loadSkill(dir))
  );
  return skills;
}

// 技能发现（远程）
async function discoverSkills(query: string): Promise<Skill[]> {
  // 从远程仓库搜索技能
  const response = await fetch('/api/skills/search', { query });
  return response.skills;
}
```

**AgentKit 集成建议**：

```rust
// 技能系统已经在 AgentKit 中实现
// 建议添加远程技能发现

pub struct SkillDiscoveryService {
    local_loader: SkillLoader,
    remote_client: Option<SkillRegistryClient>,
}

impl SkillDiscoveryService {
    pub async fn discover(&self, query: &str) -> Result<Vec<SkillInfo>> {
        let mut skills = Vec::new();
        
        // 1. 本地搜索
        let local_skills = self.local_loader.search(query).await?;
        skills.extend(local_skills);
        
        // 2. 远程搜索（如果配置了）
        if let Some(client) = &self.remote_client {
            let remote_skills = client.search(query).await?;
            skills.extend(remote_skills);
        }
        
        Ok(skills)
    }
    
    pub async fn install(&self, skill_name: &str) -> Result<()> {
        // 1. 从远程仓库下载
        // 2. 验证签名
        // 3. 安装到本地目录
        // 4. 更新技能索引
        
        Ok(())
    }
}

// 技能配置扩展
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillConfig {
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: Option<String>,
    pub tags: Vec<String>,
    pub input_schema: Value,
    pub output_schema: Value,
    pub timeout: Option<u64>,
    
    // 新增字段
    pub dependencies: Vec<SkillDependency>,
    pub permissions: Vec<SkillPermission>,
    pub repository: Option<String>,  // 远程仓库 URL
}
```

### 2. 技能改进系统

**Claude Code 设计**：
```typescript
// 技能使用追踪
interface SkillUsageStats {
  skillName: string;
  usageCount: number;
  successRate: number;
  averageExecutionTime: number;
  lastUsedAt: number;
}

// 自动改进建议
async function suggestSkillImprovements(skill: Skill): Promise<Improvement[]> {
  const stats = await getSkillUsageStats(skill.name);
  const improvements = [];
  
  if (stats.successRate < 0.8) {
    improvements.push({
      type: 'error_handling',
      suggestion: '添加更多错误处理'
    });
  }
  
  if (stats.averageExecutionTime > 5000) {
    improvements.push({
      type: 'performance',
      suggestion: '优化执行性能'
    });
  }
  
  return improvements;
}
```

**AgentKit 集成建议**：

```rust
// 添加技能使用追踪

pub struct SkillUsageTracker {
    stats: RwLock<HashMap<String, SkillUsageStats>>,
}

#[derive(Debug, Clone)]
pub struct SkillUsageStats {
    pub skill_name: String,
    pub usage_count: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub total_execution_time_ms: u64,
    pub last_used_at: u64,
}

impl SkillUsageStats {
    pub fn success_rate(&self) -> f64 {
        if self.usage_count == 0 {
            return 0.0;
        }
        self.success_count as f64 / self.usage_count as f64
    }
    
    pub fn average_execution_time_ms(&self) -> u64 {
        if self.usage_count == 0 {
            return 0;
        }
        self.total_execution_time_ms / self.usage_count
    }
}

impl SkillUsageTracker {
    pub fn record_usage(&self, skill_name: &str, success: bool, execution_time_ms: u64) {
        let mut stats = self.stats.write().unwrap();
        let entry = stats.entry(skill_name.to_string())
            .or_insert_with(|| SkillUsageStats {
                skill_name: skill_name.to_string(),
                usage_count: 0,
                success_count: 0,
                failure_count: 0,
                total_execution_time_ms: 0,
                last_used_at: 0,
            });
        
        entry.usage_count += 1;
        if success {
            entry.success_count += 1;
        } else {
            entry.failure_count += 1;
        }
        entry.total_execution_time_ms += execution_time_ms;
        entry.last_used_at = timestamp();
    }
    
    pub fn get_improvement_suggestions(&self, skill_name: &str) -> Vec<SkillImprovement> {
        let stats = self.stats.read().unwrap();
        let Some(stats) = stats.get(skill_name) else {
            return Vec::new();
        };
        
        let mut suggestions = Vec::new();
        
        if stats.success_rate() < 0.8 {
            suggestions.push(SkillImprovement {
                improvement_type: "error_handling".to_string(),
                suggestion: "添加更多错误处理".to_string(),
            });
        }
        
        if stats.average_execution_time_ms() > 5000 {
            suggestions.push(SkillImprovement {
                improvement_type: "performance".to_string(),
                suggestion: "优化执行性能".to_string(),
            });
        }
        
        suggestions
    }
}
```

---

## 记忆与上下文管理

### 1. 上下文压缩系统 (Context Compact)

**Claude Code 设计**：

Claude Code 实现了多层上下文压缩策略，防止上下文超出模型限制：

```typescript
// 压缩触发阈值
const AUTOCOMPACT_BUFFER_TOKENS = 13_000      // 自动压缩缓冲区
const WARNING_THRESHOLD_BUFFER_TOKENS = 20_000  // 警告阈值
const ERROR_THRESHOLD_BUFFER_TOKENS = 20_000    // 错误阈值
const MANUAL_COMPACT_BUFFER_TOKENS = 3_000      // 手动压缩缓冲区

// 压缩触发条件
function getAutoCompactThreshold(model: string): number {
  const effectiveContextWindow = getEffectiveContextWindowSize(model)
  return effectiveContextWindow - AUTOCOMPACT_BUFFER_TOKENS
}

// 压缩策略类型
type CompactStrategy = 
  | 'auto'          // 自动压缩（接近限制时）
  | 'reactive'      // 响应式压缩（API 拒绝时）
  | 'manual'        // 用户手动触发
  | 'session_memory' // 会话记忆压缩
```

**压缩流程**：

```
1. 监控 token 使用量
   ↓
2. 达到阈值 → 触发压缩
   ↓
3. 分组消息（按 API 轮次）
   ↓
4. 调用压缩 Agent 生成摘要
   ↓
5. 替换旧消息 + 保存边界
   ↓
6. 后压缩清理（恢复关键文件）
```

**压缩提示词模板**：

```typescript
const BASE_COMPACT_PROMPT = `Your task is to create a detailed summary...

Your summary should include:
1. Primary Request and Intent (用户主要请求和意图)
2. Key Technical Concepts (关键技术概念)
3. Files and Code Sections (文件和代码段，包含完整代码片段)
4. Errors and fixes (错误和修复方法)
5. Problem Solving (解决的问题)
6. All user messages (所有用户消息)
7. Pending Tasks (待处理任务)
8. Current Work (当前工作详情)
9. Optional Next Step (可选的下一步)`
```

**消息分组策略**：

```typescript
// 按 API 轮次分组（而非按用户轮次）
function groupMessagesByApiRound(messages: Message[]): Message[][] {
  const groups: Message[][] = []
  let lastAssistantId: string | undefined
  
  for (const msg of messages) {
    // 当新的 assistant 响应开始时创建边界
    if (msg.type === 'assistant' && 
        msg.message.id !== lastAssistantId && 
        current.length > 0) {
      groups.push(current)
      current = [msg]
    } else {
      current.push(msg)
    }
    if (msg.type === 'assistant') {
      lastAssistantId = msg.message.id
    }
  }
  
  if (current.length > 0) {
    groups.push(current)
  }
  return groups
}
```

**后压缩清理**：

```typescript
// 压缩后恢复关键文件内容
const POST_COMPACT_MAX_FILES_TO_RESTORE = 5
const POST_COMPACT_TOKEN_BUDGET = 50_000
const POST_COMPACT_MAX_TOKENS_PER_FILE = 5_000
const POST_COMPACT_MAX_TOKENS_PER_SKILL = 5_000
const POST_COMPACT_SKILLS_TOKEN_BUDGET = 25_000

// 恢复最近访问的文件，确保上下文连续性
async function runPostCompactCleanup(): Promise<void> {
  // 1. 识别最近修改的文件
  // 2. 读取文件内容
  // 3. 插入到压缩后的消息流
  // 4. 恢复技能指令
}
```

**AgentKit 集成建议**：

```rust
// 添加上下文压缩系统

#[derive(Debug, Clone)]
pub struct CompactConfig {
    pub auto_compact_enabled: bool,
    pub auto_compact_buffer_tokens: u32,
    pub warning_buffer_tokens: u32,
    pub error_buffer_tokens: u32,
    pub manual_compact_buffer_tokens: u32,
    pub strategy: CompactStrategy,
}

#[derive(Debug, Clone)]
pub enum CompactStrategy {
    Auto,           // 自动压缩
    Reactive,       // 响应式压缩
    Manual,         // 手动压缩
    SessionMemory,  // 会话记忆压缩
}

pub struct ContextManager {
    messages: Vec<Message>,
    token_count: u32,
    compact_boundary: Option<usize>,
    config: CompactConfig,
}

impl ContextManager {
    pub fn should_compact(&self, model: &str) -> bool {
        let threshold = self.get_compact_threshold(model);
        self.token_count >= threshold
    }
    
    fn get_compact_threshold(&self, model: &str) -> u32 {
        let context_window = get_context_window_for_model(model);
        context_window - self.config.auto_compact_buffer_tokens
    }
    
    pub async fn compact(&mut self, provider: &dyn LlmProvider) -> Result<()> {
        // 1. 分组消息
        let groups = self.group_messages_by_api_round();
        
        // 2. 选择要压缩的组（保留最近的）
        let groups_to_compact = self.select_groups_to_compact(&groups);
        
        // 3. 调用压缩 Agent
        let summary = self.generate_compact_summary(
            provider,
            &groups_to_compact,
        ).await?;
        
        // 4. 创建边界消息
        let boundary_message = self.create_compact_boundary(summary);
        
        // 5. 替换旧消息
        self.replace_compacted_messages(boundary_message);
        
        // 6. 后压缩清理
        self.run_post_compact_cleanup().await?;
        
        Ok(())
    }
    
    fn group_messages_by_api_round(&self) -> Vec<Vec<Message>> {
        // 实现按 API 轮次分组逻辑
        Vec::new()
    }
    
    async fn generate_compact_summary(
        &self,
        provider: &dyn LlmProvider,
        messages: &[Vec<Message>],
    ) -> Result<String> {
        // 使用压缩提示词模板调用 LLM 生成摘要
        Ok(String::new())
    }
}

// 压缩提示词模板
pub const COMPACT_PROMPT_TEMPLATE: &str = r#"
你的任务是对对话进行详细摘要，以便后续继续开发工作而不丢失上下文。

请包含以下部分：

1. 主要请求和意图：详细捕获用户的明确请求和意图
2. 关键技术概念：列出所有重要的技术概念、技术和框架
3. 文件和代码段：枚举具体检查和修改的文件，包含完整代码片段
4. 错误和修复：列出所有遇到的错误及修复方法
5. 问题解决：记录解决的问题和正在进行的调试工作
6. 所有用户消息：列出所有非工具结果的用户消息
7. 待处理任务：概述明确要求处理的待处理任务
8. 当前工作：详细描述最近正在处理的工作
9. 可选下一步：列出与最近工作相关的下一步

请基于以上结构提供详细的摘要。
"#;
```

### 2. 记忆形状遥测

**Claude Code 设计**：
```typescript
// 记忆形状追踪
interface MemoryShapeTelemetry {
  sessionId: string;
  messageCount: number;
  tokenUsage: {
    input: number;
    output: number;
  };
  contextCompressionEvents: number;
  memoryRetrievalEvents: number;
}

// 定期发送遥测
function emitMemoryShapeTelemetry(): void {
  const telemetry = collectMemoryShapeTelemetry();
  analytics.track('memory_shape', telemetry);
}
```

**AgentKit 集成建议**：

```rust
// 添加记忆遥测

pub struct MemoryTelemetry {
    session_stats: RwLock<HashMap<String, SessionStats>>,
}

#[derive(Debug, Clone)]
pub struct SessionStats {
    pub session_id: String,
    pub message_count: u64,
    pub token_input: u64,
    pub token_output: u64,
    pub context_compression_events: u64,
    pub memory_retrieval_events: u64,
}

impl MemoryTelemetry {
    pub fn record_message(&self, session_id: &str, tokens_input: u64, tokens_output: u64) {
        let mut stats = self.session_stats.write().unwrap();
        let entry = stats.entry(session_id.to_string())
            .or_insert_with(|| SessionStats {
                session_id: session_id.to_string(),
                message_count: 0,
                token_input: 0,
                token_output: 0,
                context_compression_events: 0,
                memory_retrieval_events: 0,
            });
        
        entry.message_count += 1;
        entry.token_input += tokens_input;
        entry.token_output += tokens_output;
    }
    
    pub fn record_compression_event(&self, session_id: &str) {
        let mut stats = self.session_stats.write().unwrap();
        if let Some(entry) = stats.get_mut(session_id) {
            entry.context_compression_events += 1;
        }
    }
    
    pub fn emit_telemetry(&self) {
        let stats = self.session_stats.read().unwrap();
        for session_stat in stats.values() {
            tracing::info!(
                target: "memory_telemetry",
                session_id = %session_stat.session_id,
                message_count = session_stat.message_count,
                token_input = session_stat.token_input,
                token_output = session_stat.token_output,
                context_compression_events = session_stat.context_compression_events,
                "memory_shape"
            );
        }
    }
}
```

### 3. 上下文压缩策略

**Claude Code 设计**：
```typescript
// 压缩策略
type CompactStrategy = 
  | 'reactive'     // 响应式压缩（token 超限时）
  | 'snip'         // 片段式压缩
  | 'micro'        // 微压缩（每 N 条消息）
  | 'cached'       // 缓存压缩结果

// 压缩配置
interface CompactConfig {
  strategy: CompactStrategy;
  targetTokenCount: number;
  preserveToolCalls: boolean;
  preserveSystemPrompt: boolean;
}
```

**AgentKit 集成建议**：

```rust
// 扩展现有 ConversationManager

#[derive(Debug, Clone)]
pub enum CompactStrategy {
    Reactive,     // token 超限时压缩
    Snip,         // 片段式压缩
    Micro,        // 微压缩（每 N 条消息）
    Cached,       // 缓存压缩结果
}

#[derive(Debug, Clone)]
pub struct CompactConfig {
    pub strategy: CompactStrategy,
    pub target_token_count: u32,
    pub preserve_tool_calls: bool,
    pub preserve_system_prompt: bool,
    pub micro_compact_interval: u32,  // 微压缩间隔
}

impl ConversationManager {
    pub fn should_compact(&self, current_tokens: u32, config: &CompactConfig) -> bool {
        match config.strategy {
            CompactStrategy::Reactive => current_tokens > config.target_token_count,
            CompactStrategy::Micro => {
                self.messages.len() as u32 % config.micro_compact_interval == 0
            }
            _ => false,
        }
    }
    
    pub async fn compact(&self, config: &CompactConfig) -> Result<()> {
        // 实现压缩逻辑
        Ok(())
    }
}
```

---

## 遥测与可观测性

### 1. 全链路追踪

**Claude Code 设计**：
```typescript
// OpenTelemetry 集成
const tracer = trace.getTracer('claude-code');

async function executeTool(tool: Tool, input: any): Promise<ToolResult> {
  return tracer.startActiveSpan('tool.execute', async (span) => {
    span.setAttribute('tool.name', tool.name);
    span.setAttribute('tool.input', JSON.stringify(input));
    
    try {
      const result = await tool.execute(input);
      span.setAttribute('tool.output.length', JSON.stringify(result).length);
      span.setStatus({ code: SpanStatusCode.OK });
      return result;
    } catch (error) {
      span.setStatus({ code: SpanStatusCode.ERROR, message: error.message });
      throw error;
    } finally {
      span.end();
    }
  });
}

// BigQuery 导出
function exportToBigQuery(events: TelemetryEvent[]): void {
  bigquery.insert('claude_code_sessions', events);
}
```

**AgentKit 集成建议**：

```rust
// 添加 OpenTelemetry 集成

use opentelemetry::{global, trace::Tracer};

pub struct TelemetryConfig {
    pub enabled: bool,
    pub exporter: TelemetryExporter,
    pub sample_rate: f64,
}

#[derive(Debug, Clone)]
pub enum TelemetryExporter {
    Stdout,
    Otlp(String),  // OTLP endpoint
    BigQuery(String),
}

pub fn init_telemetry(config: TelemetryConfig) -> Result<()> {
    if !config.enabled {
        return Ok(());
    }
    
    let tracer = global::tracer("agentkit");
    
    // 配置导出器
    match config.exporter {
        TelemetryExporter::Stdout => {
            // 输出到 stdout
        }
        TelemetryExporter::Otlp(endpoint) => {
            // OTLP 导出
        }
        TelemetryExporter::BigQuery(project) => {
            // BigQuery 导出
        }
    }
    
    Ok(())
}

// 在 ToolAgent 中使用
impl<P> ToolAgent<P> {
    async fn execute_tool_call(&self, call: &ToolCall) -> Result<ToolResult> {
        let tracer = global::tracer("agentkit");
        
        tracer.in_span("tool.execute", |cx| {
            let span = cx.span();
            span.set_attribute(Key::new("tool.name").string(call.name.clone()));
            span.set_attribute(Key::new("tool.input").string(call.input.to_string()));
            
            let result = self._execute_tool(call).await;
            
            match &result {
                Ok(output) => {
                    span.set_attribute(
                        Key::new("tool.output.length")
                            .i64(output.to_string().len() as i64)
                    );
                    span.set_status(Status::Ok);
                }
                Err(e) => {
                    span.set_status(Status::Error {
                        description: e.to_string().into(),
                    });
                }
            }
            
            result
        })
    }
}
```

### 2. 性能监控

**Claude Code 设计**：
```typescript
// 性能指标
interface PerformanceMetrics {
  toolExecutionTime: Histogram;
  llmLatency: Histogram;
  hookExecutionTime: Histogram;
  memoryUsage: Gauge;
  activeTasks: Gauge;
}

// 性能监控工具
const TungstenTool = {
  async execute(): Promise<PerformanceReport> {
    return {
      avgToolExecutionTime: metrics.toolExecutionTime.mean(),
      p95LlmLatency: metrics.llmLatency.p95(),
      peakMemoryUsage: metrics.memoryUsage.max(),
    };
  }
};
```

**AgentKit 集成建议**：

```rust
// 添加性能监控

use metrics::{histogram, gauge};

pub struct PerformanceMonitor {
    tool_execution_time: Histogram,
    llm_latency: Histogram,
    hook_execution_time: Histogram,
}

impl PerformanceMonitor {
    pub fn record_tool_execution(&self, tool_name: &str, duration_ms: u64) {
        histogram!("tool.execution.time.ms", duration_ms)
            .with_tag("tool", tool_name.to_string());
    }
    
    pub fn record_llm_latency(&self, model: &str, latency_ms: u64) {
        histogram!("llm.latency.ms", latency_ms)
            .with_tag("model", model.to_string());
    }
    
    pub fn record_active_tasks(&self, count: u64) {
        gauge!("tasks.active").set(count as f64);
    }
    
    pub fn generate_report(&self) -> PerformanceReport {
        PerformanceReport {
            avg_tool_execution_time: self.tool_execution_time.mean(),
            p95_llm_latency: self.llm_latency.p95(),
            // ...
        }
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceReport {
    pub avg_tool_execution_time_ms: f64,
    pub p95_llm_latency_ms: f64,
    pub peak_memory_mb: f64,
    pub active_tasks: u64,
}
```

---

## 推荐集成优先级

### Phase 1 (立即 - 1-2 周)

| 功能 | 工作量 | 价值 | 优先级 |
|------|--------|------|--------|
| 任务状态机 | 中 | 高 | ⭐⭐⭐ |
| 工具进度追踪 | 中 | 高 | ⭐⭐⭐ |
| 危险命令分类器 | 低 | 高 | ⭐⭐⭐ |

**理由**：这些功能可以立即提升用户体验和安全性，实现成本较低。

### Phase 2 (短期 - 2-4 周)

| 功能 | 工作量 | 价值 | 优先级 |
|------|--------|------|--------|
| 权限规则引擎 | 高 | 高 | ⭐⭐⭐ |
| 渐进式权限模式 | 中 | 高 | ⭐⭐⭐ |
| 任务输出持久化 | 中 | 中 | ⭐⭐ |

**理由**：权限系统是生产级 Agent 的核心需求，需要优先实现。

### Phase 3 (中期 - 1-2 月)

| 功能 | 工作量 | 价值 | 优先级 |
|------|--------|------|--------|
| 钩子系统 | 高 | 中 | ⭐⭐ |
| 多 Agent 协调器 | 高 | 中 | ⭐⭐ |
| 队友管理 | 高 | 中 | ⭐⭐ |

**理由**：这些功能针对复杂场景，适合在基础功能完善后实现。

### Phase 4 (长期 - 2-3 月)

| 功能 | 工作量 | 价值 | 优先级 |
|------|--------|------|--------|
| 技能市场 | 高 | 中 | ⭐ |
| 遥测系统 | 中 | 低 | ⭐ |
| 记忆形状遥测 | 低 | 低 | ⭐ |

**理由**：这些是锦上添花的功能，可以在核心功能稳定后考虑。

---

## 总结

Claude Code 源码展示了构建生产级 Agent 系统的最佳实践：

1. **渐进式架构**：从简单对话到完全自主，12 层渐进式机制
2. **统一工具接口**：所有工具统一接口，支持进度追踪和权限声明
3. **权限规则引擎**：6 层渐进式权限 + 规则引擎 + 危险命令分类
4. **钩子系统**：异步钩子注册表，支持超时处理和进度追踪
5. **任务管理**：类型化任务 + 状态机 + 输出持久化
6. **多 Agent 协作**：协调器模式 + 队友管理 + 权限同步
7. **技能系统**：技能发现 + 加载 + 版本管理 + 使用追踪
8. **遥测与可观测性**：全链路追踪 + 性能监控 + BigQuery 导出

AgentKit 可以逐步集成这些设计思路，构建更强大的生产级 Agent 框架。

---

*报告生成时间：2026 年 4 月 1 日*  
*分析基于 Claude Code v2.1.88 源码*
