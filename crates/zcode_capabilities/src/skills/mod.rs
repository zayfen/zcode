//! Skills system — select relevant `docs/skills/*/SKILL.md` files and inject
//! them into the agent system prompt.
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
//! Relevant skills are sorted by priority and relevance before injection.
//! The directory is optional — if `docs/skills/` is absent, no skills are loaded.

use std::collections::BTreeSet;
use std::path::Path;

pub const DEFAULT_MAX_SELECTED_SKILLS: usize = 4;

// ─────────────────────────────────────────────
// Skill
// ─────────────────────────────────────────────

/// Priority of a skill (controls insertion order in system prompt).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
    /// Optional comma-separated trigger phrases from frontmatter.
    pub triggers: Vec<String>,
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
    /// Load all skills from `<project_root>/docs/skills/`.
    ///
    /// Each skill must live in its own subdirectory containing a `SKILL.md` file:
    /// ```text
    /// docs/skills/
    /// └── rust-conventions/
    ///     └── SKILL.md         ← loaded
    /// ```
    ///
    /// Returns an empty vec if the directory does not exist.
    pub fn load(project_root: &Path, extra_dirs: &[String]) -> Vec<Skill> {
        let mut all_skills = Vec::new();

        // 1. Load from project root
        let skills_dir = project_root.join("docs").join("skills");
        all_skills.extend(Self::load_from_dir(&skills_dir));

        // 2. Load from extra configuration directories
        for extra in extra_dirs {
            let path = std::path::PathBuf::from(extra);
            all_skills.extend(Self::load_from_dir(&path));
        }

        // Sort: High first, then Medium, then Low.
        all_skills.sort_by(|a, b| b.priority.cmp(&a.priority));
        all_skills
    }

    fn load_from_dir(skills_dir: &Path) -> Vec<Skill> {
        if !skills_dir.is_dir() {
            return vec![];
        }

        std::fs::read_dir(skills_dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().is_dir())
            .filter_map(|entry| {
                let skill_file = entry.path().join("SKILL.md");
                if skill_file.exists() {
                    Self::parse_skill_file(&skill_file)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Select the relevant skills for a prompt.
    ///
    /// Selection is generic: it compares the current prompt with each skill's
    /// name, description, triggers, and body. Unrelated skills are omitted
    /// instead of being injected into every request.
    pub fn select_relevant(skills: &[Skill], prompt: &str, max_skills: usize) -> Vec<Skill> {
        if skills.is_empty() || max_skills == 0 {
            return Vec::new();
        }

        let query = tokenize_for_selection(prompt);
        if query.is_empty() {
            return Vec::new();
        }

        let mut candidates: Vec<_> = skills
            .iter()
            .enumerate()
            .filter_map(|(index, skill)| {
                let name_tokens = tokenize_for_selection(&skill.name);
                let description_tokens = tokenize_for_selection(&skill.description);
                let trigger_tokens = tokenize_for_selection(&skill.triggers.join(" "));
                let content_tokens = tokenize_for_selection(&skill.content);

                let name_overlap = overlap_count(&query, &name_tokens);
                let description_overlap = overlap_count(&query, &description_tokens);
                let trigger_overlap = overlap_count(&query, &trigger_tokens);
                let content_overlap = overlap_count(&query, &content_tokens);

                let metadata_overlap = name_overlap + description_overlap + trigger_overlap;
                if metadata_overlap == 0 && content_overlap < 2 {
                    return None;
                }

                let score = (trigger_overlap * 5)
                    + (name_overlap * 4)
                    + (description_overlap * 3)
                    + content_overlap;
                Some((index, score, skill.priority, skill.clone()))
            })
            .collect();

        candidates.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then_with(|| b.2.cmp(&a.2))
                .then_with(|| a.0.cmp(&b.0))
        });
        candidates
            .into_iter()
            .take(max_skills)
            .map(|(_, _, _, skill)| skill)
            .collect()
    }

    /// Build an enhanced system prompt by appending selected skills.
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

    /// Select relevant skills for `prompt`, then build the system prompt.
    pub fn build_relevant_system_prompt(
        base_prompt: &str,
        skills: &[Skill],
        prompt: &str,
    ) -> String {
        let selected = Self::select_relevant(skills, prompt, DEFAULT_MAX_SELECTED_SKILLS);
        Self::build_system_prompt(base_prompt, &selected)
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
        let triggers = Self::fm_field(&frontmatter, "triggers")
            .map(|value| split_triggers(&value))
            .unwrap_or_default();
        let priority = Self::fm_field(&frontmatter, "priority")
            .map(|s| SkillPriority::from_str(&s))
            .unwrap_or(SkillPriority::Medium);

        Some(Skill {
            name,
            description,
            triggers,
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

fn split_triggers(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

fn tokenize_for_selection(text: &str) -> BTreeSet<String> {
    let lower = text.to_lowercase();
    let mut tokens = BTreeSet::new();
    let mut ascii = String::new();
    let mut cjk_run = Vec::new();

    for ch in lower.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            flush_cjk(&mut cjk_run, &mut tokens);
            ascii.push(ch);
            continue;
        }

        flush_ascii(&mut ascii, &mut tokens);
        if is_cjk(ch) {
            cjk_run.push(ch);
        } else {
            flush_cjk(&mut cjk_run, &mut tokens);
        }
    }

    flush_ascii(&mut ascii, &mut tokens);
    flush_cjk(&mut cjk_run, &mut tokens);
    tokens
}

fn flush_ascii(buffer: &mut String, tokens: &mut BTreeSet<String>) {
    if buffer.chars().count() >= 2 && !is_stop_token(buffer) {
        tokens.insert(buffer.clone());
    }
    buffer.clear();
}

fn flush_cjk(buffer: &mut Vec<char>, tokens: &mut BTreeSet<String>) {
    match buffer.len() {
        0 => {}
        1 => {
            let token = buffer[0].to_string();
            if !is_stop_token(&token) {
                tokens.insert(token);
            }
        }
        _ => {
            for window in buffer.windows(2) {
                let token: String = window.iter().collect();
                if !is_stop_token(&token) {
                    tokens.insert(token);
                }
            }
            for window in buffer.windows(3) {
                let token: String = window.iter().collect();
                if !is_stop_token(&token) {
                    tokens.insert(token);
                }
            }
        }
    }
    buffer.clear();
}

fn overlap_count(left: &BTreeSet<String>, right: &BTreeSet<String>) -> usize {
    left.iter().filter(|item| right.contains(*item)).count()
}

fn is_stop_token(token: &str) -> bool {
    matches!(
        token,
        "the"
            | "and"
            | "for"
            | "with"
            | "this"
            | "that"
            | "what"
            | "which"
            | "who"
            | "when"
            | "where"
            | "how"
            | "can"
            | "you"
            | "please"
            | "about"
            | "task"
            | "agent"
            | "current"
            | "这个"
            | "一个"
            | "我们"
            | "你们"
            | "请你"
            | "帮我"
            | "一下"
            | "继续"
            | "刚才"
    )
}

fn is_cjk(ch: char) -> bool {
    matches!(
        ch as u32,
        0x4E00..=0x9FFF
            | 0x3400..=0x4DBF
            | 0x20000..=0x2A6DF
            | 0x2A700..=0x2B73F
            | 0x2B740..=0x2B81F
            | 0x2B820..=0x2CEAF
    )
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_skills_dir(root: &Path) -> std::path::PathBuf {
        let dir = root.join("docs").join("skills");
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Create a skill subdirectory with a SKILL.md file.
    fn write_skill(skills_dir: &Path, skill_name: &str, content: &str) {
        let skill_dir = skills_dir.join(skill_name);
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), content).unwrap();
    }

    #[test]
    fn test_load_no_skills_dir_returns_empty() {
        let dir = TempDir::new().unwrap();
        let skills = SkillsLoader::load(dir.path(), &[]);
        assert!(skills.is_empty());
    }

    #[test]
    fn test_load_empty_skills_dir_returns_empty() {
        let dir = TempDir::new().unwrap();
        make_skills_dir(dir.path());
        let skills = SkillsLoader::load(dir.path(), &[]);
        assert!(skills.is_empty());
    }

    #[test]
    fn test_load_single_skill_with_frontmatter() {
        let dir = TempDir::new().unwrap();
        let skills_dir = make_skills_dir(dir.path());
        write_skill(
            &skills_dir,
            "rust-errors",
            "---\nname: rust-error-handling\ndescription: Error rules\npriority: high\n---\n\nAlways use ZcodeError.\n",
        );

        let skills = SkillsLoader::load(dir.path(), &[]);
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "rust-error-handling");
        assert_eq!(skills[0].description, "Error rules");
        assert_eq!(skills[0].priority, SkillPriority::High);
        assert!(skills[0].triggers.is_empty());
        assert!(skills[0].content.contains("ZcodeError"));
    }

    #[test]
    fn test_load_skill_with_triggers() {
        let dir = TempDir::new().unwrap();
        let skills_dir = make_skills_dir(dir.path());
        write_skill(
            &skills_dir,
            "vue",
            "---\nname: vue-best-practices\ndescription: Vue components\npriority: medium\ntriggers: vue, component, composable\n---\n\nUse Vue 3 patterns.\n",
        );

        let skills = SkillsLoader::load(dir.path(), &[]);

        assert_eq!(
            skills[0].triggers,
            vec![
                "vue".to_string(),
                "component".to_string(),
                "composable".to_string()
            ]
        );
    }

    #[test]
    fn test_load_skill_without_frontmatter_uses_dirname() {
        let dir = TempDir::new().unwrap();
        let skills_dir = make_skills_dir(dir.path());
        // No frontmatter — name falls back to the SKILL.md stem ("SKILL")
        write_skill(&skills_dir, "my-skill", "# My Skill\n\nSome content.\n");

        let skills = SkillsLoader::load(dir.path(), &[]);
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].priority, SkillPriority::Medium);
    }

    #[test]
    fn test_skill_dir_without_skill_md_is_ignored() {
        let dir = TempDir::new().unwrap();
        let skills_dir = make_skills_dir(dir.path());
        // Subdirectory without SKILL.md should be silently ignored.
        let orphan = skills_dir.join("no-skill-md");
        fs::create_dir_all(&orphan).unwrap();
        fs::write(orphan.join("README.md"), "Not a skill.").unwrap();

        let skills = SkillsLoader::load(dir.path(), &[]);
        assert!(skills.is_empty());
    }

    #[test]
    fn test_skills_sorted_by_priority() {
        let dir = TempDir::new().unwrap();
        let skills_dir = make_skills_dir(dir.path());

        write_skill(
            &skills_dir,
            "low",
            "---\nname: low\npriority: low\n---\nLow skill",
        );
        write_skill(
            &skills_dir,
            "high",
            "---\nname: high\npriority: high\n---\nHigh skill",
        );
        write_skill(
            &skills_dir,
            "mid",
            "---\nname: mid\npriority: medium\n---\nMid skill",
        );

        let skills = SkillsLoader::load(dir.path(), &[]);
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
            triggers: Vec::new(),
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
    fn test_flat_md_files_in_skills_dir_are_ignored() {
        // Flat .md files placed directly in docs/skills/ (not in a subdirectory)
        // should be ignored — only SKILL.md inside subdirs counts.
        let dir = TempDir::new().unwrap();
        let skills_dir = make_skills_dir(dir.path());
        fs::write(skills_dir.join("stray.md"), "# Stray file").unwrap();

        let skills = SkillsLoader::load(dir.path(), &[]);
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

    #[test]
    fn test_select_relevant_matches_prompt_metadata() {
        let skills = vec![
            Skill {
                name: "rust-conventions".into(),
                description: "Rust coding conventions".into(),
                triggers: vec!["rust".into(), "cargo".into()],
                priority: SkillPriority::High,
                content: "Use Result and ZcodeError.".into(),
            },
            Skill {
                name: "vue-ui".into(),
                description: "Vue component rules".into(),
                triggers: vec!["vue".into(), "component".into()],
                priority: SkillPriority::High,
                content: "Use composables.".into(),
            },
        ];

        let selected = SkillsLoader::select_relevant(&skills, "fix rust cargo tests", 4);

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].name, "rust-conventions");
    }

    #[test]
    fn test_select_relevant_omits_unrelated_high_priority_skill() {
        let skills = vec![Skill {
            name: "rust-conventions".into(),
            description: "Rust coding conventions".into(),
            triggers: vec!["rust".into()],
            priority: SkillPriority::High,
            content: "Use Result and ZcodeError.".into(),
        }];

        let selected = SkillsLoader::select_relevant(&skills, "深圳今天的天气", 4);

        assert!(selected.is_empty());
    }

    #[test]
    fn test_select_relevant_uses_content_overlap_when_metadata_missing() {
        let skills = vec![Skill {
            name: "backend".into(),
            description: "No description".into(),
            triggers: Vec::new(),
            priority: SkillPriority::Medium,
            content: "Always use serde json schema validation for config parsing.".into(),
        }];

        let selected = SkillsLoader::select_relevant(&skills, "improve serde config validation", 4);

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].name, "backend");
    }

    #[test]
    fn test_build_relevant_system_prompt_only_includes_selected_skills() {
        let skills = vec![
            Skill {
                name: "rust".into(),
                description: "Rust rules".into(),
                triggers: vec!["rust".into()],
                priority: SkillPriority::Medium,
                content: "Rust content.".into(),
            },
            Skill {
                name: "vue".into(),
                description: "Vue rules".into(),
                triggers: vec!["vue".into()],
                priority: SkillPriority::Medium,
                content: "Vue content.".into(),
            },
        ];

        let prompt = SkillsLoader::build_relevant_system_prompt("", &skills, "fix rust module");

        assert!(prompt.contains("Rust content."));
        assert!(!prompt.contains("Vue content."));
    }
}
