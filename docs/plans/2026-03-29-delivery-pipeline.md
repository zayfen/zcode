# 交付管道 (Delivery Pipeline) 设计文档

> 日期: 2026-03-29 | 优先级: P3 | 状态: 设计中

---

## 1. 背景与动机

当前 zcode 在代码执行和验证完成后，缺少自动化的交付流程：

- **没有 CI/CD 集成** — 代码验证通过后，需要手动推送、创建 PR、等待 CI
- **没有 Changelog** — 变更记录散落在 task JSON 和 git commits 中，无法汇总
- **没有版本管理** — 没有自动 semver bump，没有 git tag
- **没有 Release Notes** — 最终交付缺少面向用户的发布文档
- **没有交付门禁** — 所有 task 完成后没有最终的"可交付"检查

---

## 2. 设计目标

| 目标 | 描述 |
|------|------|
| **全自动化** | 验证通过 → 创建分支 → 推送 → PR → CI → 合并，一键完成 |
| **可追溯** | 每个 PR 自动关联 task 记录、验证分数、变更详情 |
| **可配置** | 支持不同的 CI 平台和交付流程（GitHub, GitLab, 自定义） |
| **安全** | 交付前执行最终门禁检查，确保无遗漏 |

---

## 3. 核心抽象

### 3.1 DeliveryPipeline

```rust
/// 交付管道 — 验证通过后的自动化交付流程
pub struct DeliveryPipeline {
    config: DeliveryConfig,
    git: GitOperations,
    task_store: TaskStore,
    workspace: Workspace,
}

/// 交付结果
pub struct DeliveryResult {
    /// 分支名称
    pub branch: String,
    /// PR URL（如果创建成功）
    pub pr_url: Option<String>,
    /// Changelog 内容
    pub changelog: String,
    /// 版本号（如果执行了 bump）
    pub version: Option<String>,
    /// CI 状态
    pub ci_status: Option<CiStatus>,
    /// 交付时间
    pub delivered_at: DateTime<Utc>,
}
```

