//! Session memory types for tracking decisions and learned patterns
//!
//! Records what happened during a session to improve future performance.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Type of memory record
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryType {
    /// Session summary
    SessionSummary,
    /// Architectural or implementation decision
    Decision,
    /// Learned pattern or convention
    LearnedPattern,
    /// Project-specific knowledge
    ProjectKnowledge,
}

impl fmt::Display for MemoryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryType::SessionSummary => write!(f, "session_summary"),
            MemoryType::Decision => write!(f, "decision"),
            MemoryType::LearnedPattern => write!(f, "learned_pattern"),
            MemoryType::ProjectKnowledge => write!(f, "project_knowledge"),
        }
    }
}

/// A decision made during a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRecord {
    /// Description of the decision
    pub description: String,
    /// Rationale for why this decision was made
    pub rationale: String,
    /// Relevant context files
    pub context_files: Vec<String>,
    /// Optional embedding for semantic search
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
}

impl DecisionRecord {
    /// Create a new decision record
    pub fn new(description: impl Into<String>, rationale: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            rationale: rationale.into(),
            context_files: Vec::new(),
            embedding: None,
        }
    }

    /// Add a context file
    pub fn with_context_file(mut self, file: impl Into<String>) -> Self {
        self.context_files.push(file.into());
        self
    }

    /// Set the embedding
    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = Some(embedding);
        self
    }
}

/// A learned pattern discovered during a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedPattern {
    /// Name of the pattern
    pub name: String,
    /// Description of how to apply it
    pub description: String,
    /// Code examples demonstrating the pattern
    pub examples: Vec<String>,
    /// Optional embedding for semantic search
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
}

impl LearnedPattern {
    /// Create a new learned pattern
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            examples: Vec::new(),
            embedding: None,
        }
    }

    /// Add an example
    pub fn with_example(mut self, example: impl Into<String>) -> Self {
        self.examples.push(example.into());
        self
    }

    /// Set the embedding
    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = Some(embedding);
        self
    }
}

/// Session memory containing decisions and learned patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMemory {
    /// Session identifier (e.g., UUID or timestamp)
    pub session_id: String,
    /// Summary of what was accomplished
    pub summary: String,
    /// Key decisions made during the session
    #[serde(default)]
    pub decisions: Vec<DecisionRecord>,
    /// Patterns learned during the session
    #[serde(default)]
    pub learned_patterns: Vec<LearnedPattern>,
    /// When the session was created (as RFC3339 string for serialization)
    pub created_at: String,
}

impl SessionMemory {
    /// Create a new session memory
    pub fn new(session_id: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            summary: summary.into(),
            decisions: Vec::new(),
            learned_patterns: Vec::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Add a decision
    pub fn add_decision(&mut self, decision: DecisionRecord) {
        self.decisions.push(decision);
    }

    /// Add a learned pattern
    pub fn add_learned_pattern(&mut self, pattern: LearnedPattern) {
        self.learned_patterns.push(pattern);
    }

    /// Get total estimated tokens
    pub fn estimated_tokens(&self) -> usize {
        let summary_tokens = self.summary.len().div_ceil(4);
        let decisions_tokens: usize = self
            .decisions
            .iter()
            .map(|d| d.description.len() + d.rationale.len())
            .sum::<usize>()
            .div_ceil(4);
        let patterns_tokens: usize = self
            .learned_patterns
            .iter()
            .map(|p| p.name.len() + p.description.len())
            .sum::<usize>()
            .div_ceil(4);

        summary_tokens + decisions_tokens + patterns_tokens
    }
}

/// A recalled memory with relevance score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecall {
    /// Type of memory
    pub memory_type: MemoryType,
    /// Content of the memory
    pub content: String,
    /// Relevance score (0-1, higher is better)
    pub relevance_score: f32,
    /// Associated session ID (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

impl MemoryRecall {
    /// Create a new memory recall
    pub fn new(memory_type: MemoryType, content: impl Into<String>, relevance_score: f32) -> Self {
        Self {
            memory_type,
            content: content.into(),
            relevance_score,
            session_id: None,
        }
    }

