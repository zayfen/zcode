//! Harness Engineering docs validator
//!
//! Before running any zcode task, the current working directory must contain
//! a valid `docs/` structure that follows the Harness Engineering convention:
//!
//! ```text
//! docs/
//! ├── prd/                        (required, ≥1 .md)
//! │   └── *.md
//! ├── specs/
//! │   └── coding.spec.md          (required)
//! ├── tasks/                      (required, ≥1 .tasks.md)
//! │   └── *.tasks.md
//! ├── validation.md               (required)
//! └── review-checklist.md         (required)
//! ```
//!
//! Each file must also contain certain required sections.

use std::fmt;
use std::path::{Path, PathBuf};

// ─────────────────────────────────────────────
// Error & Result types
// ─────────────────────────────────────────────

/// A single validation failure.
#[derive(Debug, Clone)]
pub struct DocsError {
    /// Human-readable description of the problem.
    pub message: String,
    /// Suggested fix (shown in the CLI output).
    pub hint: String,
}

impl DocsError {
    fn new(message: impl Into<String>, hint: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            hint: hint.into(),
        }
    }
}

impl fmt::Display for DocsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}\n  Hint: {}", self.message, self.hint)
    }
}

/// Aggregated result from a docs validation pass.
#[derive(Debug, Default)]
pub struct DocsValidationResult {
    pub errors: Vec<DocsError>,
}

impl DocsValidationResult {
    /// Returns `true` when every check passed.
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    /// Append another error to this result.
    pub fn add_error(&mut self, err: DocsError) {
        self.errors.push(err);
    }
}

impl fmt::Display for DocsValidationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_valid() {
            return write!(f, "docs/ validation passed ✓");
        }
        writeln!(f, "docs/ validation FAILED ({} error(s)):", self.errors.len())?;
        for (i, e) in self.errors.iter().enumerate() {
            writeln!(f, "  {}. {}", i + 1, e)?;
        }
        Ok(())
    }
}

// ─────────────────────────────────────────────
// Validator
// ─────────────────────────────────────────────

/// Validates a project directory against the Harness Engineering docs convention.
pub struct DocsValidator {
    /// Root of the project being inspected (usually `cwd`).
    project_root: PathBuf,
}

