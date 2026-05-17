use super::*;
use crate::agent::graph::graph::GraphEvent;
use std::collections::VecDeque;
use std::sync::Mutex;
use zcode_core::agent::{DefaultState, Task};
use zcode_core::llm::{LlmResponse as ProviderLlmResponse, UsageStats};
use zcode_llm_provider::Message;

struct ScriptedProvider {
    responses: Mutex<VecDeque<String>>,
}

impl ScriptedProvider {
    fn new(responses: impl IntoIterator<Item = &'static str>) -> Self {
        Self {
            responses: Mutex::new(responses.into_iter().map(String::from).collect()),
        }
    }
}

impl LlmProvider for ScriptedProvider {
    fn complete(&self, _prompt: &str) -> Result<String> {
        Ok(self
            .responses
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| "PASS".to_string()))
    }

    fn chat(
        &self,
        _messages: &[Message],
        _tools: &[serde_json::Value],
    ) -> Result<ProviderLlmResponse> {
        let content = self
            .responses
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| "PASS".to_string());
        Ok(ProviderLlmResponse {
            content: content.clone(),
            model: "scripted".to_string(),
            usage: Some(UsageStats {
                input_tokens: 1,
                output_tokens: 1,
            }),
            raw_response: serde_json::json!({ "content": content }),
        })
    }

    fn stream_complete(&self, _prompt: &str) -> Result<zcode_llm_provider::StreamingResponse> {
        Err(zcode_core::ZcodeError::InternalError(
            "streaming is not used by pipeline tests".to_string(),
        ))
    }
}

fn empty_registry() -> Arc<ToolRegistry> {
    Arc::new(ToolRegistry::new())
}

fn test_runtimes(provider: Arc<dyn LlmProvider>) -> TaskAgentRuntimes {
    TaskAgentRuntimes {
        supervisor: AgentRuntime::new(Arc::clone(&provider), "model".to_string(), false),
        investigator: AgentRuntime::new(Arc::clone(&provider), "model".to_string(), false),
        planner: AgentRuntime::new(Arc::clone(&provider), "model".to_string(), false),
        coder: AgentRuntime::new(Arc::clone(&provider), "model".to_string(), false),
        reviewer: AgentRuntime::new(Arc::clone(&provider), "model".to_string(), false),
        fast: AgentRuntime::new(Arc::clone(&provider), "fast-model".to_string(), false),
    }
}

#[tokio::test]
async fn test_task_pipeline_starts_with_orchestrator_and_reviewer_gate() {
    let provider: Arc<dyn LlmProvider> = Arc::new(ScriptedProvider::new([
        r#"{"action":"continue_task","next_agent":"planner","step_title":"Plan the implementation","reason":"need a plan"}"#,
        "PLAN: do it",
        r#"{"action":"continue_task","next_agent":"coder","step_title":"Apply the changes","reason":"execute the plan"}"#,
        "CODER: done",
        r#"{"action":"continue_task","next_agent":"reviewer","step_title":"Verify the result","reason":"review the work"}"#,
        "PASS",
        r#"{"action":"finish","next_agent":null,"step_title":null,"reason":"review passed"}"#,
    ]));
    let graph = build_task_pipeline_with_limit(
        test_runtimes(provider),
        empty_registry(),
        String::new(),
        20,
    )
    .compile()
    .unwrap();

    let mut state = DefaultState::new(Task::new("implement task"));
    let output = graph.execute(&mut state).await.unwrap();

    assert_eq!(
        output.nodes_executed,
        vec![
            "supervisor",
            "execute_step",
            "supervisor",
            "execute_step",
            "supervisor",
            "execute_step",
            "supervisor"
        ]
    );
    assert_eq!(
        state.metadata.get("root_agent").and_then(|v| v.as_str()),
        Some("supervisor")
    );
    assert_eq!(
        state
            .metadata
            .get("review_passed")
            .and_then(|v| v.as_bool()),
        Some(true)
    );
}

#[tokio::test]
async fn test_task_pipeline_retries_coder_after_reviewer_failure() {
    let provider: Arc<dyn LlmProvider> = Arc::new(ScriptedProvider::new([
        r#"{"action":"continue_task","next_agent":"planner","step_title":"Plan the implementation","reason":"need a plan"}"#,
        "PLAN: do it",
        r#"{"action":"continue_task","next_agent":"coder","step_title":"Apply the changes","reason":"execute the plan"}"#,
        "CODER: first attempt",
        r#"{"action":"continue_task","next_agent":"reviewer","step_title":"Verify the result","reason":"review the work"}"#,
        "FAIL: missing behavior",
        r#"{"action":"continue_task","next_agent":"coder","step_title":"Fix review findings","reason":"address reviewer feedback"}"#,
        "CODER: fixed",
        r#"{"action":"continue_task","next_agent":"reviewer","step_title":"Verify the result","reason":"review the fix"}"#,
        "PASS",
        r#"{"action":"finish","next_agent":null,"step_title":null,"reason":"review passed"}"#,
    ]));
    let graph = build_task_pipeline_with_limit(
        test_runtimes(provider),
        empty_registry(),
        String::new(),
        20,
    )
    .compile()
    .unwrap();

    let mut state = DefaultState::new(Task::new("implement task"));
    let output = graph.execute(&mut state).await.unwrap();

    assert_eq!(
        output.nodes_executed,
        vec![
            "supervisor",
            "execute_step",
            "supervisor",
            "execute_step",
            "supervisor",
            "execute_step",
            "supervisor",
            "execute_step",
            "supervisor",
            "execute_step",
            "supervisor"
        ]
    );
    assert_eq!(
        state.metadata.get("coder_retries").and_then(|v| v.as_u64()),
        Some(2)
    );
    assert_eq!(
        state
            .metadata
            .get("review_passed")
            .and_then(|v| v.as_bool()),
        Some(true)
    );
}

