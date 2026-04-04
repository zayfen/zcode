//! Knowledge management from multiple sources
//!
//! Aggregates knowledge from project docs, external APIs, and learned patterns.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Kind of knowledge fragment
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnowledgeKind {
    /// API usage examples
    ApiUsage,
    /// Architecture documentation
    Architecture,
    /// Code examples
    CodeExample,
    /// Dependency documentation
    DependencyDoc,
    /// Project-specific context
    ProjectContext,
    /// Best practices
    BestPractice,
    /// Other
    Other,
}

/// A query for knowledge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeQuery {
    /// What requirement is being addressed
    pub requirement: String,
    /// Relevant concepts or keywords
    #[serde(default)]
    pub concepts: Vec<String>,
    /// Preferred kind of knowledge
    pub kind: KnowledgeKind,
}

impl KnowledgeQuery {
    /// Create a new knowledge query
    pub fn new(requirement: impl Into<String>, kind: KnowledgeKind) -> Self {
        Self {
            requirement: requirement.into(),
            concepts: Vec::new(),
            kind,
        }
    }

    /// Add a concept
    pub fn with_concept(mut self, concept: impl Into<String>) -> Self {
        self.concepts.push(concept.into());
        self
    }

    /// Extract keywords for searching
    pub fn keywords(&self) -> Vec<String> {
        let mut keywords = Vec::new();

        // Add words from requirement
        for word in self.requirement.split_whitespace() {
            if word.len() > 3 {
                keywords.push(word.to_lowercase());
            }
        }

        // Add concepts
        keywords.extend(self.concepts.iter().map(|c| c.to_lowercase()));

        keywords
    }
}

/// A fragment of knowledge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeFragment {
    /// Where this knowledge came from
    pub source: String,
    /// The knowledge content
    pub content: String,
    /// Relevance score (0-1)
    pub relevance_score: f32,
    /// Kind of knowledge
    pub kind: KnowledgeKind,
    /// Estimated token count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_tokens: Option<usize>,
}

impl KnowledgeFragment {
    /// Create a new knowledge fragment
    pub fn new(
        source: impl Into<String>,
        content: impl Into<String>,
        relevance_score: f32,
        kind: KnowledgeKind,
    ) -> Self {
        let content = content.into();
        let estimated_tokens = Some(content.len().div_ceil(4));

        Self {
            source: source.into(),
            content,
            relevance_score,
            kind,
            estimated_tokens,
        }
    }

    /// Check if this fragment is relevant enough
    pub fn is_relevant(&self, threshold: f32) -> bool {
        self.relevance_score >= threshold
    }
}

/// Assembled knowledge context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeContext {
    /// Knowledge fragments
    pub fragments: Vec<KnowledgeFragment>,
    /// Total estimated tokens
    pub estimated_tokens: usize,
    /// Whether some results were truncated
    pub truncated: bool,
}

impl KnowledgeContext {
    /// Create an empty context
    pub fn empty() -> Self {
        Self {
            fragments: Vec::new(),
            estimated_tokens: 0,
            truncated: false,
        }
    }

    /// Create from fragments
    pub fn from_fragments(fragments: Vec<KnowledgeFragment>, max_tokens: Option<usize>) -> Self {
        let mut total_tokens = 0usize;
        let mut kept_fragments = Vec::new();
        let mut truncated = false;

        for fragment in fragments {
            let frag_tokens = fragment.estimated_tokens.unwrap_or_else(|| {
                fragment.content.len().div_ceil(4)
            });

            if let Some(max) = max_tokens {
                if total_tokens + frag_tokens > max {
                    truncated = true;
                    break;
                }
            }

            total_tokens += frag_tokens;
            kept_fragments.push(fragment);
        }

        Self {
            fragments: kept_fragments,
            estimated_tokens: total_tokens,
            truncated,
        }
    }

    /// Check if context is empty
    pub fn is_empty(&self) -> bool {
        self.fragments.is_empty()
    }

    /// Number of fragments
    pub fn len(&self) -> usize {
        self.fragments.len()
    }