### 3.2 DeliveryConfig

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryConfig {
    /// 是否启用自动 PR 创建
    pub auto_pr: bool,

    /// 是否自动生成 Changelog
    pub auto_changelog: bool,

    /// 是否自动 bump 版本
    pub auto_version_bump: bool,

    /// Git 平台
    pub platform: GitPlatform,

    /// CI 配置
    pub ci: Option<CiConfig>,

    /// 交付前的最终门禁检查列表
    pub gate_checks: Vec<GateCheck>,

    /// 分支命名模板
    pub branch_template: String,

    /// PR 模板
    pub pr_template: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GitPlatform {
    GitHub,
    GitLab,
    Gitea,
    Bitbucket,
    Custom { cli_command: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiConfig {
    /// CI 平台
    pub platform: CiPlatform,
    /// 是否等待 CI 通过后才标记交付成功
    pub block_on_ci: bool,
    /// CI 超时时间
    pub timeout: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CiPlatform {
    GitHubActions,
    GitLabCI,
    CircleCI,
    Custom { check_command: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateCheck {
    pub name: String,
    pub check_type: GateCheckType,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GateCheckType {
    /// 运行命令并检查退出码
    Command { command: String, expected_exit_code: i32 },
    /// 检查文件存在
    FileExists { path: String },
    /// 检查所有 task 验证分数高于阈值
    MinVerificationScore { min_score: f64 },
    /// 检查没有未提交的变更
    CleanWorkingTree,
    /// 检查目标分支可以 fast-forward 合并
    CanFastForward { target_branch: String },
}

impl Default for DeliveryConfig {
    fn default() -> Self {
        Self {
            auto_pr: true,
            auto_changelog: true,
            auto_version_bump: false,
            platform: GitPlatform::GitHub,
            ci: Some(CiConfig {
                platform: CiPlatform::GitHubActions,
                block_on_ci: true,
                timeout: Duration::from_secs(600),
            }),
            gate_checks: vec![
                GateCheck {
                    name: "all_tasks_verified".into(),
                    check_type: GateCheckType::MinVerificationScore { min_score: 70.0 },
                    required: true,
                },
                GateCheck {
                    name: "clean_tree".into(),
                    check_type: GateCheckType::CleanWorkingTree,
                    required: true,
                },
            ],
            branch_template: "zcode/{{date}}-{{task_summary}}".into(),
            pr_template: None,
        }
    }
}
```

---

## 4. 交付流程

```
    ┌───────────────────────────────────────────────┐
    │          所有 Task 验证通过                      │
    └───────────────────────┬───────────────────────┘
                            │
    ┌───────────────────────▼───────────────────────┐
    │           最终门禁检查 (Gate Checks)             │
    │  • 所有 task 分数 ≥ 70                         │
    │  • Working tree clean                          │
    │  • cargo test 全量通过                          │
    │  • cargo clippy 无 warning                     │
    └───────────┬───────────────────┬───────────────┘
           Pass │                   │ Fail
    ┌───────────▼──────────┐  ┌─────▼────────────────┐
    │  Changelog 生成       │  │  报告失败原因         │
    │  从 task records 汇总 │  │  返回执行层修复       │
    └───────────┬──────────┘  └──────────────────────┘
                │
    ┌───────────▼──────────┐
    │  版本 Bump (可选)     │
    │  根据 task 类型判断    │
    │  major / minor / patch│
    └───────────┬──────────┘
                │
    ┌───────────▼──────────┐
    │  创建分支 + 提交      │
    │  git checkout -b      │
    │  git add + commit     │
    └───────────┬──────────┘
                │
    ┌───────────▼──────────┐
    │  推送到远程            │
    │  git push -u origin   │
    └───────────┬──────────┘
                │
    ┌───────────▼──────────┐
    │  创建 PR              │
    │  gh pr create / glab  │
    │  包含: task列表,       │
    │  验证分数, changelog   │
    └───────────┬──────────┘
                │
    ┌───────────▼──────────┐
    │  触发 / 等待 CI       │
    │  监控 CI 状态         │
    │  超时则报警           │
    └───────────┬──────────┘
                │
    ┌───────────▼──────────┐
    │  生成 Release Notes   │
    │  汇总所有交付物        │
    └──────────────────────┘
```

---

## 5. Changelog 生成设计

### 5.1 ChangelogGenerator

```rust
pub struct ChangelogGenerator;

impl ChangelogGenerator {
    /// 从 task records 和 git commits 生成 changelog
    pub fn generate(
        tasks: &[TaskRecord],
        commits: &[String],
        diff: &DiffContext,
    ) -> String {
        let mut sections: HashMap<ChangeCategory, Vec<ChangeEntry>> = HashMap::new();

        for task in tasks {
            let category = Self::categorize_task(task);
            let entry = ChangeEntry {
                description: task.task.clone(),
                score: task.final_score,
                files_changed: diff.changed_file_names(),
            };
            sections.entry(category).or_default().push(entry);
        }

        Self::format_changelog(&sections)
    }

    fn categorize_task(task: &TaskRecord) -> ChangeCategory {
        let desc = task.task.to_lowercase();
        if desc.contains("fix") || desc.contains("bug") || desc.contains("patch") {
            ChangeCategory::BugFixes
        } else if desc.contains("breaking") || desc.contains("remove") || desc.contains("deprecat") {
            ChangeCategory::BreakingChanges
        } else if desc.contains("add") || desc.contains("new") || desc.contains("implement") || desc.contains("feature") {
            ChangeCategory::Features
        } else if desc.contains("refactor") || desc.contains("improv") || desc.contains("optim") {
            ChangeCategory::Improvements
        } else if desc.contains("test") || desc.contains("doc") {
            ChangeCategory::Other
        } else {
            ChangeCategory::Features
        }
    }

    fn format_changelog(sections: &HashMap<ChangeCategory, Vec<ChangeEntry>>) -> String {
        let mut md = String::from("# Changelog\n\n");

        for category in &[
            ChangeCategory::BreakingChanges,
            ChangeCategory::Features,
            ChangeCategory::BugFixes,
            ChangeCategory::Improvements,
            ChangeCategory::Other,
        ] {
            if let Some(entries) = sections.get(category) {
                md.push_str(&format!("## {}\n\n", category.title()));
                for entry in entries {
                    md.push_str(&format!("- {} (score: {:.0}/100)\n",
                        entry.description,
                        entry.score.unwrap_or(0.0)));
                }
                md.push('\n');
            }
        }

        md
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum ChangeCategory {
    Features,
    BugFixes,
    BreakingChanges,
    Improvements,
    Other,
}

impl ChangeCategory {
    fn title(&self) -> &str {
        match self {
            Self::Features => "Features",
            Self::BugFixes => "Bug Fixes",
            Self::BreakingChanges => "Breaking Changes",
            Self::Improvements => "Improvements",
            Self::Other => "Other Changes",
        }
    }
}
```

---

## 6. 版本管理设计

### 6.1 VersionManager

```rust
pub struct VersionManager;

impl VersionManager {
    /// 根据任务类型判断版本 bump 类型
    pub fn detect_bump_type(tasks: &[TaskRecord]) -> BumpType {
        let desc_all: String = tasks.iter()
            .map(|t| t.task.to_lowercase())
            .collect::<Vec<_>>()
            .join(" ");

        if desc_all.contains("breaking") || desc_all.contains("remove api") {
            BumpType::Major
        } else if desc_all.contains("new feature") || desc_all.contains("add ") || desc_all.contains("implement") {
            BumpType::Minor
        } else {
            BumpType::Patch
        }
    }

    /// 执行版本 bump
    pub fn bump_version(project_root: &Path, bump_type: BumpType) -> Result<String> {
        let new_version = if project_root.join("Cargo.toml").exists() {
            Self::bump_cargo_version(project_root, bump_type)?
        } else if project_root.join("package.json").exists() {
            Self::bump_npm_version(project_root, bump_type)?
        } else {
            return Err(ZcodeError::InternalError("No version file found".into()));
        };

        // 创建 git tag
        let tag = format!("v{}", new_version);
        std::process::Command::new("git")
            .args(["tag", &tag])
            .current_dir(project_root)
            .output()?;

        Ok(new_version)
    }

    fn bump_cargo_version(root: &Path, bump: BumpType) -> Result<String> {
        let cargo_toml = root.join("Cargo.toml");
        let content = std::fs::read_to_string(&cargo_toml)?;
        // 解析当前版本，bump，写回
        // ... semver 解析逻辑 ...
        todo!()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BumpType {
    Major,
    Minor,
    Patch,
}
```

---

## 7. PR 自动化设计

### 7.1 PullRequestCreator

```rust
pub struct PullRequestCreator {
    platform: GitPlatform,
}

impl PullRequestCreator {
    /// 创建 PR
    pub async fn create(&self, opts: PrOptions) -> Result<PrResult> {
        let cmd = match &self.platform {
            GitPlatform::GitHub => self.gh_pr_create(&opts),
            GitPlatform::GitLab => self.glab_mr_create(&opts),
            GitPlatform::Gitea => self.gitea_pr_create(&opts),
            GitPlatform::Custom { cli_command } => self.custom_pr_create(cli_command, &opts),
        };

        let output = cmd.output()?;
        if !output.status.success() {
            return Err(ZcodeError::InternalError(
                format!("PR creation failed: {}", String::from_utf8_lossy(&output.stderr))
            ));
        }

        // 解析 PR URL 从输出
        let url = Self::parse_pr_url(&String::from_utf8_lossy(&output.stdout));
        Ok(PrResult { url })
    }

    fn gh_pr_create(&self, opts: &PrOptions) -> std::process::Command {
        let mut cmd = std::process::Command::new("gh");
        cmd.args(["pr", "create",
            "--title", &opts.title,
            "--body", &opts.body,
            "--base", &opts.base_branch,
        ]);
        if let Some(labels) = &opts.labels {
            cmd.arg("--label").arg(labels.join(","));
        }
        if opts.draft {
            cmd.arg("--draft");
        }
        cmd
    }

    /// 构建 PR body
    pub fn build_pr_body(tasks: &[TaskRecord], changelog: &str, scores: &[(String, f64)]) -> String {
        let mut body = String::new();

        body.push_str("## Summary\n\n");
        body.push_str(&format!("{} tasks completed.\n\n", tasks.len()));

        body.push_str("## Task Details\n\n");
        body.push_str("| Task | Score | Status |\n");
        body.push_str("|------|-------|--------|\n");
        for task in tasks {
            let score = task.final_score.map(|s| format!("{:.0}/100", s)).unwrap_or("-".into());
            body.push_str(&format!("| {} | {} | {} |\n",
                &task.task.chars().take(60).collect::<String>(),
                score,
                &task.status.to_string()));
        }

        body.push_str("\n\n## Changelog\n\n");
        body.push_str(changelog);

        body.push_str("\n\n---\n");
        body.push_str("Generated by [zcode](https://github.com/user/zcode)");

        body
    }
}

pub struct PrOptions {
    pub title: String,
    pub body: String,
    pub base_branch: String,
    pub labels: Option<Vec<String>>,
    pub draft: bool,
}

pub struct PrResult {
    pub url: String,
}
```

---

## 8. CI 集成设计

### 8.1 CiMonitor

```rust
pub struct CiMonitor {
    platform: CiPlatform,
    timeout: Duration,
}

impl CiMonitor {
    /// 等待 CI 通过
    pub async fn wait_for_ci(&self, repo: &str, pr_number: u32) -> Result<CiStatus> {
        let start = Instant::now();

        loop {
            let status = self.check_ci_status(repo, pr_number).await?;
            match status {
                CiStatus::Passed => return Ok(CiStatus::Passed),
                CiStatus::Failed { reason } => return Ok(CiStatus::Failed { reason }),
                CiStatus::Running => {
                    if start.elapsed() > self.timeout {
                        return Ok(CiStatus::Timeout);
                    }
                    tokio::time::sleep(Duration::from_secs(30)).await;
                }
            }
        }
    }

    async fn check_ci_status(&self, repo: &str, pr: u32) -> Result<CiStatus> {
        match &self.platform {
            CiPlatform::GitHubActions => {
                let output = std::process::Command::new("gh")
                    .args(["pr", "checks", &pr.to_string(), "--repo", repo])
                    .output()?;
                // 解析输出...
                Ok(CiStatus::Running)
            }
            _ => Ok(CiStatus::Running),
        }
    }
}

#[derive(Debug, Clone)]
pub enum CiStatus {
    Passed,
    Failed { reason: String },
    Running,
    Timeout,
    NotAvailable,
}
```

---

## 9. 配置方案

```toml
[delivery]
auto_pr = true
auto_changelog = true
auto_version_bump = false
branch_template = "zcode/{{date}}-{{summary}}"
base_branch = "main"

[delivery.platform]
type = "github"  # github | gitlab | gitea | custom

[delivery.ci]
type = "github_actions"
block_on_ci = true
timeout_secs = 600

[[delivery.gate_checks]]
name = "all_tasks_verified"
type = "min_verification_score"
min_score = 70.0
required = true

[[delivery.gate_checks]]
name = "clean_tree"
type = "clean_working_tree"
required = true

[[delivery.gate_checks]]
name = "full_test_suite"
type = "command"
command = "cargo test"
expected_exit_code = 0
required = true
```

---

## 10. 文件组织

```
src/
└── delivery/
    ├── mod.rs              — DeliveryPipeline 公共接口
    ├── config.rs           — DeliveryConfig, GitPlatform, CiConfig
    ├── changelog.rs        — ChangelogGenerator
    ├── version.rs          — VersionManager, BumpType
    ├── pull_request.rs     — PullRequestCreator (gh, glab)
    ├── ci_monitor.rs       — CiMonitor, CiStatus
    └── gate.rs             — GateCheck 执行器
```

---

## 11. 实现路线图

### Phase 1: 核心框架 + Changelog（1 周）
- [ ] 创建 `delivery/` 模块骨架
- [ ] 实现 `DeliveryConfig` 配置
- [ ] 实现 `ChangelogGenerator`（从 task records 汇总）
- [ ] 实现最终门禁检查（Gate Checks）

### Phase 2: PR 自动化 + 版本管理（1 周）
- [ ] 实现 `PullRequestCreator`（gh pr create）
- [ ] 实现 PR body 模板生成
- [ ] 实现 `VersionManager`（semver bump + git tag）

### Phase 3: CI 集成（1 周）
- [ ] 实现 `CiMonitor`（gh pr checks）
- [ ] 实现 CI 超时和失败处理
- [ ] GitLab CI 支持

### Phase 4: Release Notes + 完善（1 周）
- [ ] 实现 Release Notes 自动生成
- [ ] 端到端集成测试
- [ ] 文档和示例配置
