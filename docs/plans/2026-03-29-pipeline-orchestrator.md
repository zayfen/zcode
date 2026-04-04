# 管道编排器 (Pipeline Orchestrator) 设计文档

> 日期: 2026-03-29 | 优先级: P4 | 状态: 设计中

---

## 1. 背景与动机

当前 zcode 的四个阶段（认知、执行、验证、交付）是分散的：

- `OrchestratorAgent` 仅做简单的路由（Planner → Coder → Reviewer）
- 没有统一的管道控制器串联所有阶段
- 验证失败后没有自动回到执行层的反馈环
- 没有全局的可观测性（token 消耗、耗时、成功率）
- 管道中断后无法恢复（crash → 从头来过）

需要一个统一的 Pipeline Orchestrator 将四个阶段串联为闭环。

---

## 2. 设计目标

| 目标 | 描述 |
|------|------|
| **闭环** | Cognition → Plan → Execute → Verify → (loop) → Deliver |
| **可观测** | 每个阶段的 token、耗时、成本都有度量 |
| **可恢复** | 管道状态持久化，crash 后可恢复 |
| **可配置** | 每个项目可自定义启用哪些阶段和参数 |
| **可扩展** | 每个阶段是 trait object，可替换实现 |

---

## 3. 核心抽象

### 3.1 PipelinePhase Trait

```rust
/// 管道阶段 trait — 每个阶段实现此接口
#[async_trait]
pub trait PipelinePhase: Send + Sync {
    /// 阶段名称
    fn name(&self) -> &str;

    /// 阶段描述
    fn description(&self) -> &str;

    /// 执行该阶段
    async fn execute(&self, context: &mut PipelineContext) -> PhaseResult;
}

/// 阶段执行结果
pub struct PhaseResult {
    /// 阶段名称
    pub phase_name: String,

    /// 执行状态
    pub status: PhaseStatus,

    /// 执行耗时
    pub duration: Duration,

    /// Token 消耗
    pub tokens_used: TokenUsage,

    /// 阶段产出的摘要信息
    pub summary: String,

    /// 阶段特有的元数据
    pub metadata: HashMap<String, String>,

    /// 错误信息（如果失败）
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PhaseStatus {
    /// 阶段成功完成
    Success,
    /// 阶段失败
    Failed,
    /// 阶段被跳过
    Skipped,
    /// 需要回退到之前的阶段
    Retry { target_phase: String, reason: String },
}
```

### 3.2 PipelineContext（共享上下文）

```rust
/// 管道共享上下文 — 在阶段间传递数据
pub struct PipelineContext {
    // ─── 输入 ───
    /// 原始需求描述
    pub requirement: String,

    /// 项目根路径
    pub project_root: PathBuf,

    /// 项目配置
    pub config: ProjectConfig,

    // ─── 认知阶段产出 ───
    /// 知识上下文（来自认知引擎）
    pub knowledge: Option<KnowledgeContext>,

    // ─── 计划阶段产出 ───
    /// 任务图
    pub task_graph: Option<TaskGraph>,

    // ─── 执行阶段产出 ───
    /// 执行结果摘要
    pub execution_summary: Option<ExecutionSummary>,

    // ─── 验证阶段产出 ───
    /// 验证结果（per-task）
    pub verification_results: HashMap<TaskId, VerificationScore>,

    /// 当前重试轮次
    pub retry_iteration: u32,

    /// 反馈信息（注入到下一轮执行）
    pub feedback: Option<VerificationFeedback>,

    // ─── 交付阶段产出 ───
    /// 交付结果
    pub delivery_result: Option<DeliveryResult>,

    // ─── 度量 ───
    /// 各阶段的度量数据
    pub metrics: PipelineMetrics,
}

/// 管道度量数据
#[derive(Debug, Clone, Default)]
pub struct PipelineMetrics {
    /// 各阶段的度量
    pub phase_metrics: Vec<PhaseMetrics>,

    /// 总 token 使用量
    pub total_tokens: u64,

    /// 总耗时
    pub total_duration: Duration,

    /// 预估成本 (USD)
    pub estimated_cost_usd: f64,

    /// 管道开始时间
    pub started_at: Option<DateTime<Utc>>,

    /// 管道结束时间
    pub finished_at: Option<DateTime<Utc>>,
}

/// 单阶段度量
#[derive(Debug, Clone)]
pub struct PhaseMetrics {
    pub phase_name: String,
    pub duration: Duration,
    pub tokens: TokenUsage,
    pub status: PhaseStatus,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
}

/// Token 使用量
#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}
```

