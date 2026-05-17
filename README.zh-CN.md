# zcode &middot; [![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE) [![Rust](https://img.shields.io/badge/rust-1.91%2B-orange)](https://www.rust-lang.org)

**zcode** 是一个用 Rust 构建的模块化 AI 编程智能体 CLI。它在分层 Cargo 工作空间中编排多智能体工作流——规划、ReAct 编码、审查和自我学习——在终端中提供确定性、可审计且高吞吐量的软件工程能力。

---

## 为什么选择 zcode

大多数 AI 编程工具将智能体视为黑盒：输入提示，输出代码。zcode 将智能体生命周期分解为显式、可检查的阶段。每个计划、每次工具调用、每个审查决策都作为结构化会话数据持久化存储，建立索引以供检索，并可事后审计。

- **严格的智能体流程** — 四阶段工作流（规划 &rarr; 编码 &rarr; 审查 &rarr; 学习），具有有界重试循环，而非单一不透明的推理过程。
- **默认清新的上下文** — 每个提示通过本地 LanceDB 向量索引与历史轮次匹配。不相关的问题从零开始；相关的问题仅接收相关的历史记录。
- **分层架构** — 七个专注的 crate，依赖方向严格。无需触及编排核心即可替换 LLM 提供商、添加功能或更换 TUI。
- **离线检索能力** — 会话上下文选择和压缩完全无需 LLM 调用。向量索引从 JSONL 日志派生，可随时重建。
- **兼容 OpenAI** — 适用于任何支持 OpenAI 聊天补全协议的服务商。

---

## 架构概览

```
src/main.rs          二进制入口
crates/zcode_cli     CLI 命令分发 (clap)
crates/zcode_ui      TUI 渲染 (Ratatui)
crates/zcode_requirements   文档脚手架、校验、任务存储
crates/zcode_orchestration  智能体图谱：规划 → 编码 → 审查 → 学习
crates/zcode_llm_provider   OpenAI 兼容的聊天补全
crates/zcode_capabilities   技能、MCP 工具、共享上下文
crates/zcode_session        JSONL 会话 + LanceDB 向量索引
crates/zcode_core           共享 DTO、错误类型、配置
```

依赖严格向下流动。`zcode_core` 是叶子节点；不存在循环依赖。

智能体图谱由根编排器协调四个专门角色运行：

| 智能体 | 职责 |
|---|---|
| **编排器 (Orchestrator)** | 根协调器。路由工作，管理重试门槛。 |
| **规划器 (Planner)** | 读取标准化需求文档并生成可执行计划。 |
| **编码器 (Coder)** | ReAct 循环：推理、调用工具、观察、重复、报告。 |
| **审查器 (Reviewer)** | 红/绿测试关卡。失败时将结果反馈给编码器重试。 |
| **自学习 (Self-Learning)** | 将反复出现的错误总结为持久化的修正笔记。 |

---

## 快速开始

### 前置条件

- Rust 1.91+（LanceDB 最低要求）
- 兼容 OpenAI 的 API 端点

### 构建

```bash
cargo build --workspace
cargo test --workspace
```

### 配置

```bash
export ZCODE_BASE_URL="https://api.openai.com/v1"
export ZCODE_API_KEY="sk-..."
export ZCODE_MODEL="gpt-4o"
export ZCODE_FAST_MODEL="gpt-4o-mini"   # 可选，用于简单任务
```

### 启动

```bash
# 交互式 TUI 聊天
target/debug/zcode chat

# 初始化需求文档脚手架
target/debug/zcode docs init

# 通过完整智能体工作流执行任务
target/debug/zcode run "实现文档中的下一个任务"

# 并行执行所有待处理任务
target/debug/zcode task run-all -j 2
```

> **启动性能说明：** 使用编译后的二进制文件（`target/debug/zcode chat`）来衡量启动延迟。`cargo run -- chat` 包含了 Cargo 依赖图检查、增量编译和链接开销——不能代表 zcode 的实际运行时延迟。

---

## 会话模型

每个交互式聊天会话存储为 `.zcode/sessions/` 下的一个仅追加 JSONL 文件。`.zcode/session-index/` 下的派生 LanceDB 索引支持相关轮次检索。

| 设计属性 | 优势 |
|---|---|
| JSONL 作为唯一真相源 | 人类可读、仅追加、易于恢复 |
| LanceDB 作为派生索引 | 真正的向量最近邻搜索；可丢弃和重建 |
| 默认清新上下文 | 不相关的提示从零开始——无跨主题污染 |
| 匹配轮次注入 | 仅相关的用户/助手轮次进入 LLM |
| 无 LLM 检索路径 | 上下文选择离线工作；LLM 调用仅用于推理 |
| 可选上下文保护 | 历史记录被标记为可选背景；当前提示具有权威性 |

---

## 能力

### MCP 工具

通过模型上下文协议（Model Context Protocol）发现和执行工具。在 `.zcode/config.toml` 中配置服务器：

```toml
[[mcp_servers]]
name = "filesystem"
command = "mcp-server-filesystem"
args = ["/workspace"]
auto_start = true
```

运行时使用 `-M` 挂载临时服务器：

```bash
zcode -M "mcp-server-filesystem /workspace" run "检查项目"
```

### 技能

项目技能位于 `docs/skills/<name>/SKILL.md`。运行时 zcode 通过对名称、描述、触发器和正文的通用相关性评分为每个提示选择技能。不相关的技能不会被注入 LLM 上下文。

```markdown
---
name: rust-conventions
description: zcode 的 Rust 编码规范
priority: high
triggers: rust, cargo, clippy, test
---

生产环境错误请使用 `ZcodeError`。
```

---

## 命令

| 命令 | 用途 |
|---|---|
| `zcode chat` | 启动交互式 TUI |
| `zcode run <描述>` | 通过完整智能体工作流执行任务 |
| `zcode docs init` | 创建标准化需求文档脚手架 |
| `zcode docs check` | 按 zcode 规范验证文档 |
| `zcode task list` | 列出持久化的任务记录 |
| `zcode task run-all -j N` | N 路并行执行所有待处理任务 |
| `zcode feed <路径>` | 将原始需求导入文档结构 |

---

## 项目结构

```
zcode/
├── crates/                  # 工作空间 crate（分层架构）
├── src/                     # 二进制入口 (main.rs) + 重导出 (lib.rs)
├── docs/                    # 需求文档、PRD、规格说明
├── templates/               # 文档模板
├── tests/                   # 集成测试
├── ARCHITECTURE.md          # 详细架构参考
├── USAGE.md                 # 使用参考
└── README.md                # English README
```

---

## 开发

```bash
cargo check --workspace
cargo test --workspace --lib
cargo test --test cli_test
cargo test --test registry_test
cargo test --test reviewer_integration
```

最低 Rust 版本要求 1.91.0。所有异步 trait 使用 `async_trait`。配置通过 `Settings` 和 `ProjectConfig` 流转。

---

## 文档

- [ARCHITECTURE.md](ARCHITECTURE.md) — 完整架构及数据流图
- [USAGE.md](USAGE.md) — 详细 CLI 使用和配置
- [docs/architecture.zh-CN.md](docs/architecture.zh-CN.md) — 中文架构说明
- [README.md](README.md) — 英文 README

---

## 许可证

MIT &copy; zcode contributors
