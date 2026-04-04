# Spec: {{FEATURE_NAME}}
<!--
  HARNESS ENGINEERING · Step 2 · Spec
  File: docs/specs/feature-name.spec.md
  
  Instructions:
  - Derive from the PRD. Do NOT invent requirements here.
  - Every section marked (required) is checked by `zcode docs check`
  - for coding.spec.md (the project-level spec), skip the "Feature-Specific" block
-->

**Date:** {{YYYY-MM-DD}}
**Status:** Draft | In Review | Approved
**Linked PRD:** [PRD](../prd/{{NNN}}-{{feature-name}}.md)

---

## Tech Stack
<!-- (required) List languages, frameworks, key libraries with versions -->

| Layer | Choice | Version | Reason |
|---|---|---|---|
| Language | {{e.g. Rust}} | {{edition/ver}} | {{why}} |
| Async runtime | {{e.g. tokio}} | {{ver}} | {{why}} |
| HTTP client | {{e.g. reqwest}} | {{ver}} | {{why}} |
| Serialisation | {{e.g. serde}} | {{ver}} | {{why}} |

## File Structure
<!-- (required) Show the directory tree for code produced by this feature -->

```
{{project-root}}/
├── src/
│   ├── {{module}}/
│   │   ├── mod.rs          # {{purpose}}
│   │   └── {{submodule}}.rs
│   └── ...
└── tests/
    └── {{feature}}_test.rs
```

## Architecture

> Describe the component interactions. Use ASCII art or Mermaid.

```
{{ComponentA}} ──────► {{ComponentB}}
      │                     │
      ▼                     ▼
 {{ComponentC}}       {{ComponentD}}
```

## API / Interface Design

> Public types, traits, and function signatures exposed by this feature.

```rust
// Example (adjust language as needed)
pub struct {{TypeName}} {
    pub {{field}}: {{Type}},
}

pub trait {{TraitName}} {
    fn {{method}}(&self, {{param}}: {{ParamType}}) -> Result<{{ReturnType}}>;
}
```

## Data Flow

> Step-by-step narrative of how data moves through the system.

1. {{Step 1: input enters here}}
2. {{Step 2: transformed by}}
3. {{Step 3: output produced}}

## Error Handling

> How errors are represented and propagated.

| Error case | Error type | Recovery |
|---|---|---|
| {{Case 1}} | `{{ErrorVariant}}` | {{Behaviour}} |
| {{Case 2}} | `{{ErrorVariant}}` | {{Behaviour}} |

## Conventions
<!-- (required for coding.spec.md) -->

- **Naming:** {{e.g. snake_case for functions, PascalCase for types}}
- **Error handling:** {{e.g. all public fns return Result<T, ZcodeError>}}
- **Testing:** {{e.g. unit tests in same file, integration tests in tests/}}
- **Logging:** {{e.g. use tracing::{info, warn, error} macros}}
- **No panics:** {{e.g. no unwrap() in non-test code}}

## Feature Flags / Configuration

> Optional. Configuration keys this feature adds.

| Key | Type | Default | Description |
|---|---|---|---|
| `{{config.key}}` | `{{Type}}` | `{{default}}` | {{description}} |

## Open Design Decisions

| # | Decision | Options | Chosen | Reason |
|---|---|---|---|---|
| 1 | {{Decision}} | A / B | {{Choice}} | {{Reason}} |
