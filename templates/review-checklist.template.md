# Review Checklist
<!--
  HARNESS ENGINEERING · Step 6 · Review
  File: docs/review-checklist.md  (project-level, shared across features)
  
  Instructions:
  - `zcode docs check` requires ≥ 3 `- [ ]` items
  - Reviewers tick items during the review session
  - Mark items N/A with a note rather than deleting them
-->

**Feature reviewed:** {{FEATURE_NAME}}
**Reviewer:** {{NAME}}
**Date:** {{YYYY-MM-DD}}
**Linked PR/commit:** {{link}}

---

## 1 · Requirements Completeness

- [ ] All PRD Goals are addressed in the implementation
- [ ] All PRD Acceptance Criteria have corresponding tests
- [ ] Non-Goals have not been accidentally implemented
- [ ] Open Questions from PRD are resolved (or deferred with ticket)

## 2 · Code Quality

- [ ] No `unwrap()` / `expect()` in non-test production code
- [ ] All public items have `///` doc comments
- [ ] Error messages are end-user actionable (not internal jargon)
- [ ] No commented-out dead code committed
- [ ] No TODO/FIXME without a linked issue

## 3 · Architecture Conformance

- [ ] Code follows the patterns in `docs/specs/coding.spec.md`
- [ ] New types/modules placed in the correct layer
- [ ] No new circular dependencies introduced
- [ ] Dependency injection used where testability requires it

## 4 · Testing

- [ ] Unit tests cover the happy path
- [ ] Unit tests cover at least 2 error/edge cases per function
- [ ] Integration tests cover the end-to-end flow
- [ ] Tests are deterministic (no flakiness, no network/time dependencies)
- [ ] `cargo test` passes with 0 failures

## 5 · Documentation

- [ ] `zcode docs check` passes
- [ ] CHANGELOG.md updated under `[Unreleased]`
- [ ] Any new CLI flags/commands are documented in `docs/specs/`
- [ ] API.md updated for new public API surface

## 6 · Security & Safety

- [ ] No secrets or credentials hardcoded
- [ ] User-supplied paths are not traversed without sanitisation
- [ ] Shell commands are not constructed with user input (no injection)

## 7 · Performance (if applicable)

- [ ] No blocking I/O on the async executor
- [ ] Large allocations are avoided in hot paths
- [ ] Performance targets in `docs/validation.md` are met

---

## Summary

> Summarise the review outcome in 2–3 sentences.

{{Summary text}}

**Decision:** ✅ Approved | 🔄 Approved with minor changes | ❌ Needs rework
