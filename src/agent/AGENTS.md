<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-29 | Updated: 2026-03-29 -->

# agent

## Purpose
Multi-agent orchestration system with tokio-based message passing. Implements Orchestrator (task routing), Planner (task decomposition), Coder (code generation), Reviewer (static analysis), and an AgentLoop for conversation + tool call management.

## Key Files
| File | Description |
|------|-------------|
| `mod.rs` | Module declarations and public re-exports |
| `types.rs` | Core types: `AgentId`, `AgentState`, `AgentType`, `Task`, `TaskPriority`, `TaskResult`, `AgentMessage` |
| `traits.rs` | `AgentTrait` — the trait all agents implement |
| `bus.rs` | `MessageBus` (tokio mpsc), `BusHandle`, `BusDispatcher` for inter-agent communication |
| `orchestrator.rs` | `OrchestratorAgent` — receives user requests, routes to specialist agents |
| `planner.rs` | `PlannerAgent` — breaks complex tasks into ordered subtasks |
| `coder.rs` | `CoderAgent` — writes/edits code via LLM + tools |
| `reviewer.rs` | `ReviewerAgent` — static analysis of code diffs (Logic, Security, Performance, Style, Testing) |
| `loop_exec.rs` | `AgentLoop` — conversation state, tool call dispatch, LLM response parsing, token counting |

## For AI Agents

### Working In This Directory
- All agents implement `AgentTrait` from `traits.rs`
- Communication between agents goes through the `MessageBus` (tokio mpsc channels)
- `AgentLoop` is the core execution loop: send messages to LLM, parse tool calls, execute, repeat
- The `ReviewerAgent` has 5 review categories — see `ARCHITECTURE.md` for details

### Testing Requirements
- Tests are inline in each file
- Agent behavior can be tested by mocking the `LlmProvider` trait

### Common Patterns
- `BusHandle` gives each agent its own sender/receiver pair
- `ConversationMessage` tracks role + content + timestamp for the agent loop
- `LoopConfig` controls iteration limits and token budgets

## Dependencies

### Internal
- `crate::llm` — LLM provider for chat completions
- `crate::tools` — Tool registry for tool call execution
- `crate::memory` — Context assembly for prompts
- `crate::error` — Error types

### External
- `tokio` (mpsc channels), `async-trait`, `uuid`, `serde_json`

<!-- MANUAL: -->
