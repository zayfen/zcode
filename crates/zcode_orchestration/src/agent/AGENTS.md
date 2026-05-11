<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-29 | Updated: 2026-05-11 -->

# agent

## Purpose
Multi-agent orchestration system with graph-based workflows and optional tokio message passing. Implements Orchestrator (root coordination), Planner (task decomposition), Coder (ReAct execution), Reviewer (review/test feedback), SelfLearning (mistake-book entries), and `AgentLoop` for conversation + tool call management.

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
| `self_learning.rs` | `SelfLearningAgent` — summarizes recurring failures and corrections |
| `loop_exec.rs` | `AgentLoop` — conversation state, tool call dispatch, LLM response parsing, token counting |

## For AI Agents

### Working In This Directory
- All agents implement `AgentTrait` from `traits.rs`
- `StateGraph` is the primary workflow engine for CLI task execution
- Communication between standalone agents can go through the `MessageBus` (tokio mpsc channels)
- `AgentLoop` is the core ReAct loop: send messages to LLM, parse tool calls, execute, observe, repeat
- Coder behavior must remain ReAct-based
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
- `zcode_llm_provider` — OpenAI-compatible LLM provider for chat completions
- `zcode_capabilities` — Tool registry and MCP/capability tool call execution
- `zcode_core` — Shared DTOs and error types

### External
- `tokio` (mpsc channels), `async-trait`, `uuid`, `serde_json`

<!-- MANUAL: -->
