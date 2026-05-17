//! Comprehensive LangGraph pipelines for Zcode
//!
//! Two pipelines are provided:
//!
//! - `build_task_pipeline()`: per-task graph
//!   (`supervisor → execute_step → supervisor ...`).
//!   The supervisor asks the LLM which agent should handle the next step, then
//!   starts exactly one investigator/planner/coder/reviewer step. Guardrails
//!   reject unsafe decisions such as reviewing before code exists and fall back
//!   to a conservative default when the supervisor response is invalid.
//!
//! - `build_reviewer_pipeline()`: global review graph run **once** after all
//!   tasks have completed.  The reviewer receives a combined report of all
//!   task outputs via the initial state messages.

use crate::agent::graph::edge::routers;
use crate::agent::graph::graph::{GraphEvent, StateGraph};
use crate::agent::graph::llm_bridge::call_llm;
use crate::agent::graph::node::AsyncFnNode;
use crate::agent::graph::state::NodeOutput;
use crate::agent::graph::task_supervisor::{
    complete_current_step, completed_step_summaries, decide_supervisor_next, load_task_plan,
    next_agent_output, sanitize_step_title, supervisor_calls, supervisor_calls_output,
    task_plan_output, StepAgent, StepStatus, SupervisorAction, TaskPlan, TaskStep,
};
use crate::agent::loop_exec::{AgentLoop, ConversationMessage, LoopConfig, LoopEvent};
use crate::agent::self_learning::SelfLearningAgent;
use crate::agent::types::AgentState;
use std::sync::Arc;
use zcode_capabilities::ToolRegistry;
use zcode_core::Result;
use zcode_llm_provider::provider::LlmProvider;

#[derive(Clone)]
pub struct AgentRuntime {
    pub provider: Arc<dyn LlmProvider>,
    pub model: String,
    pub explicit_model: bool,
    pub event_sink: Option<Arc<dyn Fn(GraphEvent) + Send + Sync>>,
}

impl AgentRuntime {
    pub fn new(provider: Arc<dyn LlmProvider>, model: String, explicit_model: bool) -> Self {
        Self {
            provider,
            model,
            explicit_model,
            event_sink: None,
        }
    }

    pub fn with_event_sink(mut self, event_sink: Arc<dyn Fn(GraphEvent) + Send + Sync>) -> Self {
        self.event_sink = Some(event_sink);
        self
    }
}

#[derive(Clone)]
pub struct TaskAgentRuntimes {
    pub supervisor: AgentRuntime,
    pub investigator: AgentRuntime,
    pub planner: AgentRuntime,
    pub coder: AgentRuntime,
    pub reviewer: AgentRuntime,
    pub fast: AgentRuntime,
}

impl TaskAgentRuntimes {
    pub fn map_event_sink(mut self, event_sink: Arc<dyn Fn(GraphEvent) + Send + Sync>) -> Self {
        self.supervisor = self.supervisor.with_event_sink(Arc::clone(&event_sink));
        self.investigator = self.investigator.with_event_sink(Arc::clone(&event_sink));
        self.planner = self.planner.with_event_sink(Arc::clone(&event_sink));
        self.coder = self.coder.with_event_sink(Arc::clone(&event_sink));
        self.reviewer = self.reviewer.with_event_sink(event_sink);
        self
    }
}

#[derive(Debug, Clone)]
pub struct AgentModelLabels {
    pub supervisor: String,
    pub investigator: String,
    pub planner: String,
    pub coder: String,
    pub reviewer: String,
    pub fast: String,
}

fn current_task(task: &Option<zcode_core::agent::Task>) -> String {
    task.clone()
        .map(|task| task.description)
        .unwrap_or_default()
}

fn step_events_output(events: Vec<serde_json::Value>) -> Option<NodeOutput> {
    if events.is_empty() {
        None
    } else {
        Some(NodeOutput::Custom(
            "__step_events".to_string(),
            serde_json::Value::Array(events),
        ))
    }
}

fn step_start_event(step: &TaskStep) -> serde_json::Value {
    serde_json::json!({
        "kind": "step_start",
        "id": step.id,
        "title": step.title,
        "agent": step.agent.as_str()
    })
}

fn step_complete_event(step: &TaskStep, success: bool) -> serde_json::Value {
    serde_json::json!({
        "kind": "step_complete",
        "id": step.id,
        "title": step.title,
        "agent": step.agent.as_str(),
        "success": success
    })
}

