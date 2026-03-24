//! Working Memory — session-scoped in-memory store
//!
//! Tracks recent files, tool executions, and token usage for the current session.

use std::collections::VecDeque;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// ─── Helpers ───────────────────────────────────────────────────────────────────

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
}

// ─── RecentFile ────────────────────────────────────────────────────────────────

/// A file that was recently read or written by the agent
#[derive(Debug, Clone)]
pub struct RecentFile {
    /// Absolute or project-relative path
    pub path: String,
    /// Simple hash of the content (len for now, can be sha256 later)
    pub content_hash: u64,
    /// Unix timestamp of last access
    pub last_read: u64,
    /// Number of times accessed this session
    pub access_count: usize,
}

impl RecentFile {
    pub fn new(path: impl Into<String>, content_hash: u64) -> Self {
        Self {
            path: path.into(),
            content_hash,
            last_read: now_secs(),
            access_count: 1,
        }
    }

    pub fn touch(&mut self, new_hash: u64) {
        self.content_hash = new_hash;
        self.last_read = now_secs();
        self.access_count += 1;
    }
}

// ─── ToolExecution ─────────────────────────────────────────────────────────────

/// A record of a single tool call during this session
#[derive(Debug, Clone)]
pub struct ToolExecution {
    pub tool_name: String,
    pub input_summary: String,
    pub output_summary: String,
    pub success: bool,
    pub timestamp: u64,
    pub duration_ms: u64,
}

impl ToolExecution {
    pub fn new(
        tool_name: impl Into<String>,
        input_summary: impl Into<String>,
        output_summary: impl Into<String>,
        success: bool,
        duration_ms: u64,
    ) -> Self {
        Self {
            tool_name: tool_name.into(),
            input_summary: input_summary.into(),
            output_summary: output_summary.into(),
            success,
            timestamp: now_secs(),
            duration_ms,
        }
    }
}

// ─── TokenUsage ────────────────────────────────────────────────────────────────

/// Cumulative token usage for this session
#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
    pub llm_calls: usize,
}

impl TokenUsage {
    pub fn add(&mut self, prompt: usize, completion: usize) {
        self.prompt_tokens += prompt;
        self.completion_tokens += completion;
        self.total_tokens += prompt + completion;
        self.llm_calls += 1;
    }
}

// ─── WorkingMemory ─────────────────────────────────────────────────────────────

/// Session-scoped in-memory working memory
pub struct WorkingMemory {
    /// Maximum number of recent files to track (LRU)
    max_files: usize,
    /// Maximum number of tool executions to keep
    max_tool_history: usize,
    /// Recent files accessed this session
    recent_files: VecDeque<RecentFile>,
    /// Tool execution history (most recent last)
    tool_history: VecDeque<ToolExecution>,
    /// Cumulative token usage
    pub token_usage: TokenUsage,
    /// Current task description
    pub current_task: Option<String>,
    /// Files modified during this session
    modified_files: Vec<String>,
}

impl WorkingMemory {
    /// Create a new working memory with default capacities
    pub fn new() -> Self {
        Self::with_capacity(50, 200)
    }

    /// Create a new working memory with custom capacities
    pub fn with_capacity(max_files: usize, max_tool_history: usize) -> Self {
        Self {
            max_files,
            max_tool_history,
            recent_files: VecDeque::new(),
            tool_history: VecDeque::new(),
            token_usage: TokenUsage::default(),
            current_task: None,
            modified_files: Vec::new(),
        }
    }

    // ── File tracking ──

    /// Record a file access (LRU eviction when at capacity)
    pub fn record_file_access(&mut self, path: impl Into<String>, content_hash: u64) {
        let path = path.into();
        // Update existing entry if present
        for file in self.recent_files.iter_mut() {
            if file.path == path {
                file.touch(content_hash);
                return;
            }
        }
        // Evict oldest if at capacity
        if self.recent_files.len() >= self.max_files {
            self.recent_files.pop_front();
        }
        self.recent_files.push_back(RecentFile::new(path, content_hash));
    }

    /// Record a file write/edit
    pub fn record_file_write(&mut self, path: impl Into<String>) {
        let path = path.into();
        self.record_file_access(&path, 0);
        if !self.modified_files.contains(&path) {
            self.modified_files.push(path);
        }
    }

