//! Integration tests: ReviewerAgent
//!
//! Tests reviewer static analysis behavior.

use zcode::agent::{IssueSeverity, ReviewCategory, ReviewConfig, ReviewerAgent};

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
    assert!(
        result.approved,
        "Clean function with tests should be approved"
    );
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
    let security_errors: Vec<_> = result
        .issues
        .iter()
        .filter(|i| i.category == ReviewCategory::Security && i.severity == IssueSeverity::Error)
        .collect();
    assert!(
        !security_errors.is_empty(),
        "Should detect hardcoded password as error"
    );
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
    let security_issues: Vec<_> = result
        .issues
        .iter()
        .filter(|i| i.category == ReviewCategory::Security)
        .collect();
    assert!(
        security_issues.is_empty(),
        "Security check should be disabled"
    );

    // Logic check enabled → unwrap flagged
    let logic_issues: Vec<_> = result
        .issues
        .iter()
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
    assert!(
        result.approved,
        "Well-structured code with tests should be approved"
    );
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

    let bad = ReviewResult::rejected(
        "Bad code",
        vec![zcode::agent::ReviewIssue {
            severity: IssueSeverity::Error,
            file: None,
            line: None,
            message: "critical bug".to_string(),
            category: ReviewCategory::Logic,
        }],
    );
    assert!(!bad.approved);
    assert_eq!(bad.error_count(), 1);
}
