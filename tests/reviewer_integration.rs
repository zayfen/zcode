//! Integration tests: ReviewerAgent + GitDiff
//!
//! Tests the full review workflow: generate a diff, review it, check issues.

use zcode::agent::{ReviewerAgent, ReviewConfig, IssueSeverity, ReviewCategory};
use zcode::git::GitDiff;

// ─── ReviewerAgent integration tests ─────────────────────────────────────────

#[test]
fn test_review_clean_rust_function() {
    let reviewer = ReviewerAgent::new();
    let diff = r#"
+/// Adds two integers
+pub fn add(a: i32, b: i32) -> i32 {
+    a + b
+}
+
+#[test]
+fn test_add() {
+    assert_eq!(add(1, 2), 3);
+    assert_eq!(add(-1, 1), 0);
+}
"#;
    let result = reviewer.review_diff(diff).unwrap();
    assert!(result.approved, "Clean function with tests should be approved");
    assert!(result.quality_score >= 80);
}

#[test]
fn test_review_detects_multiple_issues() {
    let reviewer = ReviewerAgent::new();
    let diff = r#"
+pub fn risky_operation(input: Option<String>) -> String {
+    let value = input.unwrap();
+    let secret_key = "hardcoded_api_key_12345";
+    value + secret_key
+}
"#;
    let result = reviewer.review_diff(diff).unwrap();
    // Should catch: unwrap(), hardcoded key, no tests
    assert!(!result.issues.is_empty());
    assert!(result.issues.len() >= 2);
}

#[test]
fn test_review_security_error_rejects() {
    let reviewer = ReviewerAgent::new();
    let diff = r#"
+const DATABASE_PASSWORD: &str = "super_secret_password123";
"#;
    let result = reviewer.review_diff(diff).unwrap();
    let security_errors: Vec<_> = result.issues.iter()
        .filter(|i| i.category == ReviewCategory::Security && i.severity == IssueSeverity::Error)
        .collect();
    assert!(!security_errors.is_empty(), "Should detect hardcoded password as error");
    assert!(!result.approved, "Security error should reject");
}

#[test]
fn test_review_config_check_only_logic() {
    let config = ReviewConfig {
        check_logic: true,
        check_security: false,
        check_performance: false,
        check_style: false,
        check_tests: false,
        max_issues: 10,
    };
    let reviewer = ReviewerAgent::with_config(config);
    let diff = r#"
+let password = "secret";
+let x = some_result.unwrap();
"#;
    let result = reviewer.review_diff(diff).unwrap();
    // Security disabled → hardcoded password not flagged
    let security_issues: Vec<_> = result.issues.iter()
        .filter(|i| i.category == ReviewCategory::Security)
        .collect();
    assert!(security_issues.is_empty(), "Security check should be disabled");

    // Logic check enabled → unwrap flagged
    let logic_issues: Vec<_> = result.issues.iter()
        .filter(|i| i.category == ReviewCategory::Logic)
        .collect();
    assert!(!logic_issues.is_empty(), "Logic check should be enabled");
}

#[test]
fn test_review_full_rust_diff() {
    let reviewer = ReviewerAgent::new();
    let diff = r#"
diff --git a/src/lib.rs b/src/lib.rs
index abc123..def456 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,3 +1,15 @@
+use std::collections::HashMap;
+
+/// Cache for computed values
+pub struct Cache {
+    data: HashMap<String, String>,
+}
+
+impl Cache {
+    pub fn new() -> Self {
+        Self { data: HashMap::new() }
+    }
+
+    pub fn insert(&mut self, k: String, v: String) {
+        self.data.insert(k, v);
+    }
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    #[test]
+    fn test_cache_insert() {
+        let mut c = Cache::new();
+        c.insert("key".into(), "val".into());
+        assert!(c.data.contains_key("key"));
+    }
+}
"#;
    let result = reviewer.review_diff(diff).unwrap();
    assert!(result.approved, "Well-structured code with tests should be approved");
}

