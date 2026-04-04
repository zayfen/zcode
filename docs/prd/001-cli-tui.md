# Feature: Zcode CLI & TUI Interface

## Goals
- 提供无缝的用户交互边界层，通过 `clap` 负责标准的命令分发与参数捕获（CLI 模式）。
- 为复杂的交互探究行为提供基于 `ratatui` 的多面板、高度拟物化的可滚屏 UI 终端体验（TUI 模式）。
- 在交互中实时透出底层 Agent 系统的存活状态、思考逻辑以及挂载的 Tools 状态集。

## Non-Goals
- 不提供完整的图形用户界面 (GUI / web 视图)。
- CLI 模式下不再包含花哨的 TUI 组件重绘，以便于适配外部管道（Piping）或脚本集成。

## User Stories
- 作为高级开发者，我期望执行 `zcode run "分析并重构这个文件"` 命令，Zcode 会在 CLI 控制台上通过流式日志反馈直接帮我做完需求并退出。
- 作为研发人员，我需要长期的编码辅导，因此我希望能执行 `zcode chat` 呼出类似 Claude Code 体验的分块终端环境。
- 当我使用 TUI 时，我期望一眼在侧边栏看穿底层挂载的所有 MCP 服务与 Agent （此时谁处于 Thinking 状态）。
- 期望 `Esc` 和 `[↑/↓]` 按键可以在 TUI 对话面板中提供自由控制权。

## System Architecture Context
本模块承接核心应用引擎 (`src/main.rs`) 的启动，并且位于应用调度的最外层。
- `src/cli`: 应用基于 `clap` 提供顶层命令如 `chat`, `run`, `version`，并进行 `execute_x` 主循环抛转操作。
- `src/tui/mod.rs` & `chat.rs`: 实现了基于 `crossterm/ratatui` 的 `TuiApp` 状态机。TUI 层将独占并锁住控制台事件环，将键盘输入代理执行到深层的 LLM / `AgentBus` 并回绘界面。

## Acceptance Criteria
- [ ] 命令行必须支持完整的 `--help` 并列举出诸如 `run`, `chat` 及其 flag (`--model`, `--mcp`) 等定义。
- [ ] TUI 启动后能够使用全屏宽带，切分出包含：聊天层、带有高亮的固定状态监控侧边栏、带响应焦点的输入区域、底部带快捷键提示的状态条。
- [ ] TUI 环境下输入栏需支持多行处理（通过 `[Alt+Enter]` 或 `[Ctrl+J]` 进行换行，`[Enter]` 进行命令提交）。
- [ ] TUI 组件 `chat` 历史在超出限定帧之后，需支持按键翻阅 (`page up/down` 或方向键)，以防止内容截断遗失。
- [ ] 相关的交互变更需维持完整的单元测试，特别是键盘事件的分叉测试需达到 90%+ 覆盖率。
