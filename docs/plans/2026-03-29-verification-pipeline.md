# 验证管道 (Verification Pipeline) 设计文档

> 日期: 2026-03-29 | 优先级: P0 (最高) | 状态: 设计中

---

## 1. 背景与动机

当前 zcode 的验证层存在严重的架构缺陷：

- **ReviewerAgent 检查太浅** — 仅检查 `unwrap()`, `panic!()`, 硬编码凭据、`.clone()` 和行长度，无法判断代码是否真正解决了需求
- **没有评分机制** — 只有 pass/fail，无法量化表达"这个实现 70 分，需要继续优化到 90 分"
- **没有反馈环** — 验证发现问题后，没有机制让 CoderAgent 自动修复并重新验证
- **没有自动化测试执行** — 系统不会自动运行 `cargo test`，不会解析测试结果
- **没有覆盖率分析** — 不知道新增代码是否有足够的测试覆盖

这导致系统在执行层面缺少质量闭环，代码质量完全依赖 LLM 单次生成的质量。

---

## 2. 设计目标

| 目标 | 描述 |
|------|------|
| **可量化** | 每次验证产出 0-100 分，而非简单的 pass/fail |
| **可插拔** | Verifier 是 trait，可自由组合、添加、替换 |
| **可配置** | 最低分数阈值、重试次数、各 Verifier 权重均可配置 |
| **自动化** | 验证 → 评分 → 反馈 → 重执行 → 重验证，全闭环 |
| **分层验证** | 单 task 验证 + 全局门禁验证（所有 task 完成后的终检） |

---

## 3. 核心抽象

### 3.1 Verifier Trait

```rust
/// 验证器 trait — 所有验证逻辑实现此接口
#[async_trait]
pub trait Verifier: Send + Sync {
    /// 验证器名称
    fn name(&self) -> &str;

    /// 该验证器的描述
    fn description(&self) -> &str;

    /// 该验证器在总分中的权重（所有 verifier 权重归一化）
    fn weight(&self) -> f64;

    /// 执行验证，返回验证结果
    async fn verify(&self, context: &VerificationContext) -> VerificationResult;
}
```

### 3.2 VerificationContext

```rust
/// 验证上下文 — 传递给每个 Verifier 的只读信息
pub struct VerificationContext {
    /// 原始需求描述
    pub requirement: String,

    /// 当前 task 描述
    pub task_description: String,

    /// 执行前的工作区 snapshot id
    pub pre_snapshot_id: Option<i64>,

    /// 执行后的 git diff（新增/修改的代码）
    pub diff_patch: String,

    /// 变更文件列表及其内容
    pub changed_files: Vec<(String, String)>,

    /// 项目根路径
    pub project_root: PathBuf,

    /// LLM provider（用于语义验证）
    pub llm_client: Option<Arc<LlmClient>>,

    /// 项目配置
    pub config: ProjectConfig,
}
```

### 3.3 VerificationResult & VerificationScore

```rust
/// 单个 Verifier 的验证结果
pub struct VerificationResult {
    /// 验证器名称
    pub verifier_name: String,

    /// 得分 (0.0 - 100.0)
    pub score: f64,

    /// 发现的问题列表
    pub issues: Vec<VerificationIssue>,

    /// 验证过程日志（用于调试和反馈）
    pub log: String,

    /// 验证耗时
    pub duration: Duration,
}

/// 量化评分
pub struct VerificationScore {
    /// 加权总分 (0.0 - 100.0)
    pub total: f64,

    /// 各 verifier 的得分明细
    pub breakdown: Vec<VerifierScoreEntry>,

    /// 是否通过（total >= policy.min_score）
    pub passed: bool,

    /// 降权/扣分最多的 top-N 问题
    pub top_issues: Vec<VerificationIssue>,
}

pub struct VerifierScoreEntry {
    pub name: String,
    pub score: f64,
    pub weight: f64,
    pub weighted_score: f64,
    pub issues_count: usize,
}

/// 单个验证问题
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationIssue {
    /// 问题严重级别
    pub severity: IssueSeverity,
    /// 问题类别
    pub category: String,
    /// 问题描述
    pub message: String,
    /// 修复建议
    pub suggestion: String,
    /// 关联的文件和行号
    pub location: Option<FileLocation>,
    /// 对应的代码片段
    pub snippet: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IssueSeverity {
    Critical,  // 必须修复，直接扣 20+ 分
    High,      // 强烈建议修复，扣 10-15 分
    Medium,    // 建议修复，扣 5-10 分
    Low,       // 可选优化，扣 1-5 分
    Info,      // 信息性提示，不扣分
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileLocation {
    pub path: String,
    pub line_start: Option<usize>,
    pub line_end: Option<usize>,
}
```

