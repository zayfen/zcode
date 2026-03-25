//! Skills system — inject `docs/skills/*.md` into the agent system prompt.
//!
//! Skills are markdown documents that provide specialised instructions to the
//! AI agent. They are read from `docs/skills/` in the project root and
//! prepended to the system prompt before each `zcode run`.
//!
//! # Skill file format
//! ```markdown
//! ---
//! name: rust-error-handling
//! description: Rules for error handling in this project
//! priority: high
//! ---
//!
//! Always use `ZcodeError`. Never use `unwrap()` outside tests.
//! ```
//!
//! # Priority levels
//! `high` > `medium` (default) > `low`
//!
//! Skills are sorted by priority and concatenated in that order.
//! The directory is optional — if `docs/skills/` is absent, no skills are loaded.

use std::path::Path;

// ─────────────────────────────────────────────
// Skill
// ─────────────────────────────────────────────

/// Priority of a skill (controls insertion order in system prompt).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SkillPriority {
    Low,
    Medium,
    High,
}

impl SkillPriority {
    fn from_str(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "high" => SkillPriority::High,
            "low" => SkillPriority::Low,
            _ => SkillPriority::Medium,
        }
    }
}

/// A single loaded skill.
#[derive(Debug, Clone)]
pub struct Skill {
    /// Internal name (from frontmatter, or derived from filename).
    pub name: String,
    /// One-line description shown in debug output.
    pub description: String,
    /// Insertion priority.
    pub priority: SkillPriority,
    /// The body of the skill document (everything after the frontmatter).
    pub content: String,
}

// ─────────────────────────────────────────────
// SkillsLoader
// ─────────────────────────────────────────────

/// Loads skill files from `docs/skills/` and builds an enhanced system prompt.
pub struct SkillsLoader;