    /// Get the most recently accessed file paths
    pub fn recent_file_paths(&self) -> Vec<&str> {
        self.recent_files
            .iter()
            .rev()
            .map(|f| f.path.as_str())
            .collect()
    }

    /// Get all files modified this session
    pub fn modified_files(&self) -> &[String] {
        &self.modified_files
    }

    /// Look up a recently accessed file by path
    pub fn get_recent_file(&self, path: &str) -> Option<&RecentFile> {
        self.recent_files.iter().find(|f| f.path == path)
    }

    /// Current capacity for recent files
    pub fn file_count(&self) -> usize {
        self.recent_files.len()
    }

    // ── Tool history ──

    /// Record a tool execution
    pub fn record_tool_execution(&mut self, execution: ToolExecution) {
        if self.tool_history.len() >= self.max_tool_history {
            self.tool_history.pop_front();
        }
        self.tool_history.push_back(execution);
    }

    /// Get the N most recent tool executions
    pub fn recent_tool_executions(&self, n: usize) -> Vec<&ToolExecution> {
        self.tool_history.iter().rev().take(n).collect()
    }

    /// Number of tool calls recorded
    pub fn tool_call_count(&self) -> usize {
        self.tool_history.len()
    }

    /// Count of successful tool calls
    pub fn successful_tool_calls(&self) -> usize {
        self.tool_history.iter().filter(|t| t.success).count()
    }

    // ── Token usage ──

    /// Record token usage for an LLM call
    pub fn record_tokens(&mut self, prompt: usize, completion: usize) {
        self.token_usage.add(prompt, completion);
    }

    // ── Task management ──

    /// Set the current task description
    pub fn set_task(&mut self, task: impl Into<String>) {
        self.current_task = Some(task.into());
    }

    /// Clear the current task
    pub fn clear_task(&mut self) {
        self.current_task = None;
    }

    // ── Summary ──

    /// Produce a text summary of the current session for inclusion in prompts
    pub fn session_summary(&self) -> String {
        let mut parts = Vec::new();

        if let Some(task) = &self.current_task {
            parts.push(format!("Current task: {}", task));
        }

        if !self.recent_files.is_empty() {
            let files: Vec<&str> = self
                .recent_files
                .iter()
                .rev()
                .take(5)
                .map(|f| f.path.as_str())
                .collect();
            parts.push(format!("Recent files: {}", files.join(", ")));
        }

        if !self.modified_files.is_empty() {
            parts.push(format!("Modified: {}", self.modified_files.join(", ")));
        }

        parts.push(format!(
            "Tokens used: {} (in {} call(s))",
            self.token_usage.total_tokens,
            self.token_usage.llm_calls
        ));

        parts.join("\n")
    }

    /// Reset the working memory (start fresh session)
    pub fn reset(&mut self) {
        self.recent_files.clear();
        self.tool_history.clear();
        self.token_usage = TokenUsage::default();
        self.current_task = None;
        self.modified_files.clear();
    }
}

impl Default for WorkingMemory {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_working_memory_new() {
        let wm = WorkingMemory::new();
        assert_eq!(wm.file_count(), 0);
        assert_eq!(wm.tool_call_count(), 0);
        assert!(wm.current_task.is_none());
    }

    #[test]
    fn test_record_file_access() {
        let mut wm = WorkingMemory::new();
        wm.record_file_access("src/main.rs", 42);
        assert_eq!(wm.file_count(), 1);
        assert_eq!(wm.recent_file_paths(), vec!["src/main.rs"]);
    }

    #[test]
    fn test_record_file_access_updates_existing() {
        let mut wm = WorkingMemory::new();
        wm.record_file_access("src/lib.rs", 10);
        wm.record_file_access("src/lib.rs", 20);
        // Should not duplicate
        assert_eq!(wm.file_count(), 1);
        assert_eq!(wm.get_recent_file("src/lib.rs").unwrap().content_hash, 20);
        assert_eq!(wm.get_recent_file("src/lib.rs").unwrap().access_count, 2);
    }