### 3.4 VerificationPolicy

```rust
/// 验证策略 — 控制验证行为
pub struct VerificationPolicy {
    /// 最低通过分数 (0.0 - 100.0)，默认 70.0
    pub min_score: f64,

    /// 最大重试次数（验证不通过时重新执行 + 重新验证），默认 3
    pub max_retries: u32,

    /// 启用的验证器列表（按名称），空则启用全部
    pub enabled_verifiers: Vec<String>,

    /// 各验证器的权重覆盖
    pub weight_overrides: HashMap<String, f64>,

    /// 是否在每次重试时注入上次的验证结果作为反馈
    pub inject_feedback: bool,

    /// 是否在最终门禁时运行全量验证（vs 增量）
    pub full_gate_verification: bool,
}

impl Default for VerificationPolicy {
    fn default() -> Self {
        Self {
            min_score: 70.0,
            max_retries: 3,
            enabled_verifiers: vec![],
            weight_overrides: HashMap::new(),
            inject_feedback: true,
            full_gate_verification: true,
        }
    }
}
```

---

## 4. 验证管道流程

```
                    ┌─────────────────────────────────────────┐
                    │           Task 执行完成                   │
                    └──────────────────┬──────────────────────┘
                                       │
                    ┌──────────────────▼──────────────────────┐
                    │         收集 VerificationContext         │
                    │  (diff, changed_files, requirement, ...) │
                    └──────────────────┬──────────────────────┘
                                       │
              ┌────────────────────────┼────────────────────────┐
              │                        │                        │
    ┌─────────▼──────────┐ ┌──────────▼─────────┐ ┌───────────▼────────┐
    │   TestVerifier      │ │  LintVerifier       │ │ SemanticVerifier   │
    │   cargo test / npm  │ │  clippy / eslint    │ │ LLM 判断正确性     │
    │   score: 0-100      │ │  score: 0-100       │ │ score: 0-100       │
    └─────────┬──────────┘ └──────────┬──────────┘ └───────────┬────────┘
              │                        │                        │
              │              ┌─────────▼─────────┐              │
              │              │ ReviewerVerifier   │              │
              │              │ 静态分析 5 类      │              │
              │              │ score: 0-100       │              │
              │              └─────────┬──────────┘              │
              │                        │                        │
              └────────────────────────┼────────────────────────┘
                                       │
                    ┌──────────────────▼──────────────────────┐
                    │         加权汇总 → VerificationScore      │
                    │   total = Σ(score_i × weight_i) / Σ(w)  │
                    └──────────────────┬──────────────────────┘
                                       │
                              ┌────────▼────────┐
                              │ total >= 70.0 ? │
                              └───┬─────────┬───┘
                           Yes    │         │  No
                              ┌───▼───┐  ┌──▼──────────────────┐
                              │ PASS  │  │ 构建反馈信息          │
                              │       │  │ 注入 top_issues      │
                              └───────┘  │ retries < max?       │
                                         └───┬─────────┬────────┘
                                          Yes │         │ No
                                    ┌─────────▼──┐ ┌───▼──────┐
                                    │ 重新执行    │ │ 标记失败  │
                                    │ CoderAgent │ │ 记录结果  │
                                    │ + 反馈context│ └──────────┘
                                    └────────────┘
```

---

## 5. 各 Verifier 详细设计

### 5.1 TestVerifier（测试验证器）

**职责**: 自动运行项目测试命令，解析结果，计算得分。

```rust
pub struct TestVerifier {
    /// 测试命令（默认从项目类型推断：cargo test / npm test / pytest）
    test_command: Option<String>,
}

impl TestVerifier {
    fn detect_test_command(project_root: &Path) -> String {
        if project_root.join("Cargo.toml").exists() {
            "cargo test --no-fail-fast -- -Z unstable-options --format json".into()
        } else if project_root.join("package.json").exists() {
            "npm test".into()
        } else if project_root.join("pytest.ini").exists() || project_root.join("pyproject.toml").exists() {
            "pytest --tb=short -q".into()
        } else {
            "make test".into()
        }
    }
}
```