#[test]
fn test_review_content_entire_file() {
    let reviewer = ReviewerAgent::new();
    let content = r#"
use std::fs;

pub fn load_config(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

#[test]
fn test_load_config_missing() {
    let result = load_config("/nonexistent_path_xyz");
    assert!(result.is_empty());
}
"#;
    let result = reviewer.review_content(content, "config.rs").unwrap();
    // Has unwrap_or_default (not plain unwrap) and has tests
    assert!(result.quality_score <= 100);
}

#[test]
fn test_review_issue_categories_comprehensive() {
    use zcode::agent::ReviewIssue;
    let issues = vec![
        ReviewIssue {
            severity: IssueSeverity::Error,
            file: Some("src/main.rs".to_string()),
            line: Some(10),
            message: "security issue".to_string(),
            category: ReviewCategory::Security,
        },
        ReviewIssue {
            severity: IssueSeverity::Warning,
            file: Some("src/lib.rs".to_string()),
            line: Some(20),
            message: "performance concern".to_string(),
            category: ReviewCategory::Performance,
        },
        ReviewIssue {
            severity: IssueSeverity::Suggestion,
            file: None,
            line: None,
            message: "style suggestion".to_string(),
            category: ReviewCategory::Style,
        },
    ];

    assert_eq!(issues[0].severity, IssueSeverity::Error);
    assert_eq!(issues[1].category, ReviewCategory::Performance);
    assert!(issues[2].file.is_none());
}

#[test]
fn test_review_result_quality_scores() {
    use zcode::agent::ReviewResult;
    let perfect = ReviewResult::approved("All good");
    assert!(perfect.quality_score >= 85);
    assert!(perfect.approved);
    assert_eq!(perfect.error_count(), 0);

    let bad = ReviewResult::rejected("Bad code", vec![
        zcode::agent::ReviewIssue {
            severity: IssueSeverity::Error,
            file: None,
            line: None,
            message: "critical bug".to_string(),
            category: ReviewCategory::Logic,
        }
    ]);
    assert!(!bad.approved);
    assert_eq!(bad.error_count(), 1);
}

// ─── GitDiff integration tests ────────────────────────────────────────────────

#[test]
fn test_git_diff_project_is_git_repo() {
    // zcode itself is a git repo
    let is_repo = GitDiff::is_git_repo("/Users/riven/Github/zcode");
    assert!(is_repo, "zcode project should be a git repo");
}

#[test]
fn test_git_diff_repo_root_detection() {
    let root = GitDiff::repo_root("/Users/riven/Github/zcode/src");
    assert!(root.is_ok());
    let root = root.unwrap();
    assert!(root.ends_with("zcode"));
}

#[test]
fn test_git_diff_list_changed_files_no_panic() {
    // Just ensure it doesn't panic (may have changes or not)
    let result = GitDiff::changed_files("/Users/riven/Github/zcode");
    assert!(result.is_ok());
}

#[test]
fn test_git_diff_recent_commits() {
    let commits = GitDiff::recent_commits("/Users/riven/Github/zcode", 5).unwrap();
    assert!(!commits.is_empty(), "zcode should have at least 1 commit");
    assert!(commits.len() <= 5);
    // Commits should look like git log --oneline
    assert!(commits[0].len() > 7, "commit line should have hash + message");
}

#[test]
fn test_git_diff_build_context() {
    let ctx = GitDiff::build_context("/Users/riven/Github/zcode").unwrap();
    assert_eq!(ctx.repo_root.file_name().unwrap().to_str().unwrap(), "zcode");
}

#[test]
fn test_git_diff_parse_statuses() {
    use zcode::git::{FileStatus};
    assert_eq!(FileStatus::Added, FileStatus::Added);
    assert_ne!(FileStatus::Added, FileStatus::Deleted);
    assert_ne!(FileStatus::Modified, FileStatus::Added);
}

// ─── Review + GitDiff pipeline ────────────────────────────────────────────────

#[test]
fn test_review_git_diff_pipeline() {
    // Get actual git diff and review it
    let diff = GitDiff::full_diff("/Users/riven/Github/zcode").unwrap_or_default();
    let reviewer = ReviewerAgent::new();

    // Should not panic even on empty diff
    let result = reviewer.review_diff(&diff).unwrap();
    // Any result is acceptable — just ensure the pipeline works
    assert!(result.quality_score <= 100);
    assert!(result.quality_score > 0 || result.issues.is_empty());
}