fn state_agent_output(
    plan: &mut TaskPlan,
    step: &TaskStep,
    result: LoopResultForStep,
    success: bool,
    mut extra_outputs: Vec<NodeOutput>,
) -> Result<NodeOutput> {
    let summary = result.answer.clone();
    complete_current_step(plan, step, summary, success);
    let mut outputs = vec![
        task_plan_output(plan),
        NodeOutput::Messages(vec![ConversationMessage::assistant_text(format!(
            "{}_REPORT:\n{}",
            step.agent.as_str().to_uppercase(),
            result.answer
        ))]),
    ];
    if let Some(tool_events) = tool_events_output(result.tool_events) {
        outputs.push(tool_events);
    }
    if let Some(events) = step_events_output(vec![step_complete_event(step, success)]) {
        outputs.push(events);
    }
    outputs.append(&mut extra_outputs);
    outputs.push(next_agent_output(plan));
    Ok(NodeOutput::Multiple(outputs))
}

fn state_agent_output_with_review_retry(
    plan: &mut TaskPlan,
    step: &TaskStep,
    result: LoopResultForStep,
    success: bool,
    retries: u64,
    mut extra_outputs: Vec<NodeOutput>,
) -> Result<NodeOutput> {
    let allow_retry = !success && retries < 3;
    let summary = result.answer.clone();
    if allow_retry {
        complete_current_step(plan, step, summary, false);
        plan.current_index = plan.steps.len();
    } else {
        complete_current_step(plan, step, summary, success);
        if !success {
            plan.current_index = plan.steps.len();
        }
    }
    let mut outputs = vec![
        task_plan_output(plan),
        NodeOutput::Messages(vec![ConversationMessage::assistant_text(format!(
            "{}_REPORT:\n{}",
            step.agent.as_str().to_uppercase(),
            result.answer
        ))]),
    ];
    if let Some(tool_events) = tool_events_output(result.tool_events) {
        outputs.push(tool_events);
    }
    if let Some(events) = step_events_output(vec![step_complete_event(step, success)]) {
        outputs.push(events);
    }
    outputs.append(&mut extra_outputs);
    outputs.push(next_agent_output(plan));
    Ok(NodeOutput::Multiple(outputs))
}

struct LoopResultForStep {
    answer: String,
    tool_events: Vec<serde_json::Value>,
}

impl LoopResultForStep {
    fn new(answer: String, tool_events: Vec<serde_json::Value>) -> Self {
        Self {
            answer,
            tool_events,
        }
    }
}

async fn run_investigator_step(
    runtime: AgentRuntime,
    registry: Arc<ToolRegistry>,
    task: String,
    completed_context: String,
    state_msgs: Vec<ConversationMessage>,
    skills_prompt: String,
) -> Result<LoopResultForStep> {
    let config = LoopConfig {
        max_iterations: 10,
        system_prompt: format!(
            "You are zcode Investigator Agent (Model: {}).\n\
             Inspect existing context needed for the current task. Use tools to read/search as needed. \
             Do not modify files. Return concise findings that later agents can use.\n\n{}",
            runtime.model, skills_prompt
        ),
    };
    let prompt = format!(
        "Original Task: {}\n\nCompleted step summaries:\n{}\n\nInvestigate the current codebase context needed for this task.",
        task, completed_context
    );
    run_step_loop(
        runtime,
        registry,
        config,
        prompt,
        state_msgs,
        "investigator",
    )
    .await
}

async fn run_planner_step(
    runtime: AgentRuntime,
    registry: Arc<ToolRegistry>,
    task: String,
    completed_context: String,
    skills_prompt: String,
) -> Result<LoopResultForStep> {
    let config = LoopConfig {
        max_iterations: 10,
        system_prompt: format!(
            "You are zcode Planner Agent (Model: {}).\n\
             Formulate a concrete technical plan for the current task. \
             If the task is extremely simple, append `[FAST_PATH]` to your plan. \
             Do not modify files.\n\n{}",
            runtime.model, skills_prompt
        ),
    };
    let prompt = format!(
        "Original Task: {}\n\nCompleted step summaries:\n{}\n\nCreate the execution plan.",
        task, completed_context
    );
    run_step_loop(runtime, registry, config, prompt, Vec::new(), "planner").await
}

