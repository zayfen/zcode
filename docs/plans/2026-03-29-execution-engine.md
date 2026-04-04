# 执行引擎增强 (Execution Engine) 设计文档

> 日期: 2026-03-29 | 优先级: P2 | 状态: 设计中

---

## 1. 背景与动机

当前 zcode 的执行层存在以下不足：

- **任务扁平列表** — 没有依赖关系，无法表达"B 依赖 A 完成后才能开始"
- **串行执行** — AgentLoop 一次只处理一个 tool call，浪费 tokio 异步能力
- **粗糙的回滚** — 只有 workspace 级别的 snapshot，无法精确回滚到某个 task 开始前
- **无预算控制** — 一个 task 可能无限制消耗 LLM token 和时间
- **无人工审批** — 高风险操作（删除文件、执行 shell）没有暂停确认机制

---

## 2. 设计目标

| 目标 | 描述 |
|------|------|
| **DAG 驱动** | 任务组织为有向无环图，支持依赖关系和并行执行 |
| **并行执行** | 独立任务并行运行，单个任务内的并行 tool call |
| **精细回滚** | 每个 task 自动创建 snapshot，失败时精确回滚 |
| **预算控制** | 限制 token、迭代次数、成本、时间 |
| **安全审批** | 高风险操作暂停等待人工确认 |

---

## 3. 核心抽象

### 3.1 TaskGraph（任务依赖图）

```rust
/// 任务依赖图 — 有向无环图
pub struct TaskGraph {
    /// 所有任务节点
    nodes: HashMap<TaskId, TaskNode>,

    /// 邻接表: task_id → 依赖它的 task_ids
    dependents: HashMap<TaskId, Vec<TaskId>>,

    /// 邻接表: task_id → 它依赖的 task_ids
    dependencies: HashMap<TaskId, Vec<TaskId>>,
}

pub struct TaskNode {
    pub task: Task,
    pub status: TaskNodeStatus,
    /// 该 task 开始前的 snapshot id
    pub pre_snapshot_id: Option<i64>,
    /// 执行预算
    pub budget: ExecutionBudget,
    /// 验证得分历史
    pub verification_history: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskNodeStatus {
    /// 等待依赖完成
    Pending,
    /// 依赖已满足，可以执行
    Ready,
    /// 正在执行
    Running,
    /// 执行完成，等待验证
    AwaitingVerification,
    /// 验证通过
    Completed,
    /// 执行或验证失败
    Failed { reason: String },
    /// 被跳过（因为依赖失败）
    Skipped,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct TaskId(pub String);
```

### 3.2 TaskGraph 方法

```rust
impl TaskGraph {
    /// 从 PlannerAgent 输出的任务列表构建 DAG
    pub fn build(tasks: Vec<Task>, dependencies: Vec<(TaskId, TaskId)>) -> Result<Self>;

    /// 拓扑排序，返回执行层级（同层可并行）
    pub fn execution_levels(&self) -> Vec<Vec<TaskId>>;

    /// 获取当前可执行的任务（依赖已完成的）
    pub fn ready_tasks(&self) -> Vec<TaskId>;

    /// 标记任务完成，释放依赖它的任务
    pub fn complete_task(&mut self, id: &TaskId) -> Vec<TaskId>;

    /// 标记任务失败，级联标记依赖它的任务为 Skipped
    pub fn fail_task(&mut self, id: &TaskId, reason: &str) -> Vec<TaskId>;

    /// 检测循环依赖
    pub fn validate_no_cycles(&self) -> Result<()>;

    /// 关键路径分析
    pub fn critical_path(&self) -> Vec<TaskId>;

    /// 导出为 DOT 格式（用于可视化）
    pub fn to_dot(&self) -> String;
}
```

**执行层级示例**:

```
Level 0: [Task A, Task B]          ← 可并行
Level 1: [Task C]                  ← 依赖 A
Level 2: [Task D, Task E]          ← 依赖 B 和 C，可并行
Level 3: [Task F]                  ← 依赖 D 和 E
```

