<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-29 | Updated: 2026-03-29 -->

# templates

## Purpose
Document templates used by `zcode docs init` to scaffold new projects. Each template provides the required structure for the Harness Engineering docs convention.

## Key Files
| File | Description |
|------|-------------|
| `README.md` | Explains the template system |
| `prd.template.md` | Template for Product Requirements Documents (goals, non-goals, user stories, acceptance criteria) |
| `spec.template.md` | Template for coding specifications (tech stack, file structure, conventions) |
| `tasks.template.md` | Template for implementation task lists |
| `validation.template.md` | Template for quality gates and acceptance validation |
| `review-checklist.template.md` | Template for code review checklists |
| `skill.template.md` | Template for skill files with YAML frontmatter |

## For AI Agents

### Working In This Directory
- Templates are plain markdown with placeholder text (TODO markers)
- Used by `src/docs/mod.rs::generate_docs_scaffold()` to create initial project docs
- Safe to modify templates — changes apply to new scaffolds only

<!-- MANUAL: -->