#[tokio::test]
async fn test_task_pipeline_emits_step_events() {
    let provider: Arc<dyn LlmProvider> = Arc::new(ScriptedProvider::new([
        r#"{"action":"continue_task","next_agent":"planner","step_title":"Plan the implementation","reason":"need a plan"}"#,
        "PLAN: do it",
        r#"{"action":"continue_task","next_agent":"coder","step_title":"Apply the changes","reason":"execute the plan"}"#,
        "CODER: done",
        r#"{"action":"continue_task","next_agent":"reviewer","step_title":"Verify the result","reason":"review the work"}"#,
        "PASS",
        r#"{"action":"finish","next_agent":null,"step_title":null,"reason":"review passed"}"#,
    ]));
    let graph = build_task_pipeline_with_limit(
        test_runtimes(provider),
        empty_registry(),
        String::new(),
        20,
    )
    .compile()
    .unwrap();

    let mut state = DefaultState::new(Task::new("implement task"));
    let mut events = Vec::new();
    graph
        .execute_with_events(&mut state, |event| events.push(event))
        .await
        .unwrap();

    assert!(events.iter().any(|event| matches!(
        event,
        GraphEvent::StepStart { agent, title, .. }
            if agent == "planner" && title == "Plan the implementation"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        GraphEvent::StepComplete { agent, title, success, .. }
            if agent == "reviewer" && title == "Verify the result" && *success
    )));
}

#[tokio::test]
async fn test_task_pipeline_can_finish_read_only_after_investigator() {
    let provider: Arc<dyn LlmProvider> = Arc::new(ScriptedProvider::new([
        r#"{"action":"continue_task","next_agent":"investigator","step_title":"List directory files","reason":"read-only question"}"#,
        "README.md\nCargo.toml\ncrates/",
        r#"{"action":"finish","next_agent":null,"step_title":null,"reason":"answer is sufficient"}"#,
    ]));
    let graph = build_task_pipeline_with_limit(
        test_runtimes(provider),
        empty_registry(),
        String::new(),
        20,
    )
    .compile()
    .unwrap();

    let mut state = DefaultState::new(Task::new("当前目录下有哪些文件"));
    let output = graph.execute(&mut state).await.unwrap();

    assert_eq!(
        output.nodes_executed,
        vec!["supervisor", "execute_step", "supervisor"]
    );
    assert!(state.messages.iter().any(|message| message
        .content
        .as_deref()
        .is_some_and(|content| content.starts_with("INVESTIGATOR_REPORT:"))));
    assert!(state.messages.iter().all(|message| message
        .content
        .as_deref()
        .map(|content| !content.starts_with("REVIEWER_REPORT:"))
        .unwrap_or(true)));
}

#[tokio::test]
async fn test_task_pipeline_falls_back_when_supervisor_json_is_invalid() {
    let provider: Arc<dyn LlmProvider> = Arc::new(ScriptedProvider::new([
        "not json",
        "PLAN: fallback plan",
        r#"{"action":"continue_task","next_agent":"coder","step_title":"Apply the changes","reason":"execute the plan"}"#,
        "CODER: done",
        r#"{"action":"continue_task","next_agent":"reviewer","step_title":"Verify the result","reason":"review the work"}"#,
        "PASS",
        r#"{"action":"finish","next_agent":null,"step_title":null,"reason":"review passed"}"#,
    ]));
    let graph = build_task_pipeline_with_limit(
        test_runtimes(provider),
        empty_registry(),
        String::new(),
        20,
    )
    .compile()
    .unwrap();

    let mut state = DefaultState::new(Task::new("implement task"));
    let output = graph.execute(&mut state).await.unwrap();

    assert_eq!(
        output.nodes_executed,
        vec![
            "supervisor",
            "execute_step",
            "supervisor",
            "execute_step",
            "supervisor",
            "execute_step",
            "supervisor"
        ]
    );
    let plan = load_task_plan(&state).unwrap();
    assert_eq!(
        plan.steps.first().map(|step| step.agent),
        Some(StepAgent::Planner)
    );
    assert_eq!(
        plan.steps.first().map(|step| step.title.as_str()),
        Some("Plan the implementation")
    );
}