    /// Render as formatted text
    pub fn render(&self) -> String {
        if self.fragments.is_empty() {
            return String::new();
        }

        let mut parts = Vec::new();

        for fragment in &self.fragments {
            parts.push(format!(
                "## {} (relevance: {:.2}, kind: {:?})\n{}",
                fragment.source,
                fragment.relevance_score,
                fragment.kind,
                fragment.content
            ));
        }

        if self.truncated {
            parts.push(
                "⚠️ Some knowledge fragments were truncated due to token limits.".to_string()
            );
        }

        parts.join("\n\n")
    }

    /// Get fragments by kind
    pub fn by_kind(&self, kind: KnowledgeKind) -> Vec<&KnowledgeFragment> {
        self.fragments.iter()
            .filter(|f| f.kind == kind)
            .collect()
    }

    /// Get fragments above a relevance threshold
    pub fn filter_relevant(&self, threshold: f32) -> Vec<&KnowledgeFragment> {
        self.fragments.iter()
            .filter(|f| f.is_relevant(threshold))
            .collect()
    }
}

/// Trait for knowledge sources
#[async_trait]
pub trait KnowledgeSource: Send + Sync {
    /// Name of this knowledge source
    fn name(&self) -> &str;

    /// Query this knowledge source
    ///
    /// Returns a String error instead of ZcodeError to avoid circular dependencies
    /// and keep the trait generic.
    async fn query(&self, query: &KnowledgeQuery) -> std::result::Result<Vec<KnowledgeFragment>, String>;

    /// Estimate relevance of this source for a query (0-1)
    fn relevance(&self, query: &KnowledgeQuery) -> f32 {
        let keywords = query.keywords();
        let source_name = self.name().to_lowercase();

        let matching_keywords = keywords.iter()
            .filter(|k| source_name.contains(k.as_str()))
            .count();

        if matching_keywords > 0 {
            matching_keywords as f32 / keywords.len() as f32
        } else {
            0.5 // Default moderate relevance
        }
    }
}

/// Simple in-memory knowledge source for testing
#[derive(Debug, Clone)]
pub struct InMemoryKnowledgeSource {
    name: String,
    fragments: Vec<(KnowledgeKind, String, String)>, // (kind, source, content)
}

impl InMemoryKnowledgeSource {
    /// Create a new in-memory source
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            fragments: Vec::new(),
        }
    }

    /// Add a knowledge fragment
    pub fn add(&mut self, kind: KnowledgeKind, source: impl Into<String>, content: impl Into<String>) {
        self.fragments.push((kind, source.into(), content.into()));
    }
}

#[async_trait]
impl KnowledgeSource for InMemoryKnowledgeSource {
    fn name(&self) -> &str {
        &self.name
    }

