---
name: rust-conventions
description: Rust coding conventions for the zcode project
priority: high
---

# Rust Conventions

## Error Handling

- Always use `ZcodeError` for all errors in production code
- Never use `unwrap()` or `expect()` outside of `#[cfg(test)]` blocks
- Use `?` operator for error propagation; avoid explicit `match` where `?` suffices
- Error messages must be end-user actionable, not internal stack traces

## Code Structure

- All public structs, traits and functions must have `///` doc comments
- Keep functions under 50 lines; refactor larger functions into helpers
- Prefer `impl Trait` return types over `Box<dyn Trait>` where possible
- Use `Arc<T>` for shared ownership, `Rc<T>` only in single-threaded code

## Testing

- Minimum coverage: every public function has at least one unit test
- Test files live adjacent to source (`#[cfg(test)] mod tests { ... }`)
- Use `tempfile::TempDir` for file-system tests; never write to `/tmp` directly
- Test names must describe what they verify: `test_<function>_<scenario>`

## Async Code

- Never block the async executor with synchronous I/O in `async fn`
- Use `tokio::fs` instead of `std::fs` in async contexts
- Spawn blocking tasks with `tokio::task::spawn_blocking`