### 3.3 PipelineResult

```rust
/// 管道最终结果
pub struct PipelineResult {
    /// 是否全部成功
    pub success: bool,

    /// 需求描述
    pub requirement: String,

    /// 任务执行汇总
    pub tasks_completed: usize,
    pub tasks_failed: usize,
    pub tasks_skipped: usize,

    /// 验证得分
    pub avg_verification_score: f64,

    /// 交付信息
    pub delivery: Option<DeliveryResult>,

    /// 度量数据
    pub metrics: PipelineMetrics,

    /// 各阶段结果
    pub phase_results: Vec<PhaseResult>,
}
```

---

## 4. 管道流程图

```
┌──────────────────────────────────────────────────────────────────┐
│                     Pipeline::run(requirement)                    │
└─────────────────────────────┬────────────────────────────────────┘
                              │
                              ▼
                  ┌───────────────────────┐
                  │   before_pipeline hook │
                  └───────────┬────────────┘
                              │
              ┌───────────────▼───────────────┐
              │    Phase 1: Cognition          │
              │    知识获取 + 索引构建           │
              │    → KnowledgeContext           │
              └───────────────┬───────────────┘
                              │
                  ┌───────────▼────────────┐
                  │  after_cognition hook   │
                  └───────────┬────────────┘
                              │
              ┌───────────────▼───────────────┐
              │    Phase 2: Planning           │
              │    任务拆分 + DAG 构建          │
              │    → TaskGraph                  │
              └───────────────┬───────────────┘
                              │
                  ┌───────────▼────────────┐
                  │  after_planning hook    │
                  │  [Plan 审批 — 可选]      │
                  └───────────┬────────────┘
                              │
              ┌───────────────▼───────────────────────────────┐
              │    Phase 3: Execution                          │
              │    ┌─────────────────────────────────────────┐ │
              │    │  for each task in task_graph:            │ │
              │    │    snapshot → execute → verify           │ │
              │    │         ┌──────────┐                     │ │
              │    │         │ score ≥  │── Yes → next task   │ │
              │    │         │ 70?      │                     │ │
              │    │         └────┬─────┘                     │ │
              │    │              │ No                        │ │
              │    │              │ retries < max?            │ │
              │    │         Yes ─┤         ┌── No            │ │
              │    │              │         │                  │ │
              │    │     inject feedback    fail task          │ │
              │    │     re-execute                            │ │
              │    └─────────────────────────────────────────┘ │
              └───────────────┬───────────────────────────────┘
                              │
                  ┌───────────▼────────────┐
                  │  after_execution hook   │
                  └───────────┬────────────┘
                              │
              ┌───────────────▼───────────────┐
              │    Phase 4: Gate Verification  │
              │    全量测试 + lint + 安全扫描    │
              └───────────────┬───────────────┘
                              │
                  ┌───────────▼────────────┐
                  │  after_verification    │
                  └───────────┬────────────┘
                              │
              ┌───────────────▼───────────────┐
              │    Phase 5: Delivery           │
              │    Changelog + PR + CI         │
              └───────────────┬───────────────┘
                              │
                  ┌───────────▼────────────┐
                  │  after_pipeline hook    │
                  └───────────┬────────────┘
                              │
                  ┌───────────▼────────────┐
                  │    PipelineResult       │
                  │    度量 + 报告           │
                  └────────────────────────┘
```

---

## 5. Pipeline 主逻辑

