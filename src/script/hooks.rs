//! Hook system for lifecycle interception
//!
//! Allows scripts to intercept Agent and tool lifecycle events.

use serde_json::Value;
use std::fmt;

// ─── HookType ──────────────────────────────────────────────────────────────────

/// Events that can be intercepted via hooks
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HookType {
    /// Before a tool is called
    BeforeToolCall,
    /// After a tool returns
    AfterToolCall,
    /// When a task starts
    OnTaskStart,
    /// When a task completes (success or failure)
    OnTaskComplete,
    /// When an error occurs
    OnError,
}

impl fmt::Display for HookType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HookType::BeforeToolCall  => write!(f, "before_tool_call"),
            HookType::AfterToolCall   => write!(f, "after_tool_call"),
            HookType::OnTaskStart     => write!(f, "on_task_start"),
            HookType::OnTaskComplete  => write!(f, "on_task_complete"),
            HookType::OnError         => write!(f, "on_error"),
        }
    }
}

// ─── HookContext ───────────────────────────────────────────────────────────────

/// Context passed to a hook handler
#[derive(Debug, Clone, Default)]
pub struct HookContext {
    /// Tool name (for tool-related hooks)
    pub tool_name: Option<String>,
    /// Task description (for task-related hooks)
    pub task_description: Option<String>,
    /// Input arguments or task parameters
    pub args: Option<Value>,
    /// Tool result or task output (available in After hooks)
    pub result: Option<Value>,
    /// Error message (available in OnError or failed AfterToolCall)
    pub error: Option<String>,
    /// Whether the hook should abort the operation
    pub abort: bool,
}

impl HookContext {
    pub fn for_tool_call(tool_name: &str, args: Value) -> Self {
        Self {
            tool_name: Some(tool_name.to_string()),
            args: Some(args),
            ..Default::default()
        }
    }

    pub fn for_task_start(description: &str) -> Self {
        Self {
            task_description: Some(description.to_string()),
            ..Default::default()
        }
    }

    pub fn for_task_complete(description: &str, result: Value) -> Self {
        Self {
            task_description: Some(description.to_string()),
            result: Some(result),
            ..Default::default()
        }
    }

    pub fn for_error(error: impl Into<String>) -> Self {
        Self {
            error: Some(error.into()),
            ..Default::default()
        }
    }
}

// ─── HookHandler ──────────────────────────────────────────────────────────────

/// A hook handler: a boxed function that receives and may modify the context
pub type HookHandler = Box<dyn Fn(&mut HookContext) + Send + Sync>;

// ─── HookRegistry ─────────────────────────────────────────────────────────────

/// Registry for lifecycle hooks
pub struct HookRegistry {
    hooks: std::collections::HashMap<HookType, Vec<(String, HookHandler)>>,
}

impl HookRegistry {
    pub fn new() -> Self {
        Self {
            hooks: std::collections::HashMap::new(),
        }
    }

    /// Register a hook handler for a given event type
    ///
    /// `name` is a friendly label for the hook (useful for debugging)
    pub fn register(
        &mut self,
        hook_type: HookType,
        name: impl Into<String>,
        handler: HookHandler,
    ) {
        self.hooks
            .entry(hook_type)
            .or_default()
            .push((name.into(), handler));
    }

    /// Register a simple closure hook
    pub fn on<F: Fn(&mut HookContext) + Send + Sync + 'static>(
        &mut self,
        hook_type: HookType,
        name: impl Into<String>,
        f: F,
    ) {
        self.register(hook_type, name, Box::new(f));
    }

    /// Trigger all hooks for a given type, passing context through each
    ///
    /// Returns the (potentially modified) context after all handlers run.
    /// If any handler sets `ctx.abort = true`, subsequent handlers are skipped.
    pub fn trigger(&self, hook_type: &HookType, mut ctx: HookContext) -> HookContext {
        if let Some(handlers) = self.hooks.get(hook_type) {
            for (_, handler) in handlers {
                handler(&mut ctx);
                if ctx.abort {
                    break;
                }
            }
        }
        ctx
    }

    /// Count of registered hooks for a type
    pub fn count(&self, hook_type: &HookType) -> usize {
        self.hooks.get(hook_type).map(|v| v.len()).unwrap_or(0)
    }

    /// Total hook count across all types
    pub fn total_count(&self) -> usize {
        self.hooks.values().map(|v| v.len()).sum()
    }

    /// Remove all hooks for a given type
    pub fn clear(&mut self, hook_type: &HookType) {
        self.hooks.remove(hook_type);
    }

    /// Remove all hooks
    pub fn clear_all(&mut self) {
        self.hooks.clear();
    }
}

