# Review Checklist

## Code Quality
- [ ] No `unwrap()`/`expect()` in production paths
- [ ] `DocsError` messages are actionable end-user facing
- [ ] All new public API items have `///` doc comments

## Architecture
- [ ] `DocsValidator` has no dependency on TUI or LLM layers
- [ ] Validation runs synchronously (no async) — stays simple
- [ ] `generate_docs_scaffold` is idempotent (safe to re-run)

## Testing
- [ ] Every validation rule has a dedicated unit test
- [ ] Scaffold test verifies generated docs pass validation
- [ ] CLI test uses `skip_docs_check: true` where no docs/ exists