impl DocsValidator {
    /// Create a validator for the given project directory.
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
        }
    }

    /// Run all validation checks and return the result.
    pub fn validate(&self) -> DocsValidationResult {
        let mut result = DocsValidationResult::default();
        let docs = self.project_root.join("docs");

        // ── 1. docs/ directory must exist ──────────────────────────
        if !docs.is_dir() {
            result.add_error(DocsError::new(
                "Missing docs/ directory",
                "Run `zcode docs init` to generate the required scaffolding.",
            ));
            // Nothing else to check without the docs dir.
            return result;
        }

        // ── 2. docs/prd/ must exist with ≥1 .md file ───────────────
        self.check_dir_with_files(&docs, "prd", "*.md", &mut result);

        // ── 3. docs/specs/coding.spec.md must exist & have sections ─
        let coding_spec = docs.join("specs").join("coding.spec.md");
        if !coding_spec.is_file() {
            result.add_error(DocsError::new(
                "Missing docs/specs/coding.spec.md",
                "Create this file with ## Tech Stack and ## File Structure sections.",
            ));
        } else {
            let content = std::fs::read_to_string(&coding_spec).unwrap_or_default();
            self.require_section(
                &content,
                &["## Tech Stack", "## 技术栈"],
                "docs/specs/coding.spec.md",
                "Add a `## Tech Stack` section listing your language, framework and key crates.",
                &mut result,
            );
            self.require_section(
                &content,
                &["## File Structure", "## 文件结构"],
                "docs/specs/coding.spec.md",
                "Add a `## File Structure` section showing the project layout.",
                &mut result,
            );
        }

        // ── 4. docs/tasks/ must contain ≥1 .tasks.md ───────────────
        self.check_dir_with_files_ext(&docs, "tasks", ".tasks.md", &mut result);

        // ── 5. docs/validation.md must exist & have Quality Gates ───
        let validation_md = docs.join("validation.md");
        if !validation_md.is_file() {
            result.add_error(DocsError::new(
                "Missing docs/validation.md",
                "Create this file with a `## Quality Gates` section.",
            ));
        } else {
            let content = std::fs::read_to_string(&validation_md).unwrap_or_default();
            self.require_section(
                &content,
                &["## Quality Gates", "## 质量标准"],
                "docs/validation.md",
                "Add a `## Quality Gates` section with your pass/fail criteria.",
                &mut result,
            );
        }

        // ── 6. docs/review-checklist.md must exist with ≥3 items ───
        let checklist = docs.join("review-checklist.md");
        if !checklist.is_file() {
            result.add_error(DocsError::new(
                "Missing docs/review-checklist.md",
                "Create this file with at least 3 `- [ ]` checklist items.",
            ));
        } else {
            let content = std::fs::read_to_string(&checklist).unwrap_or_default();
            let item_count = content.lines().filter(|l| l.contains("- [ ]")).count();
            if item_count < 3 {
                result.add_error(DocsError::new(
                    format!(
                        "docs/review-checklist.md has only {} checklist item(s), need ≥3",
                        item_count
                    ),
                    "Add at least 3 `- [ ]` review items.",
                ));
            }
        }

        result
    }

    // ── Helpers ───────────────────────────────────────────────────

    /// Check that `docs/<subdir>/` exists and contains at least one `.md` file.
    fn check_dir_with_files(
        &self,
        docs: &Path,
        subdir: &str,
        _pattern: &str,
        result: &mut DocsValidationResult,
    ) {
        let dir = docs.join(subdir);
        if !dir.is_dir() {
            result.add_error(DocsError::new(
                format!("Missing docs/{}/", subdir),
                format!("Create docs/{}/ with at least one .md file.", subdir),
            ));
            return;
        }
        let has_md = std::fs::read_dir(&dir)
            .map(|entries| {
                entries.flatten().any(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext == "md")
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);
        if !has_md {
            result.add_error(DocsError::new(
                format!("docs/{}/ contains no .md files", subdir),
                format!("Add at least one Markdown document to docs/{}/.", subdir),
            ));
        }
    }

    /// Check that `docs/<subdir>/` exists and contains at least one file ending with `suffix`.
    fn check_dir_with_files_ext(
        &self,
        docs: &Path,
        subdir: &str,
        suffix: &str,
        result: &mut DocsValidationResult,
    ) {
        let dir = docs.join(subdir);
        if !dir.is_dir() {
            result.add_error(DocsError::new(
                format!("Missing docs/{}/", subdir),
                format!(
                    "Create docs/{}/ with at least one `*{}` file.",
                    subdir, suffix
                ),
            ));
            return;
        }
        let has_file = std::fs::read_dir(&dir)
            .map(|entries| {
                entries.flatten().any(|e| {
                    e.path()
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.ends_with(suffix))
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);
        if !has_file {
            result.add_error(DocsError::new(
                format!("docs/{}/ contains no `*{}` files", subdir, suffix),
                format!(
                    "Add at least one `*{}` file to docs/{}/.",
                    suffix, subdir
                ),
            ));
        }
    }

    /// Require at least one of the given heading variants to be present in `content`.
    fn require_section(
        &self,
        content: &str,
        headings: &[&str],
        file: &str,
        hint: &str,
        result: &mut DocsValidationResult,
    ) {
        let found = headings.iter().any(|h| content.contains(h));
        if !found {
            result.add_error(DocsError::new(
                format!(
                    "{} is missing required section `{}`",
                    file, headings[0]
                ),
                hint,
            ));
        }
    }
}

// ─────────────────────────────────────────────
// Scaffolding generator
// ─────────────────────────────────────────────

/// Generate the `docs/` skeleton in `project_root`.
///
/// Only creates files that don't already exist — safe to re-run.
pub fn generate_docs_scaffold(project_root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let docs = project_root.join("docs");
    let mut created = Vec::new();

    macro_rules! write_if_missing {
        ($path:expr, $content:expr) => {{
            let p: PathBuf = $path;
            if !p.exists() {
                if let Some(parent) = p.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&p, $content)?;
                created.push(p);
            }
        }};
    }

    // docs/prd/001-feature.md
    write_if_missing!(
        docs.join("prd").join("001-feature.md"),
        "# Feature: <Feature Name>\n\
         \n\
         ## Goals\n\
         - TODO: describe what this feature does\n\
         \n\
         ## Non-Goals\n\
         - TODO: list explicit out-of-scope items\n\
         \n\
         ## User Stories\n\
         - As a user, I want to …\n\
         \n\
         ## Acceptance Criteria\n\
         - [ ] TODO: add testable criteria\n"
    );

    // docs/specs/coding.spec.md
    write_if_missing!(
        docs.join("specs").join("coding.spec.md"),
        "# Coding Spec\n\
         \n\
         ## Tech Stack\n\
         - Language: Rust 2021\n\
         - Async runtime: tokio\n\
         - TODO: fill in your stack\n\
         \n\
         ## File Structure\n\
         ```\n\
         src/\n\
         └── TODO: describe layout\n\
         ```\n\
         \n\
         ## Conventions\n\
         - TODO: naming, error handling, testing conventions\n"
    );

    // docs/tasks/001-feature.tasks.md
    write_if_missing!(
        docs.join("tasks").join("001-feature.tasks.md"),
        "# Tasks: <Feature Name>\n\
         \n\
         ## Implementation Tasks\n\
         - [ ] TODO: first atomic task\n\
         - [ ] TODO: second atomic task\n\
         - [ ] TODO: third atomic task\n\
         \n\
         ## Test Tasks\n\
         - [ ] Write unit tests\n\
         - [ ] Write integration tests\n"
    );

    // docs/validation.md
    write_if_missing!(
        docs.join("validation.md"),
        "# Validation\n\
         \n\
         ## Quality Gates\n\
         - [ ] `cargo test` — 0 failures\n\
         - [ ] `cargo clippy -- -D warnings` — 0 warnings\n\
         - [ ] `cargo fmt --check` — clean formatting\n\
         \n\
         ## Acceptance Validation\n\
         - [ ] All PRD Acceptance Criteria met\n\
         - [ ] Manual smoke test completed\n"
    );

    // docs/review-checklist.md
    write_if_missing!(
        docs.join("review-checklist.md"),
        "# Review Checklist\n\
         \n\
         ## Code Quality\n\
         - [ ] No unnecessary unwrap() / expect()\n\
         - [ ] Error types are meaningful\n\
         - [ ] Public API has doc comments\n\
         \n\
         ## Architecture\n\
         - [ ] Follows the patterns in docs/specs/coding.spec.md\n\
         - [ ] No circular dependencies introduced\n\
         \n\
         ## Testing\n\
         - [ ] New code has unit tests\n\
         - [ ] Edge cases are covered\n"
    );

    Ok(created)
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Helper: create a validating docs/ structure.
    fn make_valid_docs(root: &Path) {
        let docs = root.join("docs");
        // prd/
        let prd = docs.join("prd");
        fs::create_dir_all(&prd).unwrap();
        fs::write(
            prd.join("001-feature.md"),
            "# Feature\n## Goals\n- one\n## Acceptance Criteria\n- [ ] pass\n",
        )
        .unwrap();
        // specs/
        let specs = docs.join("specs");
        fs::create_dir_all(&specs).unwrap();
        fs::write(
            specs.join("coding.spec.md"),
            "# Coding Spec\n## Tech Stack\n- Rust\n## File Structure\n```\nsrc/\n```\n",
        )
        .unwrap();
        // tasks/
        let tasks = docs.join("tasks");
        fs::create_dir_all(&tasks).unwrap();
        fs::write(
            tasks.join("001-feature.tasks.md"),
            "# Tasks\n- [ ] a\n- [ ] b\n- [ ] c\n",
        )
        .unwrap();
        // validation.md
        fs::write(
            docs.join("validation.md"),
            "# Validation\n## Quality Gates\n- [ ] pass\n",
        )
        .unwrap();
        // review-checklist.md
        fs::write(
            docs.join("review-checklist.md"),
            "# Review\n- [ ] one\n- [ ] two\n- [ ] three\n",
        )
        .unwrap();
    }

    #[test]
    fn test_valid_docs_passes() {
        let dir = TempDir::new().unwrap();
        make_valid_docs(dir.path());
        let v = DocsValidator::new(dir.path());
        let result = v.validate();
        assert!(result.is_valid(), "Expected valid: {}", result);
    }

    #[test]
    fn test_missing_docs_dir() {
        let dir = TempDir::new().unwrap();
        let v = DocsValidator::new(dir.path());
        let result = v.validate();
        assert!(!result.is_valid());
        assert!(result.errors[0].message.contains("Missing docs/"));
    }

    #[test]
    fn test_missing_prd_dir() {
        let dir = TempDir::new().unwrap();
        make_valid_docs(dir.path());
        fs::remove_dir_all(dir.path().join("docs/prd")).unwrap();
        let result = DocsValidator::new(dir.path()).validate();
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.message.contains("prd")));
    }

    #[test]
    fn test_missing_coding_spec() {
        let dir = TempDir::new().unwrap();
        make_valid_docs(dir.path());
        fs::remove_file(dir.path().join("docs/specs/coding.spec.md")).unwrap();
        let result = DocsValidator::new(dir.path()).validate();
        assert!(!result.is_valid());
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("coding.spec.md")));
    }

    #[test]
    fn test_coding_spec_missing_tech_stack_section() {
        let dir = TempDir::new().unwrap();
        make_valid_docs(dir.path());
        fs::write(
            dir.path().join("docs/specs/coding.spec.md"),
            "# Coding Spec\n## File Structure\n```\nsrc/\n```\n",
        )
        .unwrap();
        let result = DocsValidator::new(dir.path()).validate();
        assert!(!result.is_valid());
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("Tech Stack")));
    }

    #[test]
    fn test_coding_spec_missing_file_structure_section() {
        let dir = TempDir::new().unwrap();
        make_valid_docs(dir.path());
        fs::write(
            dir.path().join("docs/specs/coding.spec.md"),
            "# Coding Spec\n## Tech Stack\n- Rust\n",
        )
        .unwrap();
        let result = DocsValidator::new(dir.path()).validate();
        assert!(!result.is_valid());
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("File Structure")));
    }

    #[test]
    fn test_missing_tasks_dir() {
        let dir = TempDir::new().unwrap();
        make_valid_docs(dir.path());
        fs::remove_dir_all(dir.path().join("docs/tasks")).unwrap();
        let result = DocsValidator::new(dir.path()).validate();
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.message.contains("tasks")));
    }

    #[test]
    fn test_tasks_dir_without_tasks_files() {
        let dir = TempDir::new().unwrap();
        make_valid_docs(dir.path());
        // Replace .tasks.md with a plain .md file
        fs::remove_file(dir.path().join("docs/tasks/001-feature.tasks.md")).unwrap();
        fs::write(dir.path().join("docs/tasks/notes.md"), "notes").unwrap();
        let result = DocsValidator::new(dir.path()).validate();
        assert!(!result.is_valid());
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains(".tasks.md")));
    }

    #[test]
    fn test_missing_validation_md() {
        let dir = TempDir::new().unwrap();
        make_valid_docs(dir.path());
        fs::remove_file(dir.path().join("docs/validation.md")).unwrap();
        let result = DocsValidator::new(dir.path()).validate();
        assert!(!result.is_valid());
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("validation.md")));
    }

    #[test]
    fn test_validation_md_missing_quality_gates() {
        let dir = TempDir::new().unwrap();
        make_valid_docs(dir.path());
        fs::write(dir.path().join("docs/validation.md"), "# Validation\n").unwrap();
        let result = DocsValidator::new(dir.path()).validate();
        assert!(!result.is_valid());
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("Quality Gates")));
    }

    #[test]
    fn test_missing_review_checklist() {
        let dir = TempDir::new().unwrap();
        make_valid_docs(dir.path());
        fs::remove_file(dir.path().join("docs/review-checklist.md")).unwrap();
        let result = DocsValidator::new(dir.path()).validate();
        assert!(!result.is_valid());
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("review-checklist.md")));
    }

    #[test]
    fn test_review_checklist_too_few_items() {
        let dir = TempDir::new().unwrap();
        make_valid_docs(dir.path());
        fs::write(
            dir.path().join("docs/review-checklist.md"),
            "# Review\n- [ ] one\n- [ ] two\n",
        )
        .unwrap();
        let result = DocsValidator::new(dir.path()).validate();
        assert!(!result.is_valid());
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("checklist item")));
    }

    #[test]
    fn test_scaffold_creates_expected_files() {
        let dir = TempDir::new().unwrap();
        let created = generate_docs_scaffold(dir.path()).unwrap();
        assert_eq!(created.len(), 5, "Expected 5 scaffold files");
        // After scaffold, validation should pass
        let result = DocsValidator::new(dir.path()).validate();
        assert!(result.is_valid(), "Scaffold should produce valid docs: {}", result);
    }

    #[test]
    fn test_scaffold_is_idempotent() {
        let dir = TempDir::new().unwrap();
        generate_docs_scaffold(dir.path()).unwrap();
        let created_second = generate_docs_scaffold(dir.path()).unwrap();
        assert_eq!(
            created_second.len(),
            0,
            "Re-running scaffold should create no additional files"
        );
    }

    #[test]
    fn test_docs_error_display() {
        let e = DocsError::new("something wrong", "fix it like this");
        let s = format!("{}", e);
        assert!(s.contains("something wrong"));
        assert!(s.contains("fix it like this"));
    }

    #[test]
    fn test_validation_result_display_valid() {
        let r = DocsValidationResult::default();
        assert!(format!("{}", r).contains("passed"));
    }

    #[test]
    fn test_validation_result_display_with_errors() {
        let mut r = DocsValidationResult::default();
        r.add_error(DocsError::new("oops", "fix it"));
        let s = format!("{}", r);
        assert!(s.contains("FAILED"));
        assert!(s.contains("oops"));
    }
}