**评分逻辑**:

| 情况 | 分数 |
|------|------|
| 所有测试通过 | 100 |
| 部分测试失败 | `passed / total * 100` |
| 测试编译失败 | 0 |
| 没有测试 | 50（中等分数，不高不低） |

**输出**: 将测试失败信息（test name, expected, actual, backtrace）注入反馈上下文。

### 5.2 LintVerifier（代码检查验证器）

**职责**: 运行 linter，解析 warnings 和 errors。

```rust
pub struct LintVerifier;

impl LintVerifier {
    fn detect_lint_command(project_root: &Path) -> Option<String> {
        if project_root.join("Cargo.toml").exists() {
            Some("cargo clippy --message-format=json 2>&1".into())
        } else if project_root.join("package.json").exists() {
            Some("npx eslint --format json . 2>/dev/null".into())
        } else {
            None
        }
    }
}
```

**评分逻辑**:

| 指标 | 分数计算 |
|------|---------|
| 0 warnings, 0 errors | 100 |
| 有 warnings | `100 - (warnings * 2)`，最低 40 |
| 有 errors | `100 - (errors * 10)`，最低 0 |
| 无 linter 可用 | 跳过（权重为 0） |

### 5.3 SemanticVerifier（语义验证器）

**职责**: 使用 LLM 判断代码变更是否真正解决了需求。

```rust
pub struct SemanticVerifier {
    llm_client: Arc<LlmClient>,
}

impl SemanticVerifier {
    fn build_verification_prompt(&self, ctx: &VerificationContext) -> String {
        format!(
            "你是一个代码审查专家。请评估以下代码变更是否正确实现了需求。\n\n\
             ## 需求\n{}\n\n\
             ## 变更的代码\n```diff\n{}\n```\n\n\
             ## 完整文件内容\n{}\n\n\
             请按以下标准评分（每项 0-100 分）：\n\
             1. **功能正确性**: 代码是否实现了需求描述的功能？\n\
             2. **边界处理**: 是否处理了边界情况和异常？\n\
             3. **代码质量**: 命名、结构、可读性如何？\n\
             4. **潜在副作用**: 是否可能引入回归或副作用？\n\n\
             输出 JSON 格式：\n\
             ```json\n\
             {{\"scores\": {{\"correctness\": N, \"edge_cases\": N, \"quality\": N, \"side_effects\": N}}, \
             \"issues\": [{{\"severity\": \"critical|high|medium|low\", \"message\": \"...\", \"suggestion\": \"...\"}}]}}\n\
             ```",
            ctx.requirement,
            ctx.diff_patch,
            ctx.changed_files.iter()
                .map(|(p, c)| format!("### {}\n```\n{}\n```", p, c))
                .collect::<Vec<_>>()
                .join("\n\n")
        )
    }
}
```

**评分**: 取 correctness × 0.4 + edge_cases × 0.2 + quality × 0.2 + side_effects × 0.2

### 5.4 ReviewerVerifier（静态分析验证器）

**职责**: 包装现有 ReviewerAgent 的 5 类检查，输出量化分数。

```rust
pub struct ReviewerVerifier;

impl ReviewerVerifier {
    /// 将 ReviewIssue 转为分数
    fn calculate_score(issues: &[ReviewIssue]) -> f64 {
        let mut deductions = 0.0;
        for issue in issues {
            let penalty = match issue.severity {
                IssueSeverity::Critical => 20.0,
                IssueSeverity::High => 12.0,
                IssueSeverity::Medium => 6.0,
                IssueSeverity::Low => 2.0,
                IssueSeverity::Info => 0.0,
            };
            deductions += penalty;
        }
        (100.0 - deductions).max(0.0)
    }
}
```

### 5.5 CoverageVerifier（覆盖率验证器）

**职责**: 分析新增代码的测试覆盖率。

```rust
pub struct CoverageVerifier;