### 3.3 ExecutionBudget（执行预算）

```rust
/// 执行预算 — 控制单个 task 的资源消耗
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionBudget {
    /// 最大 LLM token 消耗
    pub max_tokens: u32,

    /// 最大 agent loop 迭代次数
    pub max_iterations: u32,

    /// 最大预估成本 (USD)
    pub max_cost_usd: f64,

    /// 最大执行时间
    pub max_duration: Duration,

    /// 最大 tool call 次数
    pub max_tool_calls: u32,
}

impl Default for ExecutionBudget {
    fn default() -> Self {
        Self {
            max_tokens: 100_000,         // 100K tokens
            max_iterations: 20,          // 20 轮迭代
            max_cost_usd: 1.0,           // $1.00
            max_duration: Duration::from_secs(300), // 5 分钟
            max_tool_calls: 50,          // 50 次 tool call
        }
    }
}

/// 预算追踪器 — 实时监控预算消耗
pub struct BudgetTracker {
    pub budget: ExecutionBudget,
    pub tokens_used: AtomicU32,
    pub iterations_used: AtomicU32,
    pub tool_calls_used: AtomicU32,
    pub start_time: Instant,
}

impl BudgetTracker {
    /// 检查是否超出预算
    pub fn is_exceeded(&self) -> bool {
        self.tokens_used.load(Ordering::Relaxed) >= self.budget.max_tokens
            || self.iterations_used.load(Ordering::Relaxed) >= self.budget.max_iterations
            || self.tool_calls_used.load(Ordering::Relaxed) >= self.budget.max_tool_calls
            || self.start_time.elapsed() >= self.budget.max_duration
    }

    /// 预算使用报告
    pub fn report(&self) -> BudgetReport {
        BudgetReport {
            tokens_used: self.tokens_used.load(Ordering::Relaxed),
            tokens_budget: self.budget.max_tokens,
            iterations_used: self.iterations_used.load(Ordering::Relaxed),
            iterations_budget: self.budget.max_iterations,
            elapsed: self.start_time.elapsed(),
            duration_budget: self.budget.max_duration,
            estimated_cost: self.estimate_cost(),
        }
    }
}
```

### 3.4 CheckpointPolicy（人工审批策略）

```rust
/// 人工审批策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointPolicy {
    /// 审批模式
    pub mode: CheckpointMode,

    /// 高风险操作模式列表（需要审批的 glob）
    pub high_risk_patterns: Vec<HighRiskPattern>,

    /// 是否在执行前展示计划给用户审批
    pub approve_plan_before_execution: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheckpointMode {
    /// 自动执行所有操作（不暂停）
    Auto,
    /// 仅高风险操作暂停
    HighRiskOnly,
    /// 每个 task 开始前都暂停
    EveryTask,
    /// 自定义规则
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighRiskPattern {
    /// 匹配的 tool 名称
    pub tool_name: String,
    /// 匹配的输入参数模式
    pub input_pattern: String,
    /// 风险描述
    pub description: String,
}

impl Default for CheckpointPolicy {
    fn default() -> Self {
        Self {
            mode: CheckpointMode::HighRiskOnly,
            high_risk_patterns: vec![
                HighRiskPattern {
                    tool_name: "shell".into(),
                    input_pattern: ".*(rm |del |rmdir |format |dd |mkfs).*".into(),
                    description: "文件删除命令".into(),
                },
                HighRiskPattern {
                    tool_name: "file_write".into(),
                    input_pattern: ".*/etc/.*".into(),
                    description: "系统目录写入".into(),
                },
                HighRiskPattern {
                    tool_name: "shell".into(),
                    input_pattern: ".*(curl|wget)\\s+.*\\|.*sh".into(),
                    description: "远程脚本执行".into(),
                },
            ],
            approve_plan_before_execution: true,
        }
    }
}
```

