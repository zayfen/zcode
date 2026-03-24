//! Context Assembler — Token Budget Manager
//!
//! Assembles LLM context from working memory, project memory, and semantic index
//! while respecting a token budget.

use crate::memory::{ProjectMemory, SemanticIndex, WorkingMemory};

// ─── Token estimation ──────────────────────────────────────────────────────────

/// Estimate token count from a string (1 token ≈ 4 chars is a common heuristic)
pub fn estimate_tokens(text: &str) -> usize {
    (text.len() + 3) / 4
}

// ─── TokenBudget ───────────────────────────────────────────────────────────────

/// Token budget allocation for a single LLM call
#[derive(Debug, Clone)]
pub struct TokenBudget {
    /// Total token budget
    pub total: usize,
    /// Reserved for system prompt
    pub system: usize,
    /// Reserved for conversation history
    pub conversation: usize,
    /// Reserved for file contents
    pub file_context: usize,
    /// Reserved for tool results
    pub tool_results: usize,
}

impl Default for TokenBudget {
    fn default() -> Self {
        Self {
            total: 200_000,
            system: 10_000,
            conversation: 80_000,
            file_context: 60_000,
            tool_results: 40_000,
        }
    }
}

impl TokenBudget {
    /// Create a small budget for testing
    pub fn small() -> Self {
        Self {
            total: 8_000,
            system: 500,
            conversation: 4_000,
            file_context: 2_500,
            tool_results: 1_000,
        }
    }

    /// Total allocated tokens (may not equal `total` if some categories overlap)
    pub fn allocated(&self) -> usize {
        self.system + self.conversation + self.file_context + self.tool_results
    }

    /// Remaining tokens (total - allocated, or 0)
    pub fn remaining(&self) -> usize {
        self.total.saturating_sub(self.allocated())
    }

    /// Check if a text fits within the file_context budget
    pub fn fits_in_file_context(&self, text: &str) -> bool {
        estimate_tokens(text) <= self.file_context
    }
}

// ─── AssembledContext ──────────────────────────────────────────────────────────

/// The fully assembled context ready to send to an LLM
#[derive(Debug, Clone)]
pub struct AssembledContext {
    /// System prompt with project memory injected
    pub system_prompt: String,
    /// Session summary from working memory
    pub session_summary: String,
    /// File contents relevant to the task (path → content)
    pub file_contents: Vec<(String, String)>,
    /// Semantic search results for the task
    pub semantic_results: Vec<String>,
    /// Estimated token count for this context
    pub estimated_tokens: usize,
    /// Whether the budget was exceeded (some content was truncated)
    pub budget_exceeded: bool,
}

impl AssembledContext {
    /// Render the context as a single string for injection into the system message
    pub fn render_system_context(&self) -> String {
        let mut parts = Vec::new();

        if !self.session_summary.is_empty() {
            parts.push(format!("## Session Context\n{}", self.session_summary));
        }

        if !self.semantic_results.is_empty() {
            parts.push(format!(
                "## Relevant Code\n{}",
                self.semantic_results.join("\n---\n")
            ));
        }

        if !self.file_contents.is_empty() {
            let files = self
                .file_contents
                .iter()
                .map(|(path, content)| format!("### {}\n```\n{}\n```", path, content))
                .collect::<Vec<_>>()
                .join("\n\n");
            parts.push(format!("## File Contents\n{}", files));
        }

        if self.budget_exceeded {
            parts.push(
                "⚠️ Note: Some context was truncated due to token budget constraints.".to_string(),
            );
        }

        parts.join("\n\n")
    }
}

// ─── ContextAssembler ──────────────────────────────────────────────────────────

/// Assembles context from memory sources respecting token budgets
pub struct ContextAssembler {
    pub budget: TokenBudget,
    pub base_system_prompt: String,
}

impl ContextAssembler {
    /// Create with default budget and system prompt
    pub fn new(base_system_prompt: impl Into<String>) -> Self {
        Self {
            budget: TokenBudget::default(),
            base_system_prompt: base_system_prompt.into(),
        }
    }