```rust
/// 管道编排器 — 串联所有阶段
pub struct Pipeline {
    /// 各阶段实现
    phases: Vec<Box<dyn PipelinePhase>>,

    /// 管道配置
    config: PipelineConfig,

    /// 生命周期钩子
    hooks: PipelineHooks,
}

impl Pipeline {
    pub fn new(config: PipelineConfig) -> Self {
        let mut phases: Vec<Box<dyn PipelinePhase>> = Vec::new();

        if config.cognition.enabled {
            phases.push(Box::new(CognitionPhase::new()));
        }
        if config.planning.enabled {
            phases.push(Box::new(PlanningPhase::new()));
        }
        if config.execution.enabled {
            phases.push(Box::new(ExecutionPhase::new()));
        }
        if config.verification.enabled {
            phases.push(Box::new(VerificationPhase::new()));
        }
        if config.delivery.enabled {
            phases.push(Box::new(DeliveryPhase::new()));
        }

        Self {
            phases,
            config,
            hooks: PipelineHooks::default(),
        }
    }

    /// 执行整个管道
    pub async fn run(&self, requirement: &str, project_root: &Path) -> PipelineResult {
        let mut context = PipelineContext::new(requirement, project_root);
        context.metrics.started_at = Some(Utc::now());

        // before_pipeline hook
        self.hooks.run("before_pipeline", &context).await;

        let mut phase_results = Vec::new();

        for phase in &self.phases {
            let phase_name = phase.name();
            let phase_start = Utc::now();

            // before_phase hook
            self.hooks.run(&format!("before_{}", phase_name), &context).await;

            // 执行阶段
            let result = phase.execute(&mut context).await;

            // 记录度量
            let metrics = PhaseMetrics {
                phase_name: phase_name.to_string(),
                duration: result.duration,
                tokens: result.tokens_used.clone(),
                status: result.status.clone(),
                started_at: phase_start,
                finished_at: Utc::now(),
            };
            context.metrics.phase_metrics.push(metrics);
            context.metrics.total_tokens += result.tokens_used.total_tokens as u64;
            context.metrics.total_duration += result.duration;

            // after_phase hook
            self.hooks.run(&format!("after_{}", phase_name), &context).await;

            // 处理 Retry 状态
            if let PhaseStatus::Retry { target_phase, reason } = &result.status {
                tracing::warn!("Phase {} requests retry to {}: {}", phase_name, target_phase, reason);
                // TODO: 实现重试逻辑 — 重置到目标阶段重新执行
            }

            phase_results.push(result);

            // 阶段失败处理
            if matches!(phase_results.last().unwrap().status, PhaseStatus::Failed) {
                // 检查是否可跳过
                if self.is_phase_optional(phase_name) {
                    tracing::warn!("Optional phase {} failed, skipping", phase_name);
                    continue;
                }
                // 不可跳过的阶段失败 → 管道失败
                break;
            }
        }

        context.metrics.finished_at = Some(Utc::now());
        context.metrics.estimated_cost_usd = self.estimate_cost(&context.metrics);

        // after_pipeline hook
        self.hooks.run("after_pipeline", &context).await;

        // 持久化管道结果
        self.persist_result(&context).await;

        PipelineResult::from_context(context, phase_results)
    }
}
```

---

## 6. 生命周期钩子设计

### 6.1 PipelineHooks

