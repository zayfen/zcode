//! Scoring engine — weighted aggregation and normalization

use crate::verification::types::{
    IssueSeverity, VerificationIssue, VerificationResult, VerificationScore, VerifierScoreEntry,
};
use crate::verification::policy::VerificationPolicy;

/// Score a set of verification results into a single VerificationScore
pub fn aggregate_scores(
    results: &[VerificationResult],
    policy: &VerificationPolicy,
) -> VerificationScore {
    if results.is_empty() {
        return VerificationScore::new(0.0, false, vec![], vec![]);
    }

    let mut breakdown = Vec::new();
    let mut total_weight = 0.0;
    let mut weighted_sum = 0.0;
    let mut all_issues: Vec<&VerificationIssue> = Vec::new();

    for result in results {
        let weight = policy.verifier_weight(&result.verifier_name, 1.0);
        let weighted_score = result.score * weight;

        breakdown.push(VerifierScoreEntry {
            name: result.verifier_name.clone(),
            score: result.score,
            weight,
            weighted_score,
            issues_count: result.issues.len(),
        });

        total_weight += weight;
        weighted_sum += weighted_score;
        all_issues.extend(result.issues.iter());
    }

    let total = if total_weight > 0.0 {
        (weighted_sum / total_weight).clamp(0.0, 100.0)
    } else {
        0.0
    };

    let passed = total >= policy.min_score;

    // Sort issues by severity (most severe first), take top 10
    all_issues.sort_by(|a, b| b.severity.cmp(&a.severity));
    let top_issues: Vec<VerificationIssue> = all_issues
        .into_iter()
        .take(10)
        .cloned()
        .collect();

    VerificationScore::new(total, passed, breakdown, top_issues)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_policy() -> VerificationPolicy {
        VerificationPolicy::default()
    }

    #[test]
    fn test_empty_results() {
        let score = aggregate_scores(&[], &make_policy());
        assert_eq!(score.total, 0.0);
        assert!(!score.passed);
    }

    #[test]
    fn test_single_passing_result() {
        let results = vec![VerificationResult::passed("test")];
        let score = aggregate_scores(&results, &make_policy());
        assert_eq!(score.total, 100.0);
        assert!(score.passed);
        assert_eq!(score.breakdown.len(), 1);
    }

    #[test]
    fn test_single_failing_result() {
        let results = vec![VerificationResult::with_issues(
            "test",
            50.0,
            vec![VerificationIssue::new(IssueSeverity::High, "test", "fail")],
        )];
        let score = aggregate_scores(&results, &make_policy());
        assert_eq!(score.total, 50.0);
        assert!(!score.passed); // 50 < 70
    }

    #[test]
    fn test_multiple_results_averaged() {
        let results = vec![
            VerificationResult::with_issues("test", 80.0, vec![]),
            VerificationResult::with_issues("lint", 60.0, vec![]),
        ];
        let score = aggregate_scores(&results, &make_policy());
        // (80 + 60) / 2 = 70.0
        assert!((score.total - 70.0).abs() < 0.01);
        assert!(score.passed); // exactly 70
    }

    #[test]
    fn test_weighted_results() {
        let mut overrides = std::collections::HashMap::new();
        overrides.insert("test".into(), 2.0);
        overrides.insert("lint".into(), 1.0);
        let policy = VerificationPolicy {
            weight_overrides: overrides,
            ..Default::default()
        };

        let results = vec![
            VerificationResult::with_issues("test", 90.0, vec![]),
            VerificationResult::with_issues("lint", 60.0, vec![]),
        ];
        let score = aggregate_scores(&results, &policy);
        // (90*2 + 60*1) / (2+1) = 240/3 = 80
        assert!((score.total - 80.0).abs() < 0.01);
    }

    #[test]
    fn test_top_issues_sorted_by_severity() {
        let results = vec![VerificationResult::with_issues(
            "test",
            50.0,
            vec![
                VerificationIssue::new(IssueSeverity::Low, "style", "minor"),
                VerificationIssue::new(IssueSeverity::Critical, "sec", "bad"),
                VerificationIssue::new(IssueSeverity::High, "logic", "wrong"),
            ],
        )];
        let score = aggregate_scores(&results, &make_policy());
        assert_eq!(score.top_issues.len(), 3);
        assert_eq!(score.top_issues[0].severity, IssueSeverity::Critical);
        assert_eq!(score.top_issues[1].severity, IssueSeverity::High);
        assert_eq!(score.top_issues[2].severity, IssueSeverity::Low);
    }
}
