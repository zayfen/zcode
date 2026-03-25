# Validation

## Quality Gates
- [ ] `cargo test` — 0 failures
- [ ] `cargo clippy -- -D warnings` — 0 warnings
- [ ] `cargo fmt --check` — clean

## Acceptance Validation
- [ ] `zcode docs init` creates 5 scaffold files in empty directory
- [ ] `zcode docs check` passes after init
- [ ] `zcode run "task"` fails with clear error when docs/ is absent
- [ ] `zcode run --skip-docs-check "task"` bypasses validation