impl CoverageVerifier {
    /// 使用 git diff 定位新增/修改的行，检查这些行是否被测试覆盖
    /// 对于 Rust: cargo-llvm-cov 或 tarpaulin
    /// 对于 Python: coverage.py
    /// 对于 JS/TS: c8 / nyc
}
```

**评分**: `covered_new_lines / total_new_lines * 100`，无覆盖工具时跳过。

---

## 6. 反馈环设计

### 6.1 VerificationFeedback

```rust
/// 验证反馈 — 注入到下一轮 CoderAgent 的执行上下文中
pub struct VerificationFeedback {
    /// 总分
    pub score: f64,
    /// 最低通过分数
    pub min_score: f64,
    /// 分数差距
    pub gap: f64,
    /// Top 问题列表（按严重程度排序）
    pub issues: Vec<VerificationIssue>,
    /// 当前重试轮次
    pub retry_iteration: u32,
    /// 最大重试次数
    pub max_retries: u32,
    /// 测试输出（如果有）
    pub test_output: Option<String>,
    /// Lint 输出（如果有）
    pub lint_output: Option<String>,
}

impl VerificationFeedback {
    /// 格式化为 LLM 可理解的反馈文本
    pub fn as_prompt_context(&self) -> String {
        format!(
            "## 验证反馈 (第 {} 轮, 共 {} 次机会)\n\
             当前得分: {:.1}/100 (需要 {:.1} 分通过)\n\n\
             ### 需要修复的问题:\n{}\n\n\
             ### 测试输出:\n```\n{}\n```\n\n\
             请修复以上问题。优先处理 Critical 和 High 级别的问题。",
            self.retry_iteration,
            self.max_retries,
            self.score,
            self.min_score,
            self.issues.iter().enumerate()
                .map(|(i, issue)| format!("{}. [{}] {} — {}", i + 1,
                    match issue.severity {
                        IssueSeverity::Critical => "🔴 Critical",
                        IssueSeverity::High => "🟠 High",
                        IssueSeverity::Medium => "🟡 Medium",
                        IssueSeverity::Low => "🔵 Low",
                        IssueSeverity::Info => "⚪ Info",
                    },
                    issue.message,
                    issue.suggestion))
                .collect::<Vec<_>>()
                .join("\n"),
            self.test_output.as_deref().unwrap_or("无测试输出")
        )
    }
}
```

### 6.2 验证-执行循环

```rust
impl VerificationPipeline {
    pub async fn verify_with_retry(
        &self,
        ctx: &VerificationContext,
        executor: &mut dyn TaskExecutor,
    ) -> PipelineVerificationResult {
        let mut retry_count = 0;
        let mut last_feedback: Option<VerificationFeedback> = None;

        loop {
            // 1. 执行验证
            let score = self.run_verifiers(ctx).await;

            // 2. 判断是否通过
            if score.total >= self.policy.min_score {
                return PipelineVerificationResult::Passed(score);
            }

            // 3. 检查重试次数
            retry_count += 1;
            if retry_count > self.policy.max_retries {
                return PipelineVerificationResult::Failed(score);
            }

            // 4. 构建反馈
            let feedback = VerificationFeedback {
                score: score.total,
                min_score: self.policy.min_score,
                gap: self.policy.min_score - score.total,
                issues: score.top_issues.clone(),
                retry_iteration: retry_count,
                max_retries: self.policy.max_retries,
                test_output: self.extract_test_output(&score),
                lint_output: self.extract_lint_output(&score),
            };

            // 5. 带反馈重新执行
            let new_ctx = executor.re_execute_with_feedback(ctx, &feedback).await;
            // 循环回到步骤 1
            last_feedback = Some(feedback);
        }
    }
}
```

---

## 7. 全局门禁验证

所有 task 完成后的终检：

```rust
pub struct GateVerification {
    pub pipeline: VerificationPipeline,
}

