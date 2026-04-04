//! Changelog generation — categorizes tasks into Features/BugFixes/BreakingChanges/Improvements

use crate::git::DiffContext;
use std::collections::HashMap;

/// A simplified task record for changelog generation.
/// The delivery module works with this lightweight representation rather than
/// depending on a full TaskStore, keeping the module self-contained.
#[derive(Debug, Clone)]
pub struct TaskRecord {
    /// Task description
    pub task: String,
    /// Final verification score (0-100), if available
    pub final_score: Option<f64>,
    /// Task status string (e.g. "completed", "failed")
    pub status: String,
}

/// A single change entry in the changelog
#[derive(Debug, Clone)]
pub struct ChangeEntry {
    /// Human-readable description
    pub description: String,
    /// Verification score
    pub score: Option<f64>,
    /// Files changed in this entry
    pub files_changed: Vec<String>,
}

/// Category for grouping changes in the changelog
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum ChangeCategory {
    Features,
    BugFixes,
    BreakingChanges,
    Improvements,
    Other,
}

impl ChangeCategory {
    /// Display title for this category
    pub fn title(&self) -> &'static str {
        match self {
            Self::Features => "Features",
            Self::BugFixes => "Bug Fixes",
            Self::BreakingChanges => "Breaking Changes",
            Self::Improvements => "Improvements",
            Self::Other => "Other Changes",
        }
    }

    /// Ordered categories for changelog output
    pub fn ordered() -> &'static [ChangeCategory] {
        &[
            ChangeCategory::BreakingChanges,
            ChangeCategory::Features,
            ChangeCategory::BugFixes,
            ChangeCategory::Improvements,
            ChangeCategory::Other,
        ]
    }
}

/// Changelog generator — produces markdown changelog from task records and diff context
pub struct ChangelogGenerator;

impl ChangelogGenerator {
    /// Generate a markdown changelog from task records, commit messages, and diff context
    pub fn generate(
        tasks: &[TaskRecord],
        commits: &[String],
        diff: &DiffContext,
    ) -> String {
        let mut sections: HashMap<ChangeCategory, Vec<ChangeEntry>> = HashMap::new();

        let changed_file_names: Vec<String> = diff
            .changed_files
            .iter()
            .chain(diff.staged_files.iter())
            .map(|f| f.path.clone())
            .collect();

        for task in tasks {
            let category = Self::categorize_task(task);
            let entry = ChangeEntry {
                description: task.task.clone(),
                score: task.final_score,
                files_changed: changed_file_names.clone(),
            };
            sections.entry(category).or_default().push(entry);
        }

        // Add commits without matching tasks as "Other"
        if !commits.is_empty() && tasks.is_empty() {
            let entry = ChangeEntry {
                description: commits.join("\n"),
                score: None,
                files_changed: changed_file_names,
            };
            sections.entry(ChangeCategory::Other).or_default().push(entry);
        }

        Self::format_changelog(&sections)
    }

    /// Categorize a task based on keywords in its description
    pub fn categorize_task(task: &TaskRecord) -> ChangeCategory {
        let desc = task.task.to_lowercase();
        if desc.contains("fix") || desc.contains("bug") || desc.contains("patch") {
            ChangeCategory::BugFixes
        } else if desc.contains("breaking")
            || desc.contains("remove")
            || desc.contains("deprecat")
        {
            ChangeCategory::BreakingChanges
        } else if desc.contains("add")
            || desc.contains("new")
            || desc.contains("implement")
            || desc.contains("feature")
        {
            ChangeCategory::Features
        } else if desc.contains("refactor")
            || desc.contains("improv")
            || desc.contains("optim")
        {
            ChangeCategory::Improvements
        } else if desc.contains("test") || desc.contains("doc") {
            ChangeCategory::Other
        } else {
            ChangeCategory::Features
        }
    }

