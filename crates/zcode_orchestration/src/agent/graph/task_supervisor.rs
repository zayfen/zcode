use super::llm_bridge::call_llm;
use super::pipeline::AgentRuntime;
use crate::agent::graph::state::NodeOutput;
use crate::agent::loop_exec::{AgentLoop, ConversationMessage, LoopConfig};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use zcode_capabilities::ToolRegistry;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct TaskPlan {
    objective: String,
    pub(super) steps: Vec<TaskStep>,
    pub(super) current_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct TaskStep {
    pub(super) id: String,
    pub(super) title: String,
    pub(super) agent: StepAgent,
    pub(super) status: StepStatus,
    pub(super) summary: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum StepAgent {
    Investigator,
    Planner,
    Coder,
    Reviewer,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum SupervisorAction {
    ContinueTask,
    Finish,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub(super) struct SupervisorDecision {
    pub(super) action: SupervisorAction,
    #[serde(default)]
    pub(super) next_agent: Option<StepAgent>,
    #[serde(default)]
    pub(super) step_title: Option<String>,
    #[serde(default)]
    reason: Option<String>,
}

impl StepAgent {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            StepAgent::Investigator => "investigator",
            StepAgent::Planner => "planner",
            StepAgent::Coder => "coder",
            StepAgent::Reviewer => "reviewer",
        }
    }

    fn default_title(self) -> &'static str {
        match self {
            StepAgent::Investigator => "Inspect existing context",
            StepAgent::Planner => "Plan the implementation",
            StepAgent::Coder => "Apply the changes",
            StepAgent::Reviewer => "Verify the result",
        }
    }
}

impl TaskPlan {
    pub(super) fn new(task: String) -> Self {
        Self {
            objective: task,
            steps: Vec::new(),
            current_index: 0,
        }
    }

    pub(super) fn current_step(&self) -> Option<&TaskStep> {
        self.steps.get(self.current_index)
    }

    pub(super) fn current_step_mut(&mut self) -> Option<&mut TaskStep> {
        self.steps.get_mut(self.current_index)
    }

    pub(super) fn append_step(&mut self, title: impl Into<String>, agent: StepAgent) {
        let id = format!("step-{}", self.steps.len() + 1);
        self.steps.push(TaskStep::new(id, title, agent));
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

pub(super) fn load_task_plan(state: &zcode_core::agent::DefaultState) -> Option<TaskPlan> {
    state
        .metadata
        .get("task_plan")
        .and_then(|value| serde_json::from_value(value.clone()).ok())
}

pub(super) fn supervisor_calls(state: &zcode_core::agent::DefaultState) -> u64 {
    state
        .metadata
        .get("supervisor_calls")
        .and_then(|value| value.as_u64())
        .unwrap_or(0)
}

pub(super) fn supervisor_calls_output(calls: u64) -> NodeOutput {
    NodeOutput::Custom("supervisor_calls".to_string(), serde_json::json!(calls))
}

pub(super) fn task_plan_output(plan: &TaskPlan) -> NodeOutput {
    NodeOutput::Custom(
        "task_plan".to_string(),
        serde_json::to_value(plan).unwrap_or_else(|_| serde_json::Value::Null),
    )
}

pub(super) fn next_agent_output(plan: &TaskPlan) -> NodeOutput {
    NodeOutput::Custom(
        "next_node".to_string(),
        plan.current_step()
            .map(|_| serde_json::json!("execute_step"))
            .unwrap_or(serde_json::Value::Null),
    )
}

pub(super) fn completed_step_summaries(plan: &TaskPlan) -> String {
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

pub(super) fn complete_current_step(
    plan: &mut TaskPlan,
    _step: &TaskStep,
    summary: String,
    success: bool,
) {
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

pub(super) fn sanitize_step_title(title: Option<String>, agent: StepAgent) -> String {
    title
        .map(|title| title.trim().to_string())
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| agent.default_title().to_string())
}

pub(super) async fn decide_supervisor_next(
    runtime: AgentRuntime,
    task: String,
    plan: TaskPlan,
    state_msgs: Vec<ConversationMessage>,
    skills_prompt: String,
    retries: u64,
) -> SupervisorDecision {
    let config = LoopConfig {
        max_iterations: 1,
        system_prompt: format!(
            "You are zcode Supervisor Agent (Model: {}). You schedule exactly one next step at a time. \
             You do not execute tools. You only return JSON decisions.\n\n{}",
            runtime.model, skills_prompt
        ),
    };
    let provider = Arc::clone(&runtime.provider);
    let loop_engine = AgentLoop::new(config, Arc::new(ToolRegistry::new()));
    let prompt = supervisor_prompt(&task, &plan, retries);
    let result = loop_engine
        .run(&prompt, &state_msgs, &[], move |msgs, tools| {
            let provider = Arc::clone(&provider);
            async move { call_llm(provider, msgs, tools).await }
        })
        .await
        .ok();

    normalize_supervisor_decision(
        result.and_then(|result| parse_supervisor_decision(&result.answer)),
        &plan,
        &task,
        retries,
    )
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
        "list",
        "show",
        "what",
        "which",
        "who",
        "when",
        "where",
        "how",
        "调查",
        "分析",
        "查找",
        "读取",
        "看看",
        "目录",
        "文件",
        "列出",
        "有哪些",
        "当前",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn is_read_only_task(task: &str) -> bool {
    let lower = task.to_lowercase();
    let mutating = [
        "implement",
        "fix",
        "add",
        "update",
        "modify",
        "change",
        "write",
        "create",
        "delete",
        "refactor",
        "实现",
        "修复",
        "增加",
        "添加",
        "更新",
        "修改",
        "创建",
        "删除",
        "重构",
    ]
    .iter()
    .any(|needle| lower.contains(needle));
    needs_investigation(task) && !mutating
}

fn last_completed_agent(plan: &TaskPlan) -> Option<StepAgent> {
    plan.steps
        .iter()
        .rev()
        .find(|step| step.status == StepStatus::Completed)
        .map(|step| step.agent)
}

fn last_step_is_failed_reviewer(plan: &TaskPlan) -> bool {
    matches!(
        plan.steps.last(),
        Some(step) if step.agent == StepAgent::Reviewer && step.status == StepStatus::Failed
    )
}

fn parse_supervisor_decision(text: &str) -> Option<SupervisorDecision> {
    serde_json::from_str(text.trim()).ok().or_else(|| {
        let start = text.find('{')?;
        let end = text.rfind('}')?;
        serde_json::from_str(&text[start..=end]).ok()
    })
}

fn normalize_supervisor_decision(
    decision: Option<SupervisorDecision>,
    plan: &TaskPlan,
    task: &str,
    retries: u64,
) -> SupervisorDecision {
    let fallback = fallback_supervisor_decision(plan, task, retries);
    let Some(mut decision) = decision else {
        return fallback;
    };

    match decision.action {
        SupervisorAction::Finish => {
            if !can_finish(plan, task, retries) {
                fallback
            } else {
                SupervisorDecision {
                    action: SupervisorAction::Finish,
                    next_agent: None,
                    step_title: None,
                    reason: decision.reason.take(),
                }
            }
        }
        SupervisorAction::ContinueTask => {
            let Some(agent) = decision.next_agent else {
                return fallback;
            };
            if !is_allowed_next_agent(agent, plan, task) {
                return fallback;
            }
            SupervisorDecision {
                action: SupervisorAction::ContinueTask,
                next_agent: Some(agent),
                step_title: Some(sanitize_step_title(decision.step_title.take(), agent)),
                reason: decision.reason.take(),
            }
        }
    }
}

fn can_finish(plan: &TaskPlan, task: &str, retries: u64) -> bool {
    let failed_reviewer = last_step_is_failed_reviewer(plan);
    !plan.steps.is_empty()
        && !plan
            .steps
            .iter()
            .any(|step| step.status == StepStatus::Running)
        && (is_read_only_task(task)
            || (failed_reviewer && retries >= 3)
            || matches!(
                last_completed_agent(plan),
                Some(StepAgent::Coder | StepAgent::Reviewer)
            ))
}

fn is_allowed_next_agent(agent: StepAgent, plan: &TaskPlan, task: &str) -> bool {
    if plan.steps.is_empty() {
        return match agent {
            StepAgent::Investigator => true,
            StepAgent::Planner => true,
            StepAgent::Coder => true,
            StepAgent::Reviewer => false,
        };
    }

    match agent {
        StepAgent::Investigator => false,
        StepAgent::Planner => !is_read_only_task(task),
        StepAgent::Coder => {
            last_step_is_failed_reviewer(plan)
                || !matches!(
                    last_completed_agent(plan),
                    Some(StepAgent::Coder | StepAgent::Reviewer)
                )
        }
        StepAgent::Reviewer => matches!(last_completed_agent(plan), Some(StepAgent::Coder)),
    }
}

fn fallback_supervisor_decision(plan: &TaskPlan, task: &str, retries: u64) -> SupervisorDecision {
    let agent = if plan.steps.is_empty() {
        if is_read_only_task(task) || needs_investigation(task) {
            StepAgent::Investigator
        } else {
            StepAgent::Planner
        }
    } else {
        match last_completed_agent(plan) {
            _ if last_step_is_failed_reviewer(plan) && retries < 3 => StepAgent::Coder,
            _ if last_step_is_failed_reviewer(plan) => {
                return finish_decision("review retry limit reached");
            }
            Some(StepAgent::Investigator) if is_read_only_task(task) => {
                return finish_decision("read-only task completed");
            }
            Some(StepAgent::Investigator) => StepAgent::Planner,
            Some(StepAgent::Planner) => StepAgent::Coder,
            Some(StepAgent::Coder) => StepAgent::Reviewer,
            Some(StepAgent::Reviewer) if retries > 0 && retries < 3 => StepAgent::Coder,
            Some(StepAgent::Reviewer) => return finish_decision("review completed"),
            None => StepAgent::Planner,
        }
    };

    let title = if retries > 0 && agent == StepAgent::Coder {
        "Fix review findings".to_string()
    } else {
        agent.default_title().to_string()
    };
    SupervisorDecision {
        action: SupervisorAction::ContinueTask,
        next_agent: Some(agent),
        step_title: Some(title),
        reason: Some("fallback policy".to_string()),
    }
}

fn finish_decision(reason: &str) -> SupervisorDecision {
    SupervisorDecision {
        action: SupervisorAction::Finish,
        next_agent: None,
        step_title: None,
        reason: Some(reason.to_string()),
    }
}

fn supervisor_prompt(task: &str, plan: &TaskPlan, retries: u64) -> String {
    format!(
        "Current task:\n{}\n\nCompleted step summaries:\n{}\n\nCurrent plan JSON:\n{}\n\nCoder retry count: {}\n\n\
         Decide the next supervisor action. Return ONLY compact JSON with this schema:\n\
         {{\"action\":\"continue_task\"|\"finish\",\"next_agent\":\"investigator\"|\"planner\"|\"coder\"|\"reviewer\"|null,\"step_title\":\"short current step name\"|null,\"reason\":\"short reason\"}}\n\n\
         Rules:\n\
         - For read-only questions, use investigator or coder and finish after the result is available.\n\
         - For code changes, normally plan, code, then review.\n\
         - Do not review before code was produced.\n\
         - Finish when the task result is already sufficient.",
        task,
        completed_step_summaries(plan),
        serde_json::to_string(plan).unwrap_or_default(),
        retries
    )
}