impl GateVerification {
    pub async fn run_gate(&self, project_root: &Path, tasks: &[TaskRecord]) -> GateResult {
        // 1. 运行完整测试套件（不仅仅是变更相关的）
        // 2. 运行全量 lint
        // 3. 运行全量安全扫描
        // 4. 检查所有 task 的验证历史
        // 5. 生成最终报告

        GateResult {
            passed: true,
            overall_score: 85.0,
            tasks_summary: vec![],
            full_test_result: None,
            security_scan_result: None,
        }
    }
}
```

---

## 8. 与现有模块集成

### 8.1 集成点

| 现有模块 | 集成方式 |
|----------|---------|
| `agent/loop_exec.rs` | AgentLoop 完成一轮 tool call 后，调用 `VerificationPipeline::verify()` |
| `agent/coder.rs` | CoderAgent 增加 `re_execute_with_feedback()` 方法，接受 `VerificationFeedback` |
| `agent/reviewer.rs` | ReviewerAgent 逻辑封装为 `ReviewerVerifier` |
| `task_store/` | TaskRecord 增加 `verification_history: Vec<VerificationScore>` 字段 |
| `session/snapshot.rs` | 验证前自动创建 snapshot，用于回滚 |
| `config/` | ProjectConfig 增加 `verification: VerificationPolicy` 字段 |

### 8.2 TaskRecord 扩展

```rust
// 在 task_store/mod.rs 中扩展
pub struct TaskRecord {
    // ... 现有字段 ...

    /// 验证历史（每次验证一条记录）
    pub verification_history: Vec<VerificationScore>,

    /// 最终验证得分
    pub final_score: Option<f64>,
}
```

---

## 9. 配置方案

在 `.zcode/config.toml` 中新增 `[verification]` 段：

```toml
[verification]
min_score = 70.0
max_retries = 3
inject_feedback = true
full_gate_verification = true

[verification.verifiers.test]
enabled = true
weight = 0.30
command = "cargo test"  # 可选覆盖

[verification.verifiers.lint]
enabled = true
weight = 0.15

[verification.verifiers.semantic]
enabled = true
weight = 0.25

[verification.verifiers.reviewer]
enabled = true
weight = 0.20

[verification.verifiers.coverage]
enabled = false  # 默认关闭，需要额外工具
weight = 0.10
min_coverage_delta = 0.8
```

---

## 10. 错误处理策略

| 错误场景 | 处理策略 |
|----------|---------|
| Verifier 超时 | 标记该 verifier 为 `TimedOut`，权重分配给其他 verifier |
| 测试命令不存在 | 跳过 TestVerifier，重新归一化权重 |
| LLM 不可用 | 跳过 SemanticVerifier，降低 min_score 阈值 |
| 部分 verifier 失败 | 用成功的 verifier 按比例计算总分 |
| 所有 verifier 失败 | 总分 0，标记失败 |

---

## 11. 文件组织

```
src/
└── verification/
    ├── mod.rs          — 公共接口 + VerificationPipeline
    ├── types.rs        — VerificationContext, VerificationResult, VerificationScore, VerificationIssue
    ├── policy.rs       — VerificationPolicy, GateVerification
    ├── feedback.rs     — VerificationFeedback + prompt formatting
    ├── pipeline.rs     — VerificationPipeline 主逻辑（run_verifiers, verify_with_retry）
    ├── verifiers/
    │   ├── mod.rs      — Verifier trait
    │   ├── test.rs     — TestVerifier
    │   ├── lint.rs     — LintVerifier
    │   ├── semantic.rs — SemanticVerifier
    │   ├── reviewer.rs — ReviewerVerifier (包装现有 ReviewerAgent)
    │   └── coverage.rs — CoverageVerifier
    └── scoring.rs      — 评分引擎（加权汇总、归一化）
```

---

## 12. 实现路线图

### Phase 1: 核心框架（1 周）
- [ ] 创建 `verification/` 模块骨架
- [ ] 实现 `Verifier` trait 和 `VerificationPipeline`
- [ ] 实现 `VerificationScore` 加权汇总
- [ ] 实现 `VerificationPolicy` 配置

### Phase 2: 基础 Verifiers（1 周）
- [ ] 实现 `TestVerifier`（支持 cargo test, npm test, pytest）
- [ ] 实现 `ReviewerVerifier`（包装现有 ReviewerAgent）
- [ ] 实现 `LintVerifier`（支持 cargo clippy, eslint）

### Phase 3: 高级 Verifier + 反馈环（1 周）
- [ ] 实现 `SemanticVerifier`（LLM 判断）
- [ ] 实现反馈环 `verify_with_retry()`
- [ ] 实现 `VerificationFeedback` prompt 构建
- [ ] 集成到 AgentLoop 和 CoderAgent

### Phase 4: 全局门禁 + 覆盖率（1 周）
- [ ] 实现 `GateVerification`
- [ ] 实现 `CoverageVerifier`
- [ ] TaskRecord 扩展 + 验证历史存储
- [ ] TUI 中展示验证分数