---

## 4. 并行 Tool Call 执行设计

### 4.1 增强 AgentLoop

当前 `AgentLoop` 串行处理 tool calls。增强后支持并行：

```rust
impl AgentLoop {
    /// 处理 LLM 返回的 tool calls（支持并行）
    async fn handle_tool_calls(
        &mut self,
        tool_calls: &[ToolCallRequest],
        registry: &ToolRegistry,
    ) -> Vec<ToolCallResponse> {
        if tool_calls.len() <= 1 {
            // 单个 tool call，直接执行
            let tc = &tool_calls[0];
            let result = registry.execute(&tc.name, tc.input.clone());
            vec![ToolCallResponse {
                id: tc.id.clone(),
                name: tc.name.clone(),
                result: result.map_err(|e| e.to_string()),
            }]
        } else {
            // 多个 tool calls，并行执行
            let futures: Vec<_> = tool_calls.iter()
                .map(|tc| {
                    let registry = registry.clone(); // Arc clone
                    let id = tc.id.clone();
                    let name = tc.name.clone();
                    let input = tc.input.clone();
                    tokio::spawn(async move {
                        let result = registry.execute(&name, input);
                        ToolCallResponse { id, name, result: result.map_err(|e| e.to_string()) }
                    })
                })
                .collect();

            let mut results = Vec::with_capacity(futures.len());
            for fut in futures {
                results.push(fut.await.unwrap());
            }
            results
        }
    }
}
```

### 4.2 并行 Task 执行

```rust
/// 任务图执行器 — 管理 DAG 中任务的并行执行
pub struct TaskGraphExecutor {
    graph: TaskGraph,
    agent_pool: AgentPool,
    concurrency_limit: usize,
}

impl TaskGraphExecutor {
    /// 执行整个任务图
    pub async fn execute(&mut self) -> TaskGraphResult {
        let mut completed = HashSet::new();
        let mut failed = HashSet::new();

        loop {
            // 1. 获取当前可执行的任务
            let ready = self.graph.ready_tasks();
            if ready.is_empty() && completed.len() + failed.len() >= self.graph.len() {
                break; // 全部完成
            }

            // 2. 并行执行（受并发限制）
            let batch: Vec<_> = ready.into_iter()
                .take(self.concurrency_limit)
                .collect();

            let mut futures = Vec::new();
            for task_id in batch {
                let agent = self.agent_pool.acquire().await;
                let task = self.graph.get_task(&task_id).clone();
                futures.push(tokio::spawn(async move {
                    agent.execute_task(&task).await
                }));
            }

            // 3. 收集结果
            for (i, result) in futures::future::join_all(futures).await.iter().enumerate() {
                let task_id = &batch[i];
                match result {
                    Ok(task_result) => {
                        self.graph.complete_task(task_id);
                        completed.insert(task_id.clone());
                    }
                    Err(e) => {
                        let skipped = self.graph.fail_task(task_id, &e.to_string());
                        failed.insert(task_id.clone());
                        failed.extend(skipped);
                    }
                }
            }
        }

        TaskGraphResult { completed, failed }
    }
}
```

---

## 5. 快照与回滚设计

### 5.1 自动 Snapshot 策略

```rust
/// 快照管理器扩展
impl Workspace {
    /// 在 task 执行前自动创建快照
    pub fn pre_task_snapshot(&mut self, task: &Task) -> Result<i64> {
        let name = format!("pre-task-{}", task.id);
        let desc = format!("Auto-snapshot before task: {}", task.description);
        self.snapshot_save(name, Some(&desc))
    }

    /// 在 task 失败时自动回滚到 task 前状态
    pub fn rollback_task(&self, snapshot_id: i64) -> Result<usize> {
        tracing::warn!("Rolling back to snapshot {}", snapshot_id);
        self.snapshot_restore(snapshot_id)
    }
}
```

### 5.2 快照策略配置