    /// Create with a custom token budget
    pub fn with_budget(mut self, budget: TokenBudget) -> Self {
        self.budget = budget;
        self
    }

    /// Assemble context for a task query
    ///
    /// - `query`: the task description or user message
    /// - `working`: current session's working memory
    /// - `project`: optional SQLite project memory
    /// - `semantic`: optional semantic index for code search
    pub fn assemble(
        &self,
        query: &str,
        working: &WorkingMemory,
        project: Option<&ProjectMemory>,
        semantic: Option<&mut SemanticIndex>,
    ) -> AssembledContext {
        let mut budget_exceeded = false;
        let mut system_parts = vec![self.base_system_prompt.clone()];
        let mut estimated_tokens = estimate_tokens(&self.base_system_prompt);

        // 1. Inject project memory (architecture decisions, patterns)
        if let Some(pm) = project {
            let mut memory_text = String::new();
            if let Ok(entries) = pm.search_by_category("architecture") {
                for e in entries.iter().take(5) {
                    memory_text.push_str(&format!("- {}: {}\n", e.key, e.value));
                }
            }
            if let Ok(entries) = pm.search_by_category("convention") {
                for e in entries.iter().take(5) {
                    memory_text.push_str(&format!("- {}: {}\n", e.key, e.value));
                }
            }
            if !memory_text.is_empty() {
                let section = format!("## Project Knowledge\n{}", memory_text);
                let cost = estimate_tokens(&section);
                if estimated_tokens + cost <= self.budget.system {
                    system_parts.push(section);
                    estimated_tokens += cost;
                } else {
                    budget_exceeded = true;
                }
            }
        }

        // 2. Session summary from working memory
        let session_summary = working.session_summary();
        estimated_tokens += estimate_tokens(&session_summary);

        // 3. Semantic search results
        let mut semantic_results = Vec::new();
        if let Some(idx) = semantic {
            let results = idx.search(query, 5);
            let mut semantic_budget = 0usize;
            for r in &results {
                let cost = estimate_tokens(&r.text);
                if semantic_budget + cost <= self.budget.file_context / 2 {
                    semantic_results.push(format!(
                        "// {} (score: {:.2})\n{}",
                        r.id, r.score, r.text
                    ));
                    semantic_budget += cost;
                } else {
                    budget_exceeded = true;
                    break;
                }
            }
            estimated_tokens += semantic_budget;
        }

        // 4. File contents (most recently accessed files)
        let mut file_contents = Vec::new();
        let mut file_budget = 0usize;
        for path in working.recent_file_paths().iter().take(10) {
            if let Ok(content) = std::fs::read_to_string(path) {
                let cost = estimate_tokens(&content);
                if file_budget + cost <= self.budget.file_context {
                    file_contents.push((path.to_string(), content));
                    file_budget += cost;
                } else {
                    // Try truncated version
                    let max_chars = (self.budget.file_context - file_budget) * 4;
                    if max_chars > 100 {
                        let truncated = format!(
                            "{}... [truncated, {} chars omitted]",
                            &content[..content.len().min(max_chars)],
                            content.len().saturating_sub(max_chars)
                        );
                        file_contents.push((path.to_string(), truncated));
                        file_budget = self.budget.file_context;
                    }
                    budget_exceeded = true;
                    break;
                }
            }
        }
        estimated_tokens += file_budget;

        let system_prompt = system_parts.join("\n\n");

        AssembledContext {
            system_prompt,
            session_summary,
            file_contents,
            semantic_results,
            estimated_tokens,
            budget_exceeded,
        }
    }
}

