//! Comprehensive LangGraph pipelines for Zcode
//!
//! Two pipelines are provided:
//!
//! - `build_task_pipeline()`: per-task graph
//!   (`orchestrator → planner → coder(ReAct) → reviewer`).
//!   The root orchestrator schedules the planner/coder/reviewer nodes. After
//!   the reviewer the graph either ends (PASS or retries exhausted) or loops
//!   back to the coder (FAIL, retries remaining). Maximum 3 coder retries per
//!   task.
//!
//! - `build_reviewer_pipeline()`: global review graph run **once** after all
//!   tasks have completed.  The reviewer receives a combined report of all
//!   task outputs via the initial state messages.

use crate::agent::graph::edge::routers;
use crate::agent::graph::graph::{GraphEvent, StateGraph};
use crate::agent::graph::node::AsyncFnNode;
use crate::agent::graph::state::NodeOutput;
use crate::agent::loop_exec::{AgentLoop, ConversationMessage, LlmResponse, LoopConfig, LoopEvent};
use crate::agent::self_learning::SelfLearningAgent;
use crate::agent::types::AgentState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use zcode_capabilities::ToolRegistry;
use zcode_core::Result;
use zcode_llm_provider::provider::LlmProvider;
use zcode_llm_provider::Message;

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
    pub investigator: AgentRuntime,
    pub planner: AgentRuntime,
    pub coder: AgentRuntime,
    pub reviewer: AgentRuntime,
    pub fast: AgentRuntime,
}

impl TaskAgentRuntimes {
    pub fn map_event_sink(mut self, event_sink: Arc<dyn Fn(GraphEvent) + Send + Sync>) -> Self {
        self.investigator = self.investigator.with_event_sink(Arc::clone(&event_sink));
        self.planner = self.planner.with_event_sink(Arc::clone(&event_sink));
        self.coder = self.coder.with_event_sink(Arc::clone(&event_sink));
        self.reviewer = self.reviewer.with_event_sink(event_sink);
        self
    }
}

