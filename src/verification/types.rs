//! Verification types — context, results, scores, issues

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Verification context — read-only info passed to each Verifier
#[derive(Debug, Clone)]
pub struct VerificationContext {
    /// Original requirement description
    pub requirement: String,

    /// Current task description
    pub task_description: String,

    /// Pre-execution workspace snapshot id
    pub pre_snapshot_id: Option<i64>,

    /// Git diff patch of the changes
    pub diff_patch: String,

    /// Changed files with their content (path, content)
    pub changed_files: Vec<(String, String)>,

    /// Project root path
    pub project_root: PathBuf,
}

/// Single Verifier's result
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// Verifier name
    pub verifier_name: String,

    /// Score (0.0 - 100.0)
    pub score: f64,

    /// Issues found
    pub issues: Vec<VerificationIssue>,

    /// Verifier log output
    pub log: String,
}

impl VerificationResult {
    /// Create a passing result with full score
    pub fn passed(verifier_name: impl Into<String>) -> Self {
        Self {
            verifier_name: verifier_name.into(),
            score: 100.0,
            issues: vec![],
            log: String::new(),
        }
    }

    /// Create a result with a specific score and issues
    pub fn with_issues(
        verifier_name: impl Into<String>,
        score: f64,
        issues: Vec<VerificationIssue>,
    ) -> Self {
        Self {
            verifier_name: verifier_name.into(),
            score,
            issues,
            log: String::new(),
        }
    }

    /// Create a skipped result (score 0, no issues)
    pub fn skipped(verifier_name: impl Into<String>, reason: &str) -> Self {
        Self {
            verifier_name: verifier_name.into(),
            score: 0.0,
            issues: vec![],
            log: format!("Skipped: {}", reason),
        }
    }
}

/// Weighted verification score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationScore {
    /// Weighted total score (0.0 - 100.0)
    pub total: f64,

    /// Per-verifier score breakdown
    pub breakdown: Vec<VerifierScoreEntry>,

    /// Whether the score passes the minimum threshold
    pub passed: bool,

    /// Top-N issues sorted by severity
    pub top_issues: Vec<VerificationIssue>,
}

impl VerificationScore {
    /// Create a score from verifier results
    pub fn new(
        total: f64,
        passed: bool,
        breakdown: Vec<VerifierScoreEntry>,
        top_issues: Vec<VerificationIssue>,
    ) -> Self {
        Self {
            total,
            passed,
            breakdown,
            top_issues,
        }
    }
}

/// Per-verifier score entry in the breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifierScoreEntry {
    /// Verifier name
    pub name: String,
    /// Raw score (0-100)
    pub score: f64,
    /// Weight used
    pub weight: f64,
    /// Weighted score contribution
    pub weighted_score: f64,
    /// Number of issues found
    pub issues_count: usize,
}

/// Single verification issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationIssue {
    /// Severity level
    pub severity: IssueSeverity,
    /// Issue category
    pub category: String,
    /// Issue description
    pub message: String,
    /// Fix suggestion
    pub suggestion: String,
    /// File location (if applicable)
    pub location: Option<FileLocation>,
    /// Associated code snippet
    pub snippet: Option<String>,
}

impl VerificationIssue {
    /// Create a new issue
    pub fn new(
        severity: IssueSeverity,
        category: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            category: category.into(),
            message: message.into(),
            suggestion: String::new(),
            location: None,
            snippet: None,
        }
    }

    /// Add a suggestion
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = suggestion.into();
        self
    }

    /// Add a file location
    pub fn with_location(mut self, path: impl Into<String>, line_start: Option<usize>) -> Self {
        self.location = Some(FileLocation {
            path: path.into(),
            line_start,
            line_end: None,
        });
        self
    }

    /// Add a code snippet
    pub fn with_snippet(mut self, snippet: impl Into<String>) -> Self {
        self.snippet = Some(snippet.into());
        self
    }

    /// Penalty deduction based on severity
    pub fn deduction(&self) -> f64 {
        self.severity.deduction()
    }
}

/// Issue severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl IssueSeverity {
    /// Score deduction for this severity level
    pub fn deduction(&self) -> f64 {
        match self {
            Self::Critical => 20.0,
            Self::High => 12.0,
            Self::Medium => 6.0,
            Self::Low => 2.0,
            Self::Info => 0.0,
        }
    }
}

impl std::fmt::Display for IssueSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Critical => write!(f, "critical"),
            Self::High => write!(f, "high"),
            Self::Medium => write!(f, "medium"),
            Self::Low => write!(f, "low"),
            Self::Info => write!(f, "info"),
        }
    }
}

/// File location for an issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileLocation {
    pub path: String,
    pub line_start: Option<usize>,
    pub line_end: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification_result_passed() {
        let r = VerificationResult::passed("test");
        assert_eq!(r.verifier_name, "test");
        assert_eq!(r.score, 100.0);
        assert!(r.issues.is_empty());
    }

    #[test]
    fn test_verification_result_with_issues() {
        let issues = vec![VerificationIssue::new(
            IssueSeverity::High,
            "test",
            "something wrong",
        )];
        let r = VerificationResult::with_issues("lint", 60.0, issues);
        assert_eq!(r.score, 60.0);
        assert_eq!(r.issues.len(), 1);
    }

    #[test]
    fn test_verification_result_skipped() {
        let r = VerificationResult::skipped("coverage", "no tool");
        assert_eq!(r.score, 0.0);
        assert!(r.log.contains("Skipped"));
    }

    #[test]
    fn test_issue_deduction() {
        assert_eq!(IssueSeverity::Critical.deduction(), 20.0);
        assert_eq!(IssueSeverity::High.deduction(), 12.0);
        assert_eq!(IssueSeverity::Medium.deduction(), 6.0);
        assert_eq!(IssueSeverity::Low.deduction(), 2.0);
        assert_eq!(IssueSeverity::Info.deduction(), 0.0);
    }

    #[test]
    fn test_issue_builder() {
        let issue = VerificationIssue::new(IssueSeverity::Critical, "security", "hardcoded key")
            .with_suggestion("use env var")
            .with_location("src/main.rs", Some(42))
            .with_snippet("api_key = \"abc\"");
        assert_eq!(issue.severity, IssueSeverity::Critical);
        assert_eq!(issue.suggestion, "use env var");
        assert!(issue.location.is_some());
        assert!(issue.snippet.is_some());
    }

    #[test]
    fn test_severity_ordering() {
        assert!(IssueSeverity::Critical > IssueSeverity::High);
        assert!(IssueSeverity::High > IssueSeverity::Medium);
        assert!(IssueSeverity::Medium > IssueSeverity::Low);
        assert!(IssueSeverity::Low > IssueSeverity::Info);
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(format!("{}", IssueSeverity::Critical), "critical");
        assert_eq!(format!("{}", IssueSeverity::Info), "info");
    }
}