    /// Format categorized entries into a markdown changelog string
    pub fn format_changelog(sections: &HashMap<ChangeCategory, Vec<ChangeEntry>>) -> String {
        let mut md = String::from("# Changelog\n\n");

        for category in ChangeCategory::ordered() {
            if let Some(entries) = sections.get(category) {
                md.push_str(&format!("## {}\n\n", category.title()));
                for entry in entries {
                    let score_str = match entry.score {
                        Some(s) => format!("{:.0}/100", s),
                        None => "-".to_string(),
                    };
                    md.push_str(&format!("- {} (score: {})\n", entry.description, score_str));
                }
                md.push('\n');
            }
        }

        md
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::{ChangedFile, FileStatus};
    use std::path::PathBuf;

    fn make_task(desc: &str, score: Option<f64>) -> TaskRecord {
        TaskRecord {
            task: desc.to_string(),
            final_score: score,
            status: "completed".to_string(),
        }
    }

    fn make_diff_context(files: Vec<&str>) -> DiffContext {
        DiffContext {
            changed_files: files
                .iter()
                .map(|f| ChangedFile {
                    path: f.to_string(),
                    status: FileStatus::Modified,
                })
                .collect(),
            staged_files: vec![],
            patch: String::new(),
            repo_root: PathBuf::from("/tmp/repo"),
        }
    }

    #[test]
    fn test_categorize_fix() {
        let task = make_task("Fix login bug", Some(90.0));
        assert_eq!(ChangelogGenerator::categorize_task(&task), ChangeCategory::BugFixes);
    }

    #[test]
    fn test_categorize_bug() {
        let task = make_task("Bug in parser", None);
        assert_eq!(ChangelogGenerator::categorize_task(&task), ChangeCategory::BugFixes);
    }

    #[test]
    fn test_categorize_patch() {
        let task = make_task("Security patch for auth", None);
        assert_eq!(ChangelogGenerator::categorize_task(&task), ChangeCategory::BugFixes);
    }

    #[test]
    fn test_categorize_breaking() {
        let task = make_task("Breaking change to API", None);
        assert_eq!(
            ChangelogGenerator::categorize_task(&task),
            ChangeCategory::BreakingChanges
        );
    }

    #[test]
    fn test_categorize_remove() {
        let task = make_task("Remove old endpoint", None);
        assert_eq!(
            ChangelogGenerator::categorize_task(&task),
            ChangeCategory::BreakingChanges
        );
    }

    #[test]
    fn test_categorize_deprecate() {
        let task = make_task("Deprecate v1 API", None);
        assert_eq!(
            ChangelogGenerator::categorize_task(&task),
            ChangeCategory::BreakingChanges
        );
    }

    #[test]
    fn test_categorize_feature() {
        let task = make_task("Add new auth module", None);
        assert_eq!(
            ChangelogGenerator::categorize_task(&task),
            ChangeCategory::Features
        );
    }

    #[test]
    fn test_categorize_new() {
        let task = make_task("New feature for dashboard", None);
        assert_eq!(
            ChangelogGenerator::categorize_task(&task),
            ChangeCategory::Features
        );
    }

    #[test]
    fn test_categorize_implement() {
        let task = make_task("Implement caching layer", None);
        assert_eq!(
            ChangelogGenerator::categorize_task(&task),
            ChangeCategory::Features
        );
    }

    #[test]
    fn test_categorize_refactor() {
        let task = make_task("Refactor database layer", None);
        assert_eq!(
            ChangelogGenerator::categorize_task(&task),
            ChangeCategory::Improvements
        );
    }

    #[test]
    fn test_categorize_improve() {
        let task = make_task("Improve performance of queries", None);
        assert_eq!(
            ChangelogGenerator::categorize_task(&task),
            ChangeCategory::Improvements
        );
    }

    #[test]
    fn test_categorize_optimize() {
        let task = make_task("Optimize memory allocation", None);
        assert_eq!(
            ChangelogGenerator::categorize_task(&task),
            ChangeCategory::Improvements
        );
    }

    #[test]
    fn test_categorize_test() {
        let task = make_task("Add tests for auth module", None);
        assert_eq!(ChangelogGenerator::categorize_task(&task), ChangeCategory::Other);
    }

    #[test]
    fn test_categorize_docs() {
        let task = make_task("Update documentation", None);
        assert_eq!(ChangelogGenerator::categorize_task(&task), ChangeCategory::Other);
    }

    #[test]
    fn test_categorize_default_is_features() {
        let task = make_task("Update config values", None);
        assert_eq!(
            ChangelogGenerator::categorize_task(&task),
            ChangeCategory::Features
        );
    }

    #[test]
    fn test_categorize_case_insensitive() {
        let task = make_task("FIX critical issue", None);
        assert_eq!(ChangelogGenerator::categorize_task(&task), ChangeCategory::BugFixes);
    }

    #[test]
    fn test_generate_changelog_multiple_categories() {
        let tasks = vec![
            make_task("Fix login bug", Some(85.0)),
            make_task("Add new dashboard", Some(92.0)),
            make_task("Breaking change to API", Some(70.0)),
            make_task("Refactor database layer", Some(88.0)),
        ];
        let diff = make_diff_context(vec!["src/auth.rs", "src/dashboard.rs"]);

        let changelog = ChangelogGenerator::generate(&tasks, &[], &diff);

        assert!(changelog.starts_with("# Changelog"));
        assert!(changelog.contains("## Bug Fixes"));
        assert!(changelog.contains("## Features"));
        assert!(changelog.contains("## Breaking Changes"));
        assert!(changelog.contains("## Improvements"));
        assert!(changelog.contains("Fix login bug"));
        assert!(changelog.contains("Add new dashboard"));
        assert!(changelog.contains("Breaking change to API"));
        assert!(changelog.contains("Refactor database layer"));
    }

    #[test]
    fn test_generate_changelog_empty_tasks() {
        let diff = make_diff_context(vec![]);
        let commits = vec!["abc1234 Initial commit".to_string()];
        let changelog = ChangelogGenerator::generate(&[], &commits, &diff);
        assert!(changelog.contains("## Other Changes"));
        assert!(changelog.contains("abc1234"));
    }

    #[test]
    fn test_generate_changelog_empty_everything() {
        let diff = make_diff_context(vec![]);
        let changelog = ChangelogGenerator::generate(&[], &[], &diff);
        assert!(changelog.starts_with("# Changelog"));
    }

    #[test]
    fn test_format_changelog_score_display() {
        let mut sections: HashMap<ChangeCategory, Vec<ChangeEntry>> = HashMap::new();
        sections.insert(
            ChangeCategory::Features,
            vec![ChangeEntry {
                description: "New feature".into(),
                score: Some(95.0),
                files_changed: vec![],
            }],
        );
        let output = ChangelogGenerator::format_changelog(&sections);
        assert!(output.contains("95/100"));
    }

    #[test]
    fn test_format_changelog_no_score() {
        let mut sections: HashMap<ChangeCategory, Vec<ChangeEntry>> = HashMap::new();
        sections.insert(
            ChangeCategory::Other,
            vec![ChangeEntry {
                description: "Some change".into(),
                score: None,
                files_changed: vec![],
            }],
        );
        let output = ChangelogGenerator::format_changelog(&sections);
        assert!(output.contains("score: -"));
    }

    #[test]
    fn test_category_ordered() {
        let ordered = ChangeCategory::ordered();
        assert_eq!(ordered[0], ChangeCategory::BreakingChanges);
        assert_eq!(ordered[1], ChangeCategory::Features);
        assert_eq!(ordered[2], ChangeCategory::BugFixes);
        assert_eq!(ordered[3], ChangeCategory::Improvements);
        assert_eq!(ordered[4], ChangeCategory::Other);
    }

    #[test]
    fn test_category_titles() {
        assert_eq!(ChangeCategory::Features.title(), "Features");
        assert_eq!(ChangeCategory::BugFixes.title(), "Bug Fixes");
        assert_eq!(ChangeCategory::BreakingChanges.title(), "Breaking Changes");
        assert_eq!(ChangeCategory::Improvements.title(), "Improvements");
        assert_eq!(ChangeCategory::Other.title(), "Other Changes");
    }
}
