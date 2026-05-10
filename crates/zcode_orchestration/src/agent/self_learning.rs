//! Self-learning agent.
//!
//! Records recurring engineering failures and lessons learned as a deterministic
//! "mistake book" document entry.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningEntry {
    pub title: String,
    pub context: String,
    pub mistake: String,
    pub correction: String,
}

pub struct SelfLearningAgent;

impl SelfLearningAgent {
    pub fn summarize(session_report: &str) -> LearningEntry {
        let title = session_report
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("Session learning")
            .chars()
            .take(80)
            .collect();

        LearningEntry {
            title,
            context: session_report.to_string(),
            mistake: extract_section(session_report, "FAIL")
                .unwrap_or_else(|| "No explicit failure section found.".to_string()),
            correction: extract_section(session_report, "PASS")
                .unwrap_or_else(|| "Record the final working approach and verification commands.".to_string()),
        }
    }
}

fn extract_section(text: &str, marker: &str) -> Option<String> {
    text.lines()
        .find(|line| line.contains(marker))
        .map(|line| line.trim().to_string())
}

