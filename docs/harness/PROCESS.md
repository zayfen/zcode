# Harness Engineering Process Guide

> Universal 7-step closed-loop workflow for AI-driven software development.
> Source: https://www.zayfen.com/posts/harness-engineering-frontend-guide/

---

## Core Philosophy

**Harness Engineering** = Structured documents drive AI → precise requirement-to-code mapping.

Problems solved:
- **Intent drift** — AI generates code that diverges from requirements
- **Quality loss** — no systematic verification
- **Collaboration chaos** — unclear agent responsibilities  
- **Doc/code disconnect** — code evolves but docs don't

---

## The 7-Step Closed Loop

```
PRD → Spec → Tasks → Implementation → Validation → Review → Iterate
```

Each step has defined inputs, outputs and AI responsibilities.

---

## Step 1 · PRD — _What_ to build

**Output:** `docs/prd/*.md`

**Required sections:**
- `## Goals` — what the feature does
- `## Non-Goals` — explicit out-of-scope items
- `## User Stories` — as a user, I want …
- `## Acceptance Criteria` — testable pass/fail conditions

**AI role:** Generate structured PRD from user input; fill boundary conditions; prioritise requirements.

---

## Step 2 · Spec — _How_ to build it

**Output:** `docs/specs/coding.spec.md` + `docs/specs/<feature>.spec.md`

**Required sections:**
- `## Tech Stack` — language, framework, key libraries
- `## File Structure` — directory layout
- `## Conventions` — naming, error handling, testing patterns

**AI role:** Derive technical design from PRD; select stack; define component hierarchy; define API contracts.

---

## Step 3 · Tasks — Atomic execution units

**Output:** `docs/tasks/<feature>.tasks.md`

**Key principles:**
- **Atomic** — each task independently completable
- **Parallel** — mark tasks safe for concurrent execution
- **Dependency-aware** — explicit ordering where required

**AI role:** Break Spec into tasks; identify dependencies; mark parallel groups.

---

## Step 4 · Implementation — Build

Multiple agents execute tasks from Step 3 in parallel.

---

## Step 5 · Validation — Verify

**Output:** `docs/validation.md`

**Required sections:**
- `## Quality Gates` — automated pass/fail criteria (tests, lint, format)
- `## Acceptance Validation` — PRD Acceptance Criteria manually verified

**AI role:** Run all quality gates; report failures with suggested fixes.

---

## Step 6 · Review — Human + AI audit

**Output:** `docs/review-checklist.md`

Combined human + AI review covering: code quality, architecture conformance, test coverage.

Minimum 3 checklist items required.

---

## Step 7 · Iterate — Feed back into the loop

When a change is required, determine the earliest affected step and update from there:

| Change type | Re-enter at |
|---|---|
| New requirement | PRD |
| Technical redesign | Spec |
| Additional tasks only | Tasks |
| Bug fix | Implementation |
| Test gap | Validation |

---

## Directory Convention (enforced by `zcode docs check`)

```
docs/
├── prd/
│   └── <NNN>-<feature>.md        ← Goals + Acceptance Criteria required
├── specs/
│   └── coding.spec.md            ← Tech Stack + File Structure required
├── tasks/
│   └── <NNN>-<feature>.tasks.md  ← ≥3 task items required
├── validation.md                  ← Quality Gates section required
└── review-checklist.md            ← ≥3 - [ ] items required
```

Run `zcode docs init` to generate this skeleton automatically.
Run `zcode docs check` to validate at any time.