#[derive(Debug, Clone)]
pub struct AgentModelLabels {
    pub investigator: String,
    pub planner: String,
    pub coder: String,
    pub reviewer: String,
    pub fast: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct TaskPlan {
    objective: String,
    steps: Vec<TaskStep>,
    current_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct TaskStep {
    id: String,
    title: String,
    agent: StepAgent,
    status: StepStatus,
    summary: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum StepAgent {
    Investigator,
    Planner,
    Coder,
    Reviewer,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

impl StepAgent {
    fn as_str(self) -> &'static str {
        match self {
            StepAgent::Investigator => "investigator",
            StepAgent::Planner => "planner",
            StepAgent::Coder => "coder",
            StepAgent::Reviewer => "reviewer",
        }
    }
}

impl TaskPlan {
    fn for_task(task: String) -> Self {
        let needs_investigation = needs_investigation(&task);
        let mut steps = Vec::new();
        if needs_investigation {
            steps.push(TaskStep::new(
                "step-1",
                "Inspect existing context",
                StepAgent::Investigator,
            ));
        }
        let planner_id = format!("step-{}", steps.len() + 1);
        steps.push(TaskStep::new(
            planner_id,
            "Plan the implementation",
            StepAgent::Planner,
        ));
        let coder_id = format!("step-{}", steps.len() + 1);
        steps.push(TaskStep::new(
            coder_id,
            "Apply the changes",
            StepAgent::Coder,
        ));
        let reviewer_id = format!("step-{}", steps.len() + 1);
        steps.push(TaskStep::new(
            reviewer_id,
            "Verify the result",
            StepAgent::Reviewer,
        ));

        Self {
            objective: task,
            steps,
            current_index: 0,
        }
    }

    fn current_step(&self) -> Option<&TaskStep> {
        self.steps.get(self.current_index)
    }

    fn current_step_mut(&mut self) -> Option<&mut TaskStep> {
        self.steps.get_mut(self.current_index)
    }
}

impl TaskStep {
    fn new(id: impl Into<String>, title: impl Into<String>, agent: StepAgent) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            agent,
            status: StepStatus::Pending,
            summary: None,
        }
    }
}

fn needs_investigation(task: &str) -> bool {
    let lower = task.to_lowercase();
    [
        "read",
        "find",
        "search",
        "analyze",
        "inspect",
        "understand",
        "调查",
        "分析",
        "查找",
        "读取",
        "看看",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn load_task_plan(state: &zcode_core::agent::DefaultState) -> Option<TaskPlan> {
    state
        .metadata
        .get("task_plan")
        .and_then(|value| serde_json::from_value(value.clone()).ok())
}

fn task_plan_output(plan: &TaskPlan) -> NodeOutput {
    NodeOutput::Custom(
        "task_plan".to_string(),
        serde_json::to_value(plan).unwrap_or_else(|_| serde_json::Value::Null),
    )
}

fn next_agent_output(plan: &TaskPlan) -> NodeOutput {
    NodeOutput::Custom(
        "next_node".to_string(),
        plan.current_step()
            .map(|_| serde_json::json!("execute_step"))
            .unwrap_or(serde_json::Value::Null),
    )
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

fn completed_step_summaries(plan: &TaskPlan) -> String {
    let summaries = plan
        .steps
        .iter()
        .filter_map(|step| {
            let summary = step.summary.as_ref()?;
            Some(format!(
                "- {} [{}]: {}",
                step.title,
                step.agent.as_str(),
                summary
            ))
        })
        .collect::<Vec<_>>();
    if summaries.is_empty() {
        "No completed steps yet.".to_string()
    } else {
        summaries.join("\n")
    }
}

fn complete_current_step(plan: &mut TaskPlan, _step: &TaskStep, summary: String, success: bool) {
    if let Some(current) = plan.current_step_mut() {
        current.status = if success {
            StepStatus::Completed
        } else {
            StepStatus::Failed
        };
        current.summary = Some(summary);
    }
    if success {
        plan.current_index += 1;
    }
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
        if let Some(reviewer) = plan.current_step_mut() {
            reviewer.status = StepStatus::Pending;
        }
        if let Some(coder_index) = plan
            .steps
            .iter()
            .position(|candidate| candidate.agent == StepAgent::Coder)
        {
            plan.current_index = coder_index;
            if let Some(coder) = plan.steps.get_mut(coder_index) {
                coder.status = StepStatus::Pending;
                coder.title = "Fix review findings".to_string();
            }
        }
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
/// orchestrator → planner → coder(ReAct) → reviewer
///                                      ├─ PASS ──────────────────────────→ END
///                                      └─ FAIL + retries < 3 ── → coder (with failure context)
///                                      └─ FAIL + retries >= 3 ─→ END (force-stop)
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

    g.add_node(AsyncFnNode::new("supervisor", move |state| {
        let task = state
            .task
            .clone()
            .map(|t| t.description)
            .unwrap_or_default();
        let mut plan = load_task_plan(state).unwrap_or_else(|| TaskPlan::for_task(task.clone()));
        state.agent_state = AgentState::Planning;

        async move {
            let mut outputs = vec![
                NodeOutput::Custom("root_agent".into(), serde_json::json!("supervisor")),
                task_plan_output(&plan),
            ];
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
            TaskPlan::for_task(task)
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

// ─── Internal LLM helper ──────────────────────────────────────────────────────

async fn call_llm(
    p: Arc<dyn LlmProvider>,
    msgs: Vec<serde_json::Value>,
    tools: Vec<serde_json::Value>,
) -> Result<LlmResponse> {
    let llm_messages: Vec<Message> = msgs
        .iter()
        .filter_map(|v| {
            let role = v.get("role")?.as_str()?;
            match role {
                "system" => {
                    let content = v
                        .get("content")
                        .and_then(|c| c.as_str())
                        .unwrap_or("")
                        .to_string();
                    Some(Message::system(content))
                }
                "assistant" => {
                    if let Some(tool_calls) = v.get("tool_calls").and_then(|tc| tc.as_array()) {
                        if !tool_calls.is_empty() {
                            let content = v
                                .get("content")
                                .and_then(|c| c.as_str())
                                .unwrap_or("")
                                .to_string();
                            let reasoning_content = v
                                .get("reasoning_content")
                                .and_then(|c| c.as_str())
                                .map(|s| s.to_string());
                            Some(Message::assistant_with_tool_calls_and_reasoning(
                                content,
                                tool_calls.clone(),
                                reasoning_content,
                            ))
                        } else {
                            let content = v
                                .get("content")
                                .and_then(|c| c.as_str())
                                .unwrap_or("")
                                .to_string();
                            Some(Message::assistant(content))
                        }
                    } else {
                        let content = v
                            .get("content")
                            .and_then(|c| c.as_str())
                            .unwrap_or("")
                            .to_string();
                        Some(Message::assistant(content))
                    }
                }
                "tool" => {
                    let tool_call_id = v
                        .get("tool_call_id")
                        .and_then(|id| id.as_str())
                        .unwrap_or("")
                        .to_string();
                    let name = v
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_string();
                    let content = v
                        .get("content")
                        .and_then(|c| c.as_str())
                        .unwrap_or("")
                        .to_string();
                    Some(Message::tool_result(tool_call_id, name, content))
                }
                _ => {
                    // "user" and anything else
                    let content = v
                        .get("content")
                        .and_then(|c| c.as_str())
                        .unwrap_or("")
                        .to_string();
                    Some(Message::user(content))
                }
            }
        })
        .collect();

    match p.chat(&llm_messages, &tools) {
        Ok(resp) => {
            if let Ok(agent_resp) = LlmResponse::from_openai_response(&resp.raw_response) {
                Ok(agent_resp)
            } else {
                Ok(LlmResponse::Text(resp.content))
            }
        }
        Err(zcode_core::ZcodeError::MissingApiKey(provider)) => Ok(LlmResponse::Text(format!(
            "Task acknowledged. No API key found for '{}'. \
                 Set ZCODE_API_KEY to enable LLM responses.",
            provider
        ))),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::graph::graph::GraphEvent;
    use std::collections::VecDeque;
    use std::sync::Mutex;
    use zcode_core::agent::{DefaultState, Task};
    use zcode_core::llm::{LlmResponse as ProviderLlmResponse, UsageStats};

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
            "PLAN: do it",
            "CODER: done",
            "PASS",
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
            "PLAN: do it",
            "CODER: first attempt",
            "FAIL: missing behavior",
            "CODER: fixed",
            "PASS",
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
            "PLAN: do it",
            "CODER: done",
            "PASS",
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
}
