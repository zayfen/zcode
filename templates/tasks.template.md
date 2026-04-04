# Tasks: {{FEATURE_NAME}}
<!--
  HARNESS ENGINEERING · Step 3 · Tasks
  File: docs/tasks/NNN-feature-name.tasks.md
  
  Instructions:
  - Each task MUST be atomic (1 engineer / 1 AI session can complete it)
  - Mark parallelism: tasks in the same "group" can run concurrently
  - At least 3 tasks are required for `zcode docs check` to pass
  - Use checkboxes: [ ] = pending, [/] = in progress, [x] = done
-->

**Date:** {{YYYY-MM-DD}}
**Status:** Draft | Active | Complete
**Linked Spec:** [Spec](../specs/{{feature-name}}.spec.md)

---

## Dependency Graph

```
[Task-01] ──► [Task-03] ──► [Task-06]
[Task-02] ──► [Task-04] ──► [Task-07]
              [Task-05] ──────────────► [Task-08]
```

> Tasks at the same level can run in parallel.

---

## Group A — Foundation  *(parallel)*

- [ ] **Task-01 [A]:** {{Atomic task description}}
  - Input: {{what is needed to start}}
  - Output: {{what is produced}}
  - Acceptance: {{how to verify it's done}}

- [ ] **Task-02 [A]:** {{Atomic task description}}
  - Input: {{what is needed}}
  - Output: {{what is produced}}
  - Acceptance: {{how to verify}}

## Group B — Core Logic  *(parallel, requires Group A)*

- [ ] **Task-03 [B]:** {{Atomic task description}}
  - Input: output of Task-01
  - Output: {{what is produced}}
  - Acceptance: {{how to verify}}

- [ ] **Task-04 [B]:** {{Atomic task description}}
  - Input: output of Task-02
  - Output: {{what is produced}}
  - Acceptance: {{how to verify}}

- [ ] **Task-05 [B]:** {{Atomic task description}}
  - Input: {{what is needed}}
  - Output: {{what is produced}}
  - Acceptance: {{how to verify}}

## Group C — Integration  *(sequential, requires Group B)*

- [ ] **Task-06 [C]:** {{Integrate components}}
  - Input: outputs of Group B
  - Output: {{integrated result}}
  - Acceptance: {{integration test passes}}

## Group D — Test & Doc  *(parallel with Group C)*

- [ ] **Task-07 [D]:** Write unit tests for {{component}}
  - Acceptance: `cargo test {{module}}` — 0 failures

- [ ] **Task-08 [D]:** Update `docs/` for this feature
  - Acceptance: `zcode docs check` passes

---

## Task Status Summary

| Task | Group | Status | Assignee |
|---|---|---|---|
| Task-01 | A | ⬜ Pending | — |
| Task-02 | A | ⬜ Pending | — |
| Task-03 | B | ⬜ Pending | — |
| Task-04 | B | ⬜ Pending | — |
| Task-05 | B | ⬜ Pending | — |
| Task-06 | C | ⬜ Pending | — |
| Task-07 | D | ⬜ Pending | — |
| Task-08 | D | ⬜ Pending | — |
