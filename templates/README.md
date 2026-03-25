# Harness Engineering Templates

Standard document templates for the **Harness Engineering** 7-step closed-loop workflow.

> Validation rules are enforced by `zcode docs check` / `src/docs/mod.rs`.

---

## Template Index

| File | Harness Step | Output Path | Validated? |
|---|---|---|---|
| [prd.template.md](prd.template.md) | **Step 1 · PRD** | `docs/prd/NNN-feature.md` | ✅ `Goals` + `Acceptance Criteria` required |
| [spec.template.md](spec.template.md) | **Step 2 · Spec** | `docs/specs/coding.spec.md` | ✅ `Tech Stack` + `File Structure` required |
| [tasks.template.md](tasks.template.md) | **Step 3 · Tasks** | `docs/tasks/NNN-feature.tasks.md` | ✅ ≥ 3 task items required |
| [validation.template.md](validation.template.md) | **Step 5 · Validation** | `docs/validation.md` | ✅ `Quality Gates` required |
| [review-checklist.template.md](review-checklist.template.md) | **Step 6 · Review** | `docs/review-checklist.md` | ✅ ≥ 3 `- [ ]` items required |

> Steps 4 (Implementation) and 7 (Iterate) produce code, not documents.

---

## Quick Start

```bash
# 1. Bootstrap scaffold (auto-generates minimal valid docs/)
zcode docs init

# 2. Copy and fill the templates for your feature
cp templates/prd.template.md              docs/prd/001-my-feature.md
cp templates/spec.template.md             docs/specs/my-feature.spec.md
cp templates/tasks.template.md            docs/tasks/001-my-feature.tasks.md

# shared files (already created by init):
#   docs/validation.md          <- update from validation.template.md
#   docs/review-checklist.md    <- update from review-checklist.template.md

# 3. Validate before running
zcode docs check

# 4. Run your task
zcode run "implement the feature"
```

---

## Variable Conventions

All templates use `{{PLACEHOLDER}}` syntax:

| Placeholder | Replace with |
|---|---|
| `{{FEATURE_NAME}}` | Human-readable feature name, e.g. `User Authentication` |
| `{{feature-name}}` | Kebab-case slug, e.g. `user-authentication` |
| `{{NNN}}` | Zero-padded 3-digit sequence, e.g. `001` |
| `{{YYYY-MM-DD}}` | ISO 8601 date |
| `{{NAME}}` | Author / reviewer name or handle |

---

## Full 7-Step Reference

See [docs/harness/PROCESS.md](../docs/harness/PROCESS.md) for the complete process guide.