    #[test]
    fn test_lru_eviction() {
        let mut wm = WorkingMemory::with_capacity(3, 100);
        wm.record_file_access("a.rs", 1);
        wm.record_file_access("b.rs", 2);
        wm.record_file_access("c.rs", 3);
        wm.record_file_access("d.rs", 4); // should evict "a.rs"
        assert_eq!(wm.file_count(), 3);
        assert!(wm.get_recent_file("a.rs").is_none());
        assert!(wm.get_recent_file("b.rs").is_some());
    }

    #[test]
    fn test_record_file_write() {
        let mut wm = WorkingMemory::new();
        wm.record_file_write("src/main.rs");
        assert!(wm.modified_files().contains(&"src/main.rs".to_string()));
        // Should not duplicate
        wm.record_file_write("src/main.rs");
        assert_eq!(wm.modified_files().len(), 1);
    }

    #[test]
    fn test_record_tool_execution() {
        let mut wm = WorkingMemory::new();
        let exec = ToolExecution::new("file_read", "src/main.rs", "fn main()", true, 10);
        wm.record_tool_execution(exec);
        assert_eq!(wm.tool_call_count(), 1);
        assert_eq!(wm.successful_tool_calls(), 1);
    }

    #[test]
    fn test_tool_history_lru() {
        let mut wm = WorkingMemory::with_capacity(100, 3);
        for i in 0..5 {
            wm.record_tool_execution(ToolExecution::new(
                format!("tool_{}", i), "", "", true, 0
            ));
        }
        assert_eq!(wm.tool_call_count(), 3); // LRU eviction
    }

    #[test]
    fn test_recent_tool_executions() {
        let mut wm = WorkingMemory::new();
        for i in 0..5 {
            wm.record_tool_execution(ToolExecution::new(
                format!("tool_{}", i), "", "", i % 2 == 0, 0
            ));
        }
        let recent = wm.recent_tool_executions(3);
        assert_eq!(recent.len(), 3);
        // Most recent first
        assert_eq!(recent[0].tool_name, "tool_4");
    }

    #[test]
    fn test_token_usage() {
        let mut wm = WorkingMemory::new();
        wm.record_tokens(1000, 500);
        wm.record_tokens(2000, 800);
        assert_eq!(wm.token_usage.total_tokens, 4300);
        assert_eq!(wm.token_usage.llm_calls, 2);
    }

    #[test]
    fn test_set_task() {
        let mut wm = WorkingMemory::new();
        wm.set_task("Implement feature X");
        assert_eq!(wm.current_task, Some("Implement feature X".to_string()));
        wm.clear_task();
        assert!(wm.current_task.is_none());
    }

    #[test]
    fn test_session_summary_empty() {
        let wm = WorkingMemory::new();
        let summary = wm.session_summary();
        assert!(summary.contains("Tokens used: 0"));
    }

    #[test]
    fn test_session_summary_with_data() {
        let mut wm = WorkingMemory::new();
        wm.set_task("Fix the bug");
        wm.record_file_access("src/main.rs", 1);
        wm.record_file_write("src/lib.rs");
        wm.record_tokens(100, 50);
        let summary = wm.session_summary();
        assert!(summary.contains("Current task: Fix the bug"));
        assert!(summary.contains("Recent files:"));
        assert!(summary.contains("Modified:"));
        assert!(summary.contains("Tokens used: 150"));
    }

    #[test]
    fn test_reset() {
        let mut wm = WorkingMemory::new();
        wm.record_file_access("x.rs", 1);
        wm.set_task("task");
        wm.record_tokens(100, 50);
        wm.reset();
        assert_eq!(wm.file_count(), 0);
        assert!(wm.current_task.is_none());
        assert_eq!(wm.token_usage.total_tokens, 0);
    }

    #[test]
    fn test_default() {
        let wm = WorkingMemory::default();
        assert_eq!(wm.file_count(), 0);
    }

    #[test]
    fn test_recent_file_paths_order() {
        let mut wm = WorkingMemory::new();
        wm.record_file_access("first.rs", 1);
        wm.record_file_access("second.rs", 2);
        wm.record_file_access("third.rs", 3);
        let paths = wm.recent_file_paths();
        // Most recent first
        assert_eq!(paths[0], "third.rs");
        assert_eq!(paths[2], "first.rs");
    }
}