```rust
/// 管道生命周期钩子
pub struct PipelineHooks {
    /// 钩子注册表: event_name → Vec<HookHandler>
    handlers: HashMap<String, Vec<HookHandler>>,
}

type HookHandler = Box<dyn Fn(&PipelineContext) -> Pin<Box<dyn Future<Output = HookResult> + Send>> + Send + Sync>;

pub enum HookResult {
    /// 继续执行
    Continue,
    /// 中止管道
    Abort { reason: String },
    /// 跳过下一阶段
    SkipNext,
}

impl PipelineHooks {
    /// 注册钩子
    pub fn on(&mut self, event: &str, handler: HookHandler) {
        self.handlers.entry(event.to_string()).or_default().push(handler);
    }

    /// 执行钩子
    pub async fn run(&self, event: &str, context: &PipelineContext) {
        if let Some(handlers) = self.handlers.get(event) {
            for handler in handlers {
                match handler(context).await {
                    HookResult::Continue => {}
                    HookResult::Abort { reason } => {
                        tracing::error!("Pipeline aborted by hook '{}': {}", event, reason);
                        // TODO: propagate abort
                    }
                    HookResult::SkipNext => {
                        tracing::info!("Next phase skipped by hook '{}'", event);
                    }
                }
            }
        }
    }

    /// 内置钩子事件列表
    pub fn standard_events() -> Vec<&'static str> {
        vec![
            "before_pipeline",
            "after_pipeline",
            "before_cognition",
            "after_cognition",
            "before_planning",
            "after_planning",
            "before_execution",
            "after_execution",
            "before_verification",
            "after_verification",
            "before_delivery",
            "after_delivery",
        ]
    }
}
```

### 6.2 ScriptHookHandler

```rust
/// 脚本钩子 — 执行用户配置的 shell 脚本
pub struct ScriptHookHandler {
    command: String,
    timeout: Duration,
}

impl ScriptHookHandler {
    pub async fn execute(&self, context: &PipelineContext) -> HookResult {
        let output = tokio::time::timeout(
            self.timeout,
            tokio::process::Command::new("sh")
                .arg("-c")
                .arg(&self.command)
                .env("ZCODE_PHASE", context.current_phase())
                .env("ZCODE_PROJECT_ROOT", &context.project_root)
                .output(),
        ).await;

        match output {
            Ok(Ok(out)) if out.status.success() => HookResult::Continue,
            Ok(Ok(out)) => HookResult::Abort {
                reason: format!("Hook script failed: {}", String::from_utf8_lossy(&out.stderr)),
            },
            Ok(Err(e)) => HookResult::Abort {
                reason: format!("Hook script error: {}", e),
            },
            Err(_) => HookResult::Abort {
                reason: "Hook script timed out".into(),
            },
        }
    }
}
```

---

## 7. 状态持久化与恢复

### 7.1 PipelineState

```rust
/// 管道状态 — 用于崩溃恢复
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineState {
    /// 唯一 ID
    pub id: String,

    /// 需求描述
    pub requirement: String,

    /// 项目路径
    pub project_root: String,

    /// 已完成的阶段列表
    pub completed_phases: Vec<String>,

    /// 当前阶段
    pub current_phase: Option<String>,

    /// 各阶段的序列化上下文
    pub phase_contexts: HashMap<String, serde_json::Value>,

    /// 管道配置
    pub config: PipelineConfig,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 最后更新时间
    pub updated_at: DateTime<Utc>,
}

impl PipelineState {
    /// 保存到 .zcode/pipeline-state.json
    pub fn save(&self, project_root: &Path) -> Result<()> {
        let dir = project_root.join(".zcode");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("pipeline-state.json");
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// 从 .zcode/pipeline-state.json 加载
    pub fn load(project_root: &Path) -> Result<Option<Self>> {
        let path = project_root.join(".zcode").join("pipeline-state.json");
        if !path.exists() {
            return Ok(None);
        }
        let json = std::fs::read_to_string(&path)?;
        let state: PipelineState = serde_json::from_str(&json)?;
        Ok(Some(state))
    }

    /// 清除状态（管道成功完成后）
    pub fn clear(project_root: &Path) -> Result<()> {
        let path = project_root.join(".zcode").join("pipeline-state.json");
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }
}
```

### 7.2 恢复流程

```rust
impl Pipeline {
    /// 恢复中断的管道
    pub async fn resume(&self, project_root: &Path) -> Result<PipelineResult> {
        let state = PipelineState::load(project_root)?
            .ok_or_else(|| ZcodeError::InternalError("No pipeline state found".into()))?;

        tracing::info!("Resuming pipeline '{}' from phase '{}'",
            state.id,
            state.current_phase.as_deref().unwrap_or("unknown"));

        // 重建 context
        let mut context = PipelineContext::new(&state.requirement, project_root);

        // 恢复已完成的阶段数据
        for phase_name in &state.completed_phases {
            if let Some(phase_ctx) = state.phase_contexts.get(phase_name) {
                context.restore_phase_data(phase_name, phase_ctx);
            }
        }

        // 从当前阶段继续执行
        let resume_from = state.completed_phases.len();
        self.run_from(resume_from, &mut context).await
    }
}
```