```rust
pub enum SnapshotStrategy {
    /// 每个 task 执行前创建快照
    PerTask,
    /// 每轮 agent loop 迭代前创建快照（更精细但更耗空间）
    PerIteration,
    /// 仅在检测到高风险操作前创建
    OnHighRiskOnly,
}
```

---

## 6. 计划验证设计

### 6.1 PlanVerifier

```rust
/// 计划验证器 — 在执行前验证 Planner 输出
pub struct PlanVerifier;

impl PlanVerifier {
    pub fn verify(plan: &TaskGraph) -> PlanVerificationResult {
        let mut issues = Vec::new();

        // 1. 循环依赖检测
        if let Err(e) = plan.validate_no_cycles() {
            issues.push(PlanIssue::CircularDependency(e.to_string()));
        }

        // 2. 孤立任务检测（无入边也无出边的任务）
        let isolated = plan.isolated_tasks();
        if !isolated.is_empty() {
            issues.push(PlanIssue::IsolatedTasks(isolated));
        }

        // 3. 过深依赖链检测（可能导致串行瓶颈）
        let depth = plan.max_depth();
        if depth > 5 {
            issues.push(PlanIssue::DeepChain { depth });
        }

        // 4. 资源预估
        let estimated_tokens = plan.estimate_total_tokens();
        let estimated_duration = plan.estimate_duration();

        PlanVerificationResult {
            valid: issues.is_empty(),
            issues,
            estimated_tokens,
            estimated_duration,
            task_count: plan.len(),
            max_parallelism: plan.max_parallelism(),
            critical_path_length: plan.critical_path().len(),
        }
    }
}
```

---

## 7. 配置方案

在 `.zcode/config.toml` 中：

```toml
[execution]
concurrency_limit = 3          # 最大并行任务数
snapshot_strategy = "per_task" # 快照策略

[execution.budget]
max_tokens = 100000
max_iterations = 20
max_cost_usd = 1.0
max_duration_secs = 300
max_tool_calls = 50

[execution.checkpoint]
mode = "high_risk_only"
approve_plan = true

[[execution.checkpoint.high_risk_patterns]]
tool_name = "shell"
input_pattern = ".*(rm |del ).*"
description = "文件删除命令"
```

---

## 8. 文件组织

```
src/
├── agent/
│   ├── graph.rs        — TaskGraph, TaskNode, TaskId, execution levels
│   ├── executor.rs     — TaskGraphExecutor (并行执行)
│   └── plan_verify.rs  — PlanVerifier
├── execution/
│   ├── mod.rs          — 公共接口
│   ├── budget.rs       — ExecutionBudget, BudgetTracker, BudgetReport
│   ├── checkpoint.rs   — CheckpointPolicy, HighRiskPattern, 审批流程
│   └── snapshot.rs     — SnapshotStrategy, 自动快照管理
```

---

## 9. 实现路线图

### Phase 1: TaskGraph + 拓扑排序（1 周）
- [ ] 实现 `TaskGraph` 数据结构
- [ ] 实现拓扑排序和 `execution_levels()`
- [ ] 实现循环依赖检测
- [ ] 修改 PlannerAgent 输出增加 `dependencies` 字段

### Phase 2: 并行执行 + 预算控制（1 周）
- [ ] 实现 `TaskGraphExecutor`
- [ ] 增强 `AgentLoop` 支持并行 tool calls
- [ ] 实现 `ExecutionBudget` 和 `BudgetTracker`

### Phase 3: 快照 + 回滚 + 审批（1 周）
- [ ] 实现自动 pre-task snapshot
- [ ] 实现失败自动回滚
- [ ] 实现 `CheckpointPolicy` 和高风险操作检测
- [ ] TUI 中集成审批对话框

### Phase 4: 计划验证 + 优化（1 周）
- [ ] 实现 `PlanVerifier`
- [ ] 实现关键路径分析
- [ ] 实现 DOT 导出（任务图可视化）
- [ ] 性能优化和集成测试