    async fn query(&self, query: &KnowledgeQuery) -> std::result::Result<Vec<KnowledgeFragment>, String> {
        let mut results = Vec::new();
        let keywords = query.keywords();

        for (kind, source, content) in &self.fragments {
            // Filter by kind
            if query.kind != KnowledgeKind::Other && *kind != query.kind && *kind != KnowledgeKind::Other {
                continue;
            }

            // Calculate relevance based on keyword matching
            let content_lower = content.to_lowercase();
            let matches = keywords.iter()
                .filter(|k: &&String| content_lower.contains(k.as_str()))
                .count();

            let relevance = if matches > 0 {
                matches as f32 / keywords.len() as f32
            } else {
                0.1 // Low base relevance
            };

            if relevance > 0.2 {
                results.push(KnowledgeFragment::new(
                    source.clone(),
                    content.clone(),
                    relevance,
                    kind.clone(),
                ));
            }
        }

        results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_knowledge_query_new() {
        let query = KnowledgeQuery::new("How to use async", KnowledgeKind::ApiUsage);
        assert_eq!(query.requirement, "How to use async");
        assert!(query.concepts.is_empty());
        assert_eq!(query.kind, KnowledgeKind::ApiUsage);
    }

    #[test]
    fn test_knowledge_query_with_concept() {
        let query = KnowledgeQuery::new("Query", KnowledgeKind::CodeExample)
            .with_concept("rust")
            .with_concept("tokio");

        assert_eq!(query.concepts.len(), 2);
    }

    #[test]
    fn test_knowledge_query_keywords() {
        let query = KnowledgeQuery::new("How to use async await in Rust", KnowledgeKind::ApiUsage)
            .with_concept("tokio");

        let keywords = query.keywords();
        assert!(keywords.contains(&"async".to_string()));
        assert!(keywords.contains(&"tokio".to_string()));
    }

    #[test]
    fn test_knowledge_fragment_new() {
        let fragment = KnowledgeFragment::new(
            "docs",
            "Use async/await",
            0.9,
            KnowledgeKind::ApiUsage
        );

        assert_eq!(fragment.source, "docs");
        assert_eq!(fragment.content, "Use async/await");
        assert!((fragment.relevance_score - 0.9).abs() < f32::EPSILON);
        assert_eq!(fragment.kind, KnowledgeKind::ApiUsage);
        assert!(fragment.estimated_tokens.is_some());
    }

    #[test]
    fn test_knowledge_fragment_is_relevant() {
        let fragment = KnowledgeFragment::new(
            "src",
            "content",
            0.7,
            KnowledgeKind::Other
        );

        assert!(fragment.is_relevant(0.5));
        assert!(fragment.is_relevant(0.7));
        assert!(!fragment.is_relevant(0.8));
    }

    #[test]
    fn test_knowledge_context_empty() {
        let ctx = KnowledgeContext::empty();
        assert!(ctx.is_empty());
        assert_eq!(ctx.len(), 0);
        assert_eq!(ctx.estimated_tokens, 0);
    }

    #[test]
    fn test_knowledge_context_from_fragments() {
        let fragments = vec![
            KnowledgeFragment::new("src1", "content one", 0.9, KnowledgeKind::CodeExample),
            KnowledgeFragment::new("src2", "content two", 0.8, KnowledgeKind::CodeExample),
        ];

        let ctx = KnowledgeContext::from_fragments(fragments, Some(100));
        assert_eq!(ctx.len(), 2);
        assert!(ctx.estimated_tokens > 0);
        assert!(!ctx.truncated);
    }

    #[test]
    fn test_knowledge_context_truncation() {
        let fragments = vec![
            KnowledgeFragment::new("src", "x".repeat(1000), 0.9, KnowledgeKind::CodeExample),
            KnowledgeFragment::new("src", "y".repeat(1000), 0.8, KnowledgeKind::CodeExample),
        ];

        let ctx = KnowledgeContext::from_fragments(fragments, Some(100));
        assert!(ctx.truncated);
        assert!(ctx.len() < 2);
    }

    #[test]
    fn test_knowledge_context_render() {
        let mut ctx = KnowledgeContext::empty();
        ctx.fragments.push(KnowledgeFragment::new(
            "docs",
            "Use async",
            0.9,
            KnowledgeKind::ApiUsage
        ));

        let rendered = ctx.render();
        assert!(rendered.contains("docs"));
        assert!(rendered.contains("Use async"));
        assert!(rendered.contains("0.90"));
    }

    #[test]
    fn test_knowledge_context_by_kind() {
        let mut ctx = KnowledgeContext::empty();
        ctx.fragments.push(KnowledgeFragment::new(
            "src1", "content", 0.9, KnowledgeKind::ApiUsage
        ));
        ctx.fragments.push(KnowledgeFragment::new(
            "src2", "content", 0.8, KnowledgeKind::Architecture
        ));

        let api_fragments = ctx.by_kind(KnowledgeKind::ApiUsage);
        assert_eq!(api_fragments.len(), 1);
    }

    #[test]
    fn test_in_memory_knowledge_source() {
        let mut source = InMemoryKnowledgeSource::new("test");
        source.add(KnowledgeKind::ApiUsage, "docs", "Use async/await for concurrency");

        assert_eq!(source.name(), "test");
        assert_eq!(source.fragments.len(), 1);
    }

    #[test]
    fn test_knowledge_kind_serialization() {
        let kind = KnowledgeKind::ApiUsage;
        let json = serde_json::to_string(&kind).unwrap();
        let decoded: KnowledgeKind = serde_json::from_str(&json).unwrap();
        assert_eq!(kind, decoded);
    }

    #[test]
    fn test_knowledge_query_serialization() {
        let query = KnowledgeQuery::new("test", KnowledgeKind::CodeExample);
        let json = serde_json::to_string(&query).unwrap();
        let decoded: KnowledgeQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(query.requirement, decoded.requirement);
    }
}
