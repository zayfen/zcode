# Validation
<!--
  HARNESS ENGINEERING · Step 5 · Validation
  File: docs/validation.md  (project-level, shared across all features)
  
  Instructions:
  - `## Quality Gates` section is REQUIRED by `zcode docs check`
  - Add feature-specific acceptance validation in "## Acceptance Validation"
  - Every item must be mechanically verifiable (command or manual step)
-->

**Last updated:** {{YYYY-MM-DD}}

---

## Quality Gates
<!-- (required) Automated pass/fail criteria. All must pass before merge. -->

### Automated

| Gate | Command | Expected |
|---|---|---|
| Unit tests | `cargo test` | 0 failures |
| Lint | `cargo clippy -- -D warnings` | 0 warnings |
| Format | `cargo fmt --check` | exit 0 |
| Docs validation | `zcode docs check` | ✓ passed |

### Coverage (optional)

| Target | Threshold | Command |
|---|---|---|
| Line coverage | ≥ {{80}}% | `cargo tarpaulin --out Stdout` |

---

## Acceptance Validation

> Map each PRD Acceptance Criterion to a verification step.

| AC | Feature | Verification Step | Status |
|---|---|---|---|
| AC-01 | {{Feature}} | {{manual step or command}} | ⬜ |
| AC-02 | {{Feature}} | {{manual step or command}} | ⬜ |
| AC-03 | {{Feature}} | {{manual step or command}} | ⬜ |

---

## Performance Targets (optional)

| Metric | Target | Measurement |
|---|---|---|
| Startup time | < {{100ms}} | `hyperfine 'zcode version'` |
| Memory (idle) | < {{200MB}} | `valgrind --tool=massif` |

---

## Known Failures / Exceptions

> Document any intentionally skipped gates and the reason.

| Gate | Skipped? | Reason | Expiry |
|---|---|---|---|
| {{Gate}} | No / Yes | {{reason}} | {{date}} |
