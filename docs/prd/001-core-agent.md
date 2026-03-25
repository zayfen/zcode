# Feature: zcode Coding Agent — Core CLI

## Goals
- Provide a command-line coding agent backed by LLM
- Support provider-agnostic LLM integration (Anthropic, OpenAI, etc.)
- Run agent tasks autonomously via `zcode run "<task>"`
- Validate project documents before executing any task

## Non-Goals
- Visual GUI or Electron-based app
- Real-time collaborative editing

## User Stories
- As a developer, I want to run `zcode run "add error handling"` and have the agent modify files autonomously
- As a developer, I want `zcode docs init` to bootstrap my project docs
- As a developer, I want `zcode docs check` to tell me if my docs are complete

## Acceptance Criteria
- [ ] `zcode run "<task>"` executes the LLM agent loop and produces output
- [ ] `zcode docs init` creates all required docs/ files in the current directory
- [ ] `zcode docs check` reports pass/fail with actionable hints
- [ ] Running `zcode run` without valid docs/ fails with a clear error message
- [ ] `--skip-docs-check` bypasses docs validation
- [ ] All unit tests pass (`cargo test`)
