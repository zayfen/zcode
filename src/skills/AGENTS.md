<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-29 | Updated: 2026-03-29 -->

# skills

## Purpose
Skills system that loads markdown skill files from `docs/skills/` and injects them into the agent's system prompt. Skills are specialized instructions that the AI agent must follow. They are sorted by priority (high > medium > low) and prepended to the system prompt.

## Key Files
| File | Description |
|------|-------------|
| `mod.rs` | `Skill` struct, `SkillPriority` enum, `SkillsLoader` — loads skills from `<project_root>/docs/skills/*/SKILL.md`, parses YAML frontmatter, and builds enhanced system prompts |

## For AI Agents

### Working In This Directory
- Skills live in subdirectories: `docs/skills/<name>/SKILL.md`
- Frontmatter format: `name`, `description`, `priority` (high/medium/low)
- `SkillsLoader::build_system_prompt()` appends skills to the base prompt
- Missing `docs/skills/` directory is not an error — returns empty vec

### Testing Requirements
- Extensive inline tests covering frontmatter parsing, priority sorting, and prompt building

## Dependencies

### Internal
- None (standalone module)

### External
- None (uses only `std`)

<!-- MANUAL: -->