    /// Set the session ID
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Check if this memory is relevant enough
    pub fn is_relevant(&self, threshold: f32) -> bool {
        self.relevance_score >= threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_type_display() {
        assert_eq!(MemoryType::SessionSummary.to_string(), "session_summary");
        assert_eq!(MemoryType::Decision.to_string(), "decision");
        assert_eq!(MemoryType::LearnedPattern.to_string(), "learned_pattern");
        assert_eq!(MemoryType::ProjectKnowledge.to_string(), "project_knowledge");
    }

    #[test]
    fn test_decision_record_new() {
        let decision = DecisionRecord::new("Use X", "Because Y");
        assert_eq!(decision.description, "Use X");
        assert_eq!(decision.rationale, "Because Y");
        assert!(decision.context_files.is_empty());
        assert!(decision.embedding.is_none());
    }

    #[test]
    fn test_decision_record_with_context() {
        let decision = DecisionRecord::new("Use X", "Because Y")
            .with_context_file("a.rs")
            .with_context_file("b.rs");

        assert_eq!(decision.context_files.len(), 2);
        assert_eq!(decision.context_files[0], "a.rs");
        assert_eq!(decision.context_files[1], "b.rs");
    }

    #[test]
    fn test_decision_record_with_embedding() {
        let decision = DecisionRecord::new("Use X", "Because Y")
            .with_embedding(vec![1.0, 2.0, 3.0]);

        assert_eq!(decision.embedding, Some(vec![1.0, 2.0, 3.0]));
    }

    #[test]
    fn test_learned_pattern_new() {
        let pattern = LearnedPattern::new("Error handling", "Use Result types");
        assert_eq!(pattern.name, "Error handling");
        assert_eq!(pattern.description, "Use Result types");
        assert!(pattern.examples.is_empty());
        assert!(pattern.embedding.is_none());
    }

    #[test]
    fn test_learned_pattern_with_examples() {
        let pattern = LearnedPattern::new("Pattern", "Description")
            .with_example("example 1")
            .with_example("example 2");

        assert_eq!(pattern.examples.len(), 2);
    }

    #[test]
    fn test_session_memory_new() {
        let memory = SessionMemory::new("session-1", "Fixed a bug");
        assert_eq!(memory.session_id, "session-1");
        assert_eq!(memory.summary, "Fixed a bug");
        assert!(memory.decisions.is_empty());
        assert!(memory.learned_patterns.is_empty());
    }

    #[test]
    fn test_session_memory_add_decision() {
        let mut memory = SessionMemory::new("session-1", "Summary");
        memory.add_decision(DecisionRecord::new("Decision", "Rationale"));

        assert_eq!(memory.decisions.len(), 1);
    }

    #[test]
    fn test_session_memory_add_pattern() {
        let mut memory = SessionMemory::new("session-1", "Summary");
        memory.add_learned_pattern(LearnedPattern::new("Pattern", "Description"));

        assert_eq!(memory.learned_patterns.len(), 1);
    }

    #[test]
    fn test_session_memory_estimated_tokens() {
        let mut memory = SessionMemory::new("session-1", "A summary of work done");
        memory.add_decision(DecisionRecord::new("Decision desc", "Decision rationale"));
        memory.add_learned_pattern(LearnedPattern::new("Pattern name", "Pattern description"));

        let tokens = memory.estimated_tokens();
        assert!(tokens > 0);
    }

    #[test]
    fn test_memory_recall_new() {
        let recall = MemoryRecall::new(MemoryType::Decision, "Use Result types", 0.9);
        assert_eq!(recall.memory_type, MemoryType::Decision);
        assert_eq!(recall.content, "Use Result types");
        assert!((recall.relevance_score - 0.9).abs() < f32::EPSILON);
        assert!(recall.session_id.is_none());
    }

    #[test]
    fn test_memory_recall_with_session() {
        let recall = MemoryRecall::new(MemoryType::Decision, "Content", 0.8)
            .with_session("session-1");

        assert_eq!(recall.session_id, Some("session-1".to_string()));
    }

    #[test]
    fn test_memory_recall_is_relevant() {
        let recall = MemoryRecall::new(MemoryType::Decision, "Content", 0.7);
        assert!(recall.is_relevant(0.5));
        assert!(recall.is_relevant(0.7));
        assert!(!recall.is_relevant(0.8));
    }

    #[test]
    fn test_memory_type_serialization() {
        let mt = MemoryType::LearnedPattern;
        let json = serde_json::to_string(&mt).unwrap();
        let decoded: MemoryType = serde_json::from_str(&json).unwrap();
        assert_eq!(mt, decoded);
    }

    #[test]
    fn test_decision_record_serialization() {
        let decision = DecisionRecord::new("Use X", "Because Y")
            .with_context_file("test.rs");
        let json = serde_json::to_string(&decision).unwrap();
        let decoded: DecisionRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(decision.description, decoded.description);
        assert_eq!(decision.context_files, decoded.context_files);
    }
}