#[allow(clippy::too_many_arguments)]
async fn run_coder_step(
    runtime: AgentRuntime,
    fast_runtime: AgentRuntime,
    registry: Arc<ToolRegistry>,
    task: String,
    completed_context: String,
    last_msg: String,
    state_msgs: Vec<ConversationMessage>,
    skills_prompt: String,
    use_fast: bool,
    attempt: u64,
) -> Result<LoopResultForStep> {
    let active_model = if use_fast {
        fast_runtime.model.clone()
    } else {
        runtime.model.clone()
    };
    let active_runtime = if use_fast { fast_runtime } else { runtime };
    let config = LoopConfig {
        max_iterations: 20,
        system_prompt: format!(
            "You are zcode Coder Agent (Model: {}).\n\
             Execute the provided plan through a ReAct workflow. You may modify files using available tools. \
             Always verify your changes before reporting completion.\n\n{}",
            active_model, skills_prompt
        ),
    };
    let prompt = if attempt <= 1 {
        format!(
            "Original Task: {}\n\nCompleted step summaries:\n{}\n\nExecution Plan or latest context:\n{}",
            task, completed_context, last_msg
        )
    } else {
        format!(
            "Original Task: {}\n\nCompleted step summaries:\n{}\n\nReview feedback from attempt {}:\n{}\n\nFix the reported issues.",
            task, completed_context, attempt, last_msg
        )
    };
    run_step_loop(
        active_runtime,
        registry,
        config,
        prompt,
        state_msgs,
        "coder",
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn run_reviewer_step(
    runtime: AgentRuntime,
    fast_runtime: AgentRuntime,
    registry: Arc<ToolRegistry>,
    task: String,
    completed_context: String,
    coder_report: String,
    state_msgs: Vec<ConversationMessage>,
    skills_prompt: String,
    use_fast: bool,
) -> Result<LoopResultForStep> {
    let active_model = if use_fast {
        fast_runtime.model.clone()
    } else {
        runtime.model.clone()
    };
    let active_runtime = if use_fast { fast_runtime } else { runtime };
    let config = LoopConfig {
        max_iterations: 15,
        system_prompt: format!(
            "You are zcode Reviewer Agent (Model: {}).\n\
             Review and test the current task result. If all checks pass and the task is fulfilled, include the exact word 'PASS'. \
             If anything fails, do not include 'PASS' and describe fixes needed.\n\n{}",
            active_model, skills_prompt
        ),
    };
    let prompt = format!(
        "Original Task: {}\n\nCompleted step summaries:\n{}\n\nCoder reported:\n{}\n\nVerify this task.",
        task, completed_context, coder_report
    );
    run_step_loop(
        active_runtime,
        registry,
        config,
        prompt,
        state_msgs,
        "reviewer",
    )
    .await
}

async fn run_step_loop(
    runtime: AgentRuntime,
    registry: Arc<ToolRegistry>,
    config: LoopConfig,
    prompt: String,
    state_msgs: Vec<ConversationMessage>,
    agent: &'static str,
) -> Result<LoopResultForStep> {
    let provider = Arc::clone(&runtime.provider);
    let event_sink = runtime.event_sink.clone();
    let loop_engine = AgentLoop::new(config, registry);
    let mut tool_events = Vec::new();
    let result = loop_engine
        .run_with_events(
            &prompt,
            &state_msgs,
            &[],
            move |msgs, tools| {
                let provider = Arc::clone(&provider);
                async move { call_llm(provider, msgs, tools).await }
            },
            |event| emit_loop_event(&mut tool_events, event_sink.as_ref(), agent, event),
        )
        .await?;
    Ok(LoopResultForStep::new(result.answer, tool_events))
}

fn append_loop_event(events: &mut Vec<serde_json::Value>, agent: &str, event: LoopEvent) {
    match event {
        LoopEvent::ToolStart { tool_name, command } => {
            events.push(serde_json::json!({
                "kind": "start",
                "agent": agent,
                "tool_name": tool_name,
                "command": command
            }));
        }
        LoopEvent::ToolComplete { tool_name, success } => {
            events.push(serde_json::json!({
                "kind": "complete",
                "agent": agent,
                "tool_name": tool_name,
                "success": success
            }));
        }
    }
}

fn emit_loop_event(
    events: &mut Vec<serde_json::Value>,
    event_sink: Option<&Arc<dyn Fn(GraphEvent) + Send + Sync>>,
    agent: &str,
    event: LoopEvent,
) {
    if let Some(event_sink) = event_sink {
        match event {
            LoopEvent::ToolStart { tool_name, command } => {
                event_sink(GraphEvent::ToolStart {
                    agent: agent.to_string(),
                    tool_name,
                    command,
                });
            }
            LoopEvent::ToolComplete { tool_name, success } => {
                event_sink(GraphEvent::ToolComplete {
                    agent: agent.to_string(),
                    tool_name,
                    success,
                });
            }
        }
    } else {
        append_loop_event(events, agent, event);
    }
}

fn tool_events_output(events: Vec<serde_json::Value>) -> Option<NodeOutput> {
    if events.is_empty() {
        None
    } else {
        Some(NodeOutput::Custom(
            "__tool_events".to_string(),
            serde_json::Value::Array(events),
        ))
    }
}

// ─── Per-task pipeline ────────────────────────────────────────────────────────

/// Build the per-task agentic workflow:
///
/// ```text
/// supervisor → execute_step → supervisor
///     ├─ LLM decision: investigator/planner/coder/reviewer
///     └─ finish → END
/// ```
pub fn build_task_pipeline(
    provider: Arc<dyn LlmProvider>,
    fast_provider: Arc<dyn LlmProvider>,
    registry: Arc<ToolRegistry>,
    model: String,
    fast_model: String,
    skills_prompt: String,
) -> StateGraph {
    let runtimes = TaskAgentRuntimes {
        supervisor: AgentRuntime::new(Arc::clone(&provider), model.clone(), false),
        investigator: AgentRuntime::new(Arc::clone(&provider), model.clone(), false),
        planner: AgentRuntime::new(Arc::clone(&provider), model.clone(), false),
        coder: AgentRuntime::new(Arc::clone(&provider), model.clone(), false),
        reviewer: AgentRuntime::new(provider, model.clone(), false),
        fast: AgentRuntime::new(fast_provider, fast_model.clone(), false),
    };
    build_task_pipeline_with_limit(runtimes, registry, skills_prompt, 50)
}

/// Build the per-task workflow with an explicit graph iteration limit.
pub fn build_task_pipeline_with_limit(
    runtimes: TaskAgentRuntimes,
    registry: Arc<ToolRegistry>,
    skills_prompt: String,
    max_iterations: usize,
) -> StateGraph {
    let mut g = StateGraph::new("supervisor");

    let supervisor_runtime = runtimes.supervisor.clone();
    let supervisor_skills_prompt = skills_prompt.clone();
    g.add_node(AsyncFnNode::new("supervisor", move |state| {
        let runtime = supervisor_runtime.clone();
        let skills_prompt = supervisor_skills_prompt.clone();
        let task = current_task(&state.task);
        let mut plan = load_task_plan(state).unwrap_or_else(|| TaskPlan::new(task.clone()));
        let state_msgs = state.messages.clone();
        let retries = state
            .metadata
            .get("coder_retries")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let calls = supervisor_calls(state).saturating_add(1);
        state.agent_state = AgentState::Planning;

        async move {
            let mut outputs = vec![
                NodeOutput::Custom("root_agent".into(), serde_json::json!("supervisor")),
                supervisor_calls_output(calls),
            ];

            if plan.current_step().is_none() {
                let decision = decide_supervisor_next(
                    runtime,
                    task,
                    plan.clone(),
                    state_msgs,
                    skills_prompt,
                    retries,
                )
                .await;
                match decision.action {
                    SupervisorAction::ContinueTask => {
                        if let Some(agent) = decision.next_agent {
                            plan.append_step(
                                sanitize_step_title(decision.step_title, agent),
                                agent,
                            );
                        }
                    }
                    SupervisorAction::Finish => {
                        plan.current_index = plan.steps.len();
                    }
                }
            }

            let mut step_event = None;
            if let Some(step) = plan.current_step_mut() {
                if step.status == StepStatus::Pending {
                    step.status = StepStatus::Running;
                    step_event = Some(step_start_event(step));
                }
            }
            outputs.push(task_plan_output(&plan));
            if let Some(event) = step_event {
                if let Some(events) = step_events_output(vec![event]) {
                    outputs.push(events);
                }
            }
            if plan.current_step().is_some() {
                outputs.push(next_agent_output(&plan));
            } else {
                outputs.push(NodeOutput::Custom(
                    "next_node".into(),
                    serde_json::Value::Null,
                ));
            }
            Ok(NodeOutput::Multiple(outputs))
        }
    }));

    let step_runtimes = runtimes.clone();
    let step_registry = Arc::clone(&registry);
    let step_skills_prompt = skills_prompt.clone();
    g.add_node(AsyncFnNode::new("execute_step", move |state| {
        let runtimes = step_runtimes.clone();
        let registry = Arc::clone(&step_registry);
        let skills_prompt = step_skills_prompt.clone();
        let mut plan = load_task_plan(state).unwrap_or_else(|| {
            let task = state
                .task
                .clone()
                .map(|t| t.description)
                .unwrap_or_default();
            TaskPlan::new(task)
        });
        let step = plan.current_step().cloned();
        let task = state
            .task
            .clone()
            .map(|t| t.description)
            .unwrap_or_default();
        let state_msgs = state.messages.clone();
        let last_msg = state
            .messages
            .last()
            .and_then(|m| m.content.clone())
            .unwrap_or_default();
        let is_simple = state
            .metadata
            .get("is_simple")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let retries = state
            .metadata
            .get("coder_retries")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        async move {
            let Some(step) = step else {
                return Ok(NodeOutput::Multiple(vec![
                    task_plan_output(&plan),
                    NodeOutput::Custom("next_node".into(), serde_json::Value::Null),
                ]));
            };

            match step.agent {
                StepAgent::Investigator => {
                    let completed_context = completed_step_summaries(&plan);
                    let result = run_investigator_step(
                        runtimes.investigator,
                        registry,
                        task,
                        completed_context,
                        state_msgs,
                        skills_prompt,
                    )
                    .await?;
                    state_agent_output(&mut plan, &step, result, true, Vec::new())
                }
                StepAgent::Planner => {
                    let completed_context = completed_step_summaries(&plan);
                    let result = run_planner_step(
                        runtimes.planner,
                        registry,
                        task,
                        completed_context,
                        skills_prompt,
                    )
                    .await?;
                    state_agent_output(&mut plan, &step, result, true, Vec::new())
                }
                StepAgent::Coder => {
                    let new_retries = retries + 1;
                    let use_fast = is_simple && !runtimes.coder.explicit_model;
                    let result = run_coder_step(
                        runtimes.coder,
                        runtimes.fast,
                        registry,
                        task,
                        completed_step_summaries(&plan),
                        last_msg,
                        state_msgs,
                        skills_prompt,
                        use_fast,
                        new_retries,
                    )
                    .await?;
                    state_agent_output(
                        &mut plan,
                        &step,
                        result,
                        true,
                        vec![
                            NodeOutput::Custom(
                                "coder_retries".into(),
                                serde_json::json!(new_retries),
                            ),
                            NodeOutput::Custom("review_passed".into(), serde_json::Value::Null),
                            NodeOutput::Custom("test_passed".into(), serde_json::Value::Null),
                        ],
                    )
                }
                StepAgent::Reviewer => {
                    let use_fast = is_simple && !runtimes.reviewer.explicit_model;
                    let result = run_reviewer_step(
                        runtimes.reviewer,
                        runtimes.fast,
                        registry,
                        task,
                        completed_step_summaries(&plan),
                        last_msg,
                        state_msgs,
                        skills_prompt,
                        use_fast,
                    )
                    .await?;
                    let is_pass = result.answer.contains("PASS");
                    let mut extra_outputs = vec![
                        NodeOutput::Custom("review_passed".into(), serde_json::json!(is_pass)),
                        NodeOutput::Custom("test_passed".into(), serde_json::json!(is_pass)),
                    ];
                    if !is_pass && retries < 3 {
                        extra_outputs.push(NodeOutput::Custom(
                            "review_passed".into(),
                            serde_json::Value::Null,
                        ));
                        extra_outputs.push(NodeOutput::Custom(
                            "test_passed".into(),
                            serde_json::Value::Null,
                        ));
                    }
                    state_agent_output_with_review_retry(
                        &mut plan,
                        &step,
                        result,
                        is_pass,
                        retries,
                        extra_outputs,
                    )
                }
            }
        }
    }));

    g.add_conditional_edge(
        "supervisor",
        routers::next_node_router(),
        vec!["execute_step", "__end__"],
    );
    g.add_edge("execute_step", "supervisor");

    g.max_iterations(max_iterations)
}

// ─── Global reviewer pipeline ─────────────────────────────────────────────────

/// Build the global reviewer pipeline run **once** after all tasks are complete.
///
/// The caller is responsible for pre-populating `state.messages` with a combined
/// summary of all task outputs before calling `graph.execute(&mut state)`.
///
/// ```text
/// reviewer → self_learning → END
/// ```
pub fn build_reviewer_pipeline(
    provider: Arc<dyn LlmProvider>,
    registry: Arc<ToolRegistry>,
    model: String,
    skills_prompt: String,
) -> StateGraph {
    build_reviewer_pipeline_with_runtime(
        AgentRuntime::new(provider, model, false),
        registry,
        skills_prompt,
    )
}

pub fn build_reviewer_pipeline_with_runtime(
    reviewer: AgentRuntime,
    registry: Arc<ToolRegistry>,
    skills_prompt: String,
) -> StateGraph {
    let mut g = StateGraph::new("reviewer");

    let p = Arc::clone(&reviewer.provider);
    let r = Arc::clone(&registry);
    let m = reviewer.model.clone();
    let event_sink = reviewer.event_sink.clone();
    let sp = skills_prompt.clone();
    g.add_node(AsyncFnNode::new("reviewer", move |state| {
        let p = Arc::clone(&p);
        let r = Arc::clone(&r);
        let m = m.clone();
        let event_sink = event_sink.clone();
        let sp = sp.clone();

        // The combined report is expected as the last message in state
        let combined_report = state.messages.last().and_then(|msg| msg.content.clone()).unwrap_or_default();
        state.agent_state = AgentState::Reviewing;
        let state_msgs = state.messages.clone();

        async move {
            tracing::info!("[reviewer] Starting global code review for all completed tasks...");
            let config = LoopConfig {
                max_iterations: 15,
                system_prompt: format!(
                    "You are zcode Reviewer Agent (Model: {}).\n\
                     All tasks have been implemented and tested. Your job is to perform a holistic \
                     code review across all the changes made in this session.\n\
                     Inspect the combined task reports, verify correctness, consistency, style, \
                     and adherence to the original requirements.\n\
                     If everything looks good, include the exact word 'PASS' in your final answer. \n\
                     Otherwise, list findings grouped by file/component for the team to address.\n\n\
                     {}",
                     m, sp
                ),
            };

            let user_prompt = format!(
                "Combined Task Completion Report:\n\n{}\n\n\
                 Please review all the above changes holistically. Reply PASS if everything is satisfactory, \
                 or list your review findings.",
                combined_report
            );
            let loop_engine = AgentLoop::new(config, r);
            let mut tool_events = Vec::new();
            let result = loop_engine.run_with_events(&user_prompt, &state_msgs, &[], move |msgs, tools| {
                let p_inner = Arc::clone(&p);
                async move { call_llm(p_inner, msgs, tools).await }
            }, |event| emit_loop_event(&mut tool_events, event_sink.as_ref(), "reviewer", event)).await?;

            let is_pass = result.answer.contains("PASS");
            tracing::info!(
                "[reviewer] Global review verdict: {} (output: {} chars)",
                if is_pass { "PASS ✅" } else { "FAIL ❌" },
                result.answer.len()
            );

            let mut outputs = vec![
                NodeOutput::Custom("review_passed".into(), serde_json::json!(is_pass)),
                NodeOutput::Messages(vec![ConversationMessage::assistant_text(format!("REVIEW_FEEDBACK:\n{}", result.answer))]),
            ];
            if let Some(tool_events) = tool_events_output(tool_events) {
                outputs.push(tool_events);
            }
            Ok(NodeOutput::Multiple(outputs))
        }
    }));

    g.add_node(AsyncFnNode::new("self_learning", move |state| {
        let review_report = state
            .messages
            .last()
            .and_then(|message| message.content.clone())
            .unwrap_or_default();
        state.agent_state = AgentState::Learning;

        async move {
            let entry = SelfLearningAgent::summarize(&review_report);
            let content = format!(
                "SELF_LEARNING:\n# {}\n\n## Context\n{}\n\n## Mistake\n{}\n\n## Correction\n{}",
                entry.title, entry.context, entry.mistake, entry.correction
            );
            Ok(NodeOutput::Messages(vec![
                ConversationMessage::assistant_text(content),
            ]))
        }
    }));

    g.add_edge("reviewer", "self_learning");
    g
}

#[cfg(test)]
mod pipeline_tests;