impl SkillsLoader {
    /// Load all `*.md` skill files from `<project_root>/docs/skills/`.
    ///
    /// Returns an empty vec if the directory does not exist.
    pub fn load(project_root: &Path) -> Vec<Skill> {
        let skills_dir = project_root.join("docs").join("skills");
        if !skills_dir.is_dir() {
            return vec![];
        }

        let mut skills: Vec<Skill> = std::fs::read_dir(&skills_dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "md")
                    .unwrap_or(false)
            })
            .filter_map(|entry| Self::parse_skill_file(&entry.path()))
            .collect();

        // Sort: High first, then Medium, then Low.
        skills.sort_by(|a, b| b.priority.cmp(&a.priority));
        skills
    }

    /// Build an enhanced system prompt by appending all loaded skills.
    pub fn build_system_prompt(base_prompt: &str, skills: &[Skill]) -> String {
        if skills.is_empty() {
            return base_prompt.to_string();
        }

        let mut parts = vec![base_prompt.to_string()];
        parts.push(
            "\n\n---\n## Project Skills & Conventions\n\
             The following rules MUST be followed for this project:\n"
                .to_string(),
        );

        for skill in skills {
            parts.push(format!(
                "\n### {} — {}\n{}",
                skill.name, skill.description, skill.content
            ));
        }

        parts.join("")
    }

    // ── Private helpers ──────────────────────────────────────────

    /// Parse a single skill markdown file.
    ///
    /// Frontmatter is delimited by `---` lines. Everything after the second
    /// `---` is treated as the skill body.
    fn parse_skill_file(path: &Path) -> Option<Skill> {
        let raw = std::fs::read_to_string(path).ok()?;
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unnamed")
            .to_string();

        let (frontmatter, body) = Self::split_frontmatter(&raw);

        let name = Self::fm_field(&frontmatter, "name").unwrap_or(stem);
        let description = Self::fm_field(&frontmatter, "description")
            .unwrap_or_else(|| "No description".to_string());
        let priority = Self::fm_field(&frontmatter, "priority")
            .map(|s| SkillPriority::from_str(&s))
            .unwrap_or(SkillPriority::Medium);

        Some(Skill {
            name,
            description,
            priority,
            content: body.trim().to_string(),
        })
    }

    /// Split a markdown document into (frontmatter_str, body_str).
    /// Returns ("", full_text) if no frontmatter found.
    fn split_frontmatter(text: &str) -> (String, String) {
        let lines: Vec<&str> = text.lines().collect();

        if lines.first().map(|l| l.trim()) != Some("---") {
            return (String::new(), text.to_string());
        }

        // Find the closing ---
        let close = lines[1..]
            .iter()
            .position(|l| l.trim() == "---")
            .map(|i| i + 1); // offset by 1 because we sliced from index 1

        match close {
            Some(end_idx) => {
                let fm = lines[1..end_idx].join("\n");
                let body = lines[end_idx + 1..].join("\n");
                (fm, body)
            }
            None => (String::new(), text.to_string()),
        }
    }

    /// Extract a YAML-style `key: value` field from the frontmatter string.
    fn fm_field(frontmatter: &str, key: &str) -> Option<String> {
        for line in frontmatter.lines() {
            if let Some(rest) = line.strip_prefix(&format!("{}:", key)) {
                return Some(rest.trim().to_string());
            }
        }
        None
    }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_skill_dir(root: &Path) -> std::path::PathBuf {
        let dir = root.join("docs").join("skills");
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_skill(dir: &Path, filename: &str, content: &str) {
        fs::write(dir.join(filename), content).unwrap();
    }

    #[test]
    fn test_load_no_skills_dir_returns_empty() {
        let dir = TempDir::new().unwrap();
        let skills = SkillsLoader::load(dir.path());
        assert!(skills.is_empty());
    }

    #[test]
    fn test_load_empty_skills_dir_returns_empty() {
        let dir = TempDir::new().unwrap();
        make_skill_dir(dir.path());
        let skills = SkillsLoader::load(dir.path());
        assert!(skills.is_empty());
    }

    #[test]
    fn test_load_single_skill_with_frontmatter() {
        let dir = TempDir::new().unwrap();
        let skills_dir = make_skill_dir(dir.path());
        write_skill(
            &skills_dir,
            "rust-errors.md",
            "---\nname: rust-error-handling\ndescription: Error rules\npriority: high\n---\n\nAlways use ZcodeError.\n",
        );

        let skills = SkillsLoader::load(dir.path());
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "rust-error-handling");
        assert_eq!(skills[0].description, "Error rules");
        assert_eq!(skills[0].priority, SkillPriority::High);
        assert!(skills[0].content.contains("ZcodeError"));
    }

    #[test]
    fn test_load_skill_without_frontmatter_uses_filename() {
        let dir = TempDir::new().unwrap();
        let skills_dir = make_skill_dir(dir.path());
        write_skill(&skills_dir, "my-skill.md", "# My Skill\n\nSome content.\n");

        let skills = SkillsLoader::load(dir.path());
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "my-skill");
        assert_eq!(skills[0].priority, SkillPriority::Medium);
    }

    #[test]
    fn test_skills_sorted_by_priority() {
        let dir = TempDir::new().unwrap();
        let skills_dir = make_skill_dir(dir.path());

        write_skill(&skills_dir, "low.md", "---\nname: low\npriority: low\n---\nLow skill");
        write_skill(&skills_dir, "high.md", "---\nname: high\npriority: high\n---\nHigh skill");
        write_skill(&skills_dir, "mid.md", "---\nname: mid\npriority: medium\n---\nMid skill");

        let skills = SkillsLoader::load(dir.path());
        assert_eq!(skills.len(), 3);
        assert_eq!(skills[0].priority, SkillPriority::High);
        assert_eq!(skills[1].priority, SkillPriority::Medium);
        assert_eq!(skills[2].priority, SkillPriority::Low);
    }

    #[test]
    fn test_build_system_prompt_no_skills() {
        let base = "You are an agent.";
        let result = SkillsLoader::build_system_prompt(base, &[]);
        assert_eq!(result, base);
    }

    #[test]
    fn test_build_system_prompt_appends_skills() {
        let base = "You are an agent.";
        let skill = Skill {
            name: "conventions".into(),
            description: "Project conventions".into(),
            priority: SkillPriority::High,
            content: "Always write tests.".into(),
        };
        let result = SkillsLoader::build_system_prompt(base, &[skill]);
        assert!(result.contains("You are an agent."));
        assert!(result.contains("Project Skills & Conventions"));
        assert!(result.contains("Always write tests."));
        assert!(result.contains("conventions"));
    }

    #[test]
    fn test_non_md_files_are_ignored() {
        let dir = TempDir::new().unwrap();
        let skills_dir = make_skill_dir(dir.path());
        write_skill(&skills_dir, "notes.txt", "This is not a skill.");
        write_skill(&skills_dir, "data.json", r#"{"key": "value"}"#);

        let skills = SkillsLoader::load(dir.path());
        assert!(skills.is_empty());
    }

    #[test]
    fn test_skill_priority_ordering() {
        assert!(SkillPriority::High > SkillPriority::Medium);
        assert!(SkillPriority::Medium > SkillPriority::Low);
    }

    #[test]
    fn test_split_frontmatter_no_delimiter() {
        let text = "# Hello\n\nSome content.";
        let (fm, body) = SkillsLoader::split_frontmatter(text);
        assert!(fm.is_empty());
        assert!(body.contains("Hello"));
    }
}
