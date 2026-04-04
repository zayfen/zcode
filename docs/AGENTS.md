<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-29 | Updated: 2026-03-29 -->

# docs

## Purpose
Project documentation following the Harness Engineering convention. Contains PRDs (Product Requirements), technical specs, task lists, design plans, validation criteria, review checklists, and skill definitions. Validated at runtime by the `src/docs/` module.

## Key Files
| File | Description |
|------|-------------|
| `validation.md` | Quality gates and acceptance validation criteria |
| `review-checklist.md` | Code review checklist (≥3 items required) |

## Subdirectories
| Directory | Purpose |
|-----------|---------|
| `prd/` | Product Requirements Documents |
| `specs/` | Technical specifications (coding spec required) |
| `tasks/` | Implementation task lists |
| `plans/` | Design and architecture plans |
| `harness/` | Harness Engineering process documentation |
| `skills/` | Markdown skill files injected into agent system prompt |

## For AI Agents

### Working In This Directory
- Structure is validated by `zcode::docs::DocsValidator`
- Required: `prd/` (≥1 .md), `specs/coding.spec.md`, `tasks/` (≥1 .tasks.md), `validation.md`, `review-checklist.md`
- `coding.spec.md` must have `## Tech Stack` and `## File Structure` sections
- `validation.md` must have `## Quality Gates` section
- `review-checklist.md` must have ≥3 `- [ ]` items
- Skills in `skills/*/SKILL.md` with YAML frontmatter (name, description, priority)

### Common Patterns
- Use `zcode docs init` to generate scaffolding (idempotent)
- All files support Chinese section headings as alternatives

<!-- MANUAL: -->