impl Default for HookRegistry {
    fn default() -> Self { Self::new() }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_hook_type_display() {
        assert_eq!(HookType::BeforeToolCall.to_string(), "before_tool_call");
        assert_eq!(HookType::OnError.to_string(), "on_error");
    }

    #[test]
    fn test_hook_context_for_tool_call() {
        let ctx = HookContext::for_tool_call("file_read", serde_json::json!({"path": "a.rs"}));
        assert_eq!(ctx.tool_name.unwrap(), "file_read");
        assert_eq!(ctx.args.unwrap()["path"], "a.rs");
        assert!(!ctx.abort);
    }

    #[test]
    fn test_hook_context_for_task_start() {
        let ctx = HookContext::for_task_start("Fix the bug");
        assert_eq!(ctx.task_description.unwrap(), "Fix the bug");
    }

    #[test]
    fn test_hook_context_for_error() {
        let ctx = HookContext::for_error("Something went wrong");
        assert_eq!(ctx.error.unwrap(), "Something went wrong");
    }

    #[test]
    fn test_registry_register_and_count() {
        let mut reg = HookRegistry::new();
        reg.on(HookType::BeforeToolCall, "test_hook", |_ctx| {});
        assert_eq!(reg.count(&HookType::BeforeToolCall), 1);
        assert_eq!(reg.total_count(), 1);
    }

    #[test]
    fn test_registry_trigger_modifies_context() {
        let mut reg = HookRegistry::new();
        reg.on(HookType::BeforeToolCall, "add_result", |ctx| {
            ctx.result = Some(serde_json::json!({ "modified": true }));
        });

        let ctx = HookContext::for_tool_call("shell", serde_json::json!({}));
        let out = reg.trigger(&HookType::BeforeToolCall, ctx);
        assert!(out.result.unwrap()["modified"].as_bool().unwrap());
    }

    #[test]
    fn test_registry_abort_stops_chain() {
        let call_count = Arc::new(Mutex::new(0usize));
        let cc = Arc::clone(&call_count);

        let mut reg = HookRegistry::new();
        reg.on(HookType::BeforeToolCall, "aborter", |ctx| {
            ctx.abort = true;
        });
        reg.on(HookType::BeforeToolCall, "should_not_run", move |_ctx| {
            *cc.lock().unwrap() += 1;
        });

        let ctx = HookContext::default();
        let out = reg.trigger(&HookType::BeforeToolCall, ctx);
        assert!(out.abort);
        assert_eq!(*call_count.lock().unwrap(), 0); // second handler was skipped
    }

    #[test]
    fn test_registry_multiple_hooks_same_type() {
        let log = Arc::new(Mutex::new(Vec::<String>::new()));
        let l1 = Arc::clone(&log);
        let l2 = Arc::clone(&log);

        let mut reg = HookRegistry::new();
        reg.on(HookType::OnTaskStart, "first", move |_| {
            l1.lock().unwrap().push("first".to_string());
        });
        reg.on(HookType::OnTaskStart, "second", move |_| {
            l2.lock().unwrap().push("second".to_string());
        });

        let ctx = HookContext::default();
        reg.trigger(&HookType::OnTaskStart, ctx);

        let log = log.lock().unwrap();
        assert_eq!(*log, vec!["first", "second"]);
    }

    #[test]
    fn test_registry_no_hooks_for_type() {
        let reg = HookRegistry::new();
        let ctx = HookContext::for_task_start("task");
        let out = reg.trigger(&HookType::OnTaskComplete, ctx);
        assert!(out.task_description.as_deref() == Some("task")); // unchanged
    }

    #[test]
    fn test_registry_clear() {
        let mut reg = HookRegistry::new();
        reg.on(HookType::OnError, "h1", |_| {});
        reg.on(HookType::OnTaskStart, "h2", |_| {});
        reg.clear(&HookType::OnError);
        assert_eq!(reg.count(&HookType::OnError), 0);
        assert_eq!(reg.count(&HookType::OnTaskStart), 1);
    }

    #[test]
    fn test_registry_clear_all() {
        let mut reg = HookRegistry::new();
        reg.on(HookType::OnError, "h1", |_| {});
        reg.on(HookType::BeforeToolCall, "h2", |_| {});
        reg.clear_all();
        assert_eq!(reg.total_count(), 0);
    }

    #[test]
    fn test_hook_context_abort_default_false() {
        let ctx = HookContext::default();
        assert!(!ctx.abort);
    }
}