impl Default for ContextAssembler {
    fn default() -> Self {
        Self::new(
            "You are zcode, a powerful AI coding agent. \
             Use the available tools to help the user with their coding tasks.",
        )
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{SemanticIndex, WorkingMemory};

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("abcd"), 1); // 4 chars = 1 token
        assert_eq!(estimate_tokens("hello world!"), 3); // 12 chars = 3 tokens
    }

    #[test]
    fn test_estimate_tokens_longer() {
        let text = "a".repeat(400);
        assert_eq!(estimate_tokens(&text), 100);
    }

    #[test]
    fn test_token_budget_default() {
        let b = TokenBudget::default();
        assert_eq!(b.total, 200_000);
        assert!(b.allocated() > 0);
        assert!(b.remaining() == b.total - b.allocated());
    }

    #[test]
    fn test_token_budget_small() {
        let b = TokenBudget::small();
        assert_eq!(b.total, 8_000);
    }

    #[test]
    fn test_token_budget_fits_in_file_context() {
        let b = TokenBudget::small();
        let short = "fn main() {}";
        let long = "x".repeat(b.file_context * 5);
        assert!(b.fits_in_file_context(short));
        assert!(!b.fits_in_file_context(&long));
    }

    #[test]
    fn test_assembled_context_render_empty() {
        let ctx = AssembledContext {
            system_prompt: "You are a bot".into(),
            session_summary: String::new(),
            file_contents: Vec::new(),
            semantic_results: Vec::new(),
            estimated_tokens: 5,
            budget_exceeded: false,
        };
        let rendered = ctx.render_system_context();
        assert!(rendered.is_empty() || !rendered.contains("Session Context"));
    }

    #[test]
    fn test_assembled_context_render_with_data() {
        let ctx = AssembledContext {
            system_prompt: "You are a bot".into(),
            session_summary: "Working on feature X".into(),
            file_contents: vec![("src/main.rs".into(), "fn main() {}".into())],
            semantic_results: vec!["// match\nfn related_func() {}".into()],
            estimated_tokens: 100,
            budget_exceeded: true,
        };
        let rendered = ctx.render_system_context();
        assert!(rendered.contains("Session Context"));
        assert!(rendered.contains("Relevant Code"));
        assert!(rendered.contains("File Contents"));
        assert!(rendered.contains("truncated"));
    }

    #[test]
    fn test_context_assembler_new() {
        let ca = ContextAssembler::new("My system prompt");
        assert_eq!(ca.base_system_prompt, "My system prompt");
    }

    #[test]
    fn test_context_assembler_assemble_empty() {
        let ca = ContextAssembler::default();
        let wm = WorkingMemory::new();
        let ctx = ca.assemble("fix the bug", &wm, None, None);
        assert!(!ctx.system_prompt.is_empty());
        assert!(ctx.file_contents.is_empty());
        assert!(ctx.semantic_results.is_empty());
        assert!(!ctx.budget_exceeded);
        assert!(ctx.estimated_tokens > 0);
    }

    #[test]
    fn test_context_assembler_with_semantic() {
        let ca = ContextAssembler::default();
        let wm = WorkingMemory::new();
        let mut idx = SemanticIndex::new();
        idx.index_chunk("src/main.rs:1-10", "fn main() { println!(\"hello\"); }");
        idx.index_chunk("src/lib.rs:1-5", "pub mod tools;");

        let ctx = ca.assemble("main function", &wm, None, Some(&mut idx));
        assert!(!ctx.semantic_results.is_empty());
    }

    #[test]
    fn test_context_assembler_with_project_memory() {
        let ca = ContextAssembler::default();
        let mut wm = WorkingMemory::new();
        wm.set_task("Fix the login bug");

        let pm = ProjectMemory::in_memory().unwrap();
        pm.store("arch/overview", "Layered tool-agent architecture", "architecture").unwrap();
        pm.store("conv/naming", "Use snake_case for all identifiers", "convention").unwrap();

        let ctx = ca.assemble("fix bug", &wm, Some(&pm), None);
        assert!(ctx.system_prompt.contains("Project Knowledge"));
        assert!(ctx.session_summary.contains("Fix the login bug"));
    }

    #[test]
    fn test_context_assembler_with_budget() {
        let ca = ContextAssembler::default()
            .with_budget(TokenBudget::small());
        assert_eq!(ca.budget.total, 8_000);
    }

    #[test]
    fn test_token_budget_remaining_zero_when_overallocated() {
        let b = TokenBudget {
            total: 100,
            system: 50,
            conversation: 60,
            file_context: 0,
            tool_results: 0,
        };
        assert_eq!(b.remaining(), 0); // saturating_sub
    }
}