---

## 8. 错误处理与恢复策略

| 场景 | 策略 |
|------|------|
| 认知阶段失败 | 跳过（可选阶段），降级到基础 context |
| 计划阶段失败 | 重试 1 次，仍失败则中止 |
| 执行阶段单 task 失败 | 标记失败，继续其他独立 task |
| 执行阶段全部失败 | 中止管道 |
| 验证阶段分数低 | 反馈到执行阶段，重试最多 N 次 |
| 交付阶段失败 | 保存当前状态，等待人工介入 |
| 管道 crash | 下次启动时检测 state 文件，自动恢复 |
| 钩子脚本失败 | 可选钩子：跳过；必需钩子：中止 |

---

## 9. 配置方案

```toml
[pipeline]
# 全局开关
enabled = true

# 各阶段启用/禁用
[pipeline.cognition]
enabled = true
index_on_open = true

[pipeline.planning]
enabled = true
require_approval = false  # 是否需要人工审批计划

[pipeline.execution]
enabled = true
concurrency = 3
budget = { max_tokens = 100000, max_iterations = 20 }

[pipeline.verification]
enabled = true
min_score = 70.0
max_retries = 3

[pipeline.delivery]
enabled = true
auto_pr = true

# 生命周期钩子
[[pipeline.hooks]]
event = "after_planning"
command = "echo 'Plan created with {{task_count}} tasks'"
required = false

[[pipeline.hooks]]
event = "before_delivery"
command = "make pre-release"
required = true
```

---

## 10. CLI 集成

```
zcode run "Add user authentication"     # 完整管道
zcode pipeline run "Add auth"            # 同上，显式管道命令
zcode pipeline resume                    # 恢复中断的管道
zcode pipeline status                    # 查看当前管道状态
zcode pipeline cancel                    # 取消管道
zcode pipeline metrics                   # 查看度量数据
```

---

## 11. 文件组织

```
src/
└── pipeline/
    ├── mod.rs          — Pipeline 公共接口 + PipelineBuilder
    ├── context.rs      — PipelineContext, PipelineMetrics, TokenUsage
    ├── result.rs       — PipelineResult, PhaseResult, PhaseStatus
    ├── state.rs        — PipelineState 持久化与恢复
    ├── hooks.rs        — PipelineHooks, HookHandler, ScriptHookHandler
    ├── config.rs       — PipelineConfig
    └── phases/
        ├── mod.rs      — PipelinePhase trait
        ├── cognition.rs   — CognitionPhase
        ├── planning.rs    — PlanningPhase
        ├── execution.rs   — ExecutionPhase
        ├── verification.rs — VerificationPhase
        └── delivery.rs    — DeliveryPhase
```

---

## 12. 实现路线图

### Phase 1: 核心框架（1 周）
- [ ] 实现 `PipelinePhase` trait
- [ ] 实现 `PipelineContext` 和 `PipelineMetrics`
- [ ] 实现 `Pipeline::run()` 主循环
- [ ] 实现 `PipelineConfig`

### Phase 2: 阶段实现（1 周）
- [ ] 实现 5 个 Phase 的适配器（连接已有模块）
- [ ] 实现验证 → 执行的反馈环
- [ ] 集成到 CLI

### Phase 3: 钩子 + 持久化（1 周）
- [ ] 实现 `PipelineHooks` 和 `ScriptHookHandler`
- [ ] 实现 `PipelineState` 持久化
- [ ] 实现 `Pipeline::resume()` 恢复逻辑

### Phase 4: 可观测性 + 完善（1 周）
- [ ] 实现 token/cost 度量
- [ ] TUI 中展示管道进度
- [ ] `zcode pipeline status/metrics` 命令
- [ ] 端到端集成测试
