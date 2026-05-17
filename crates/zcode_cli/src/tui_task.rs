use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};

use zcode_capabilities::{Skill, SkillsLoader, ToolRegistry};
use zcode_core::Result;
use zcode_orchestration::{
    build_task_pipeline_with_limit, ConversationMessage, GraphEvent, TaskAgentRuntimes, TaskResult,
};
use zcode_requirements::{TaskRecord, TaskStatus};
use zcode_session::OPTIONAL_CONTEXT_GUARD;
use zcode_ui::{TaskExecutor, TaskRequest, TaskUiEvent};

use crate::cli_events::make_tui_tool_event_sink;

pub(crate) fn build_tui_task_executor(
    cwd: PathBuf,
    runtimes: TaskAgentRuntimes,
    registry: Arc<ToolRegistry>,
    skills: Vec<Skill>,
    max_iterations: usize,
) -> TaskExecutor {
    Arc::new(
        move |request: TaskRequest, cancel: Arc<AtomicBool>, tx: mpsc::Sender<TaskUiEvent>| {
            let cwd = cwd.clone();
            let runtimes = runtimes.clone();
            let registry = Arc::clone(&registry);
            let skills = skills.clone();

            let runtime = match tokio::runtime::Runtime::new() {
                Ok(runtime) => runtime,
                Err(error) => {
                    let _ = tx.send(TaskUiEvent::Error(format!(
                        "Failed to create task runtime: {}",
                        error
                    )));
                    return;
                }
            };

            runtime.block_on(async move {
                run_tui_task(
                    request,
                    cwd,
                    runtimes,
                    registry,
                    skills,
                    max_iterations,
                    cancel,
                    tx,
                )
                .await;
            });
        },
    )
}

pub(crate) fn seed_task_history(task_record: &mut TaskRecord, history: Vec<ConversationMessage>) {
    if history.is_empty() {
        return;
    }
    task_record
        .state
        .messages
        .push(ConversationMessage::system(OPTIONAL_CONTEXT_GUARD));
    task_record.state.messages.extend(history);
}

pub(crate) fn task_user_answer(messages: &[ConversationMessage]) -> String {
    find_report(messages, "CODER_REPORT")
        .or_else(|| find_report(messages, "INVESTIGATOR_REPORT"))
        .or_else(|| {
            messages
                .iter()
                .rev()
                .filter(|message| message.role == "assistant")
                .filter_map(|message| message.content.as_deref())
                .find(|content| !is_internal_report(content))
                .map(str::to_string)
        })
        .unwrap_or_else(|| "No output generated".to_string())
}

fn find_report(messages: &[ConversationMessage], prefix: &str) -> Option<String> {
    messages
        .iter()
        .rev()
        .filter(|message| message.role == "assistant")
        .filter_map(|message| message.content.as_deref())
        .find_map(|content| strip_report_prefix(content, prefix).map(str::to_string))
}

fn strip_report_prefix<'a>(content: &'a str, prefix: &str) -> Option<&'a str> {
    let rest = content.strip_prefix(prefix)?;
    let rest = rest.strip_prefix(':')?;
    Some(rest.strip_prefix('\n').unwrap_or(rest).trim())
}

fn is_internal_report(content: &str) -> bool {
    [
        "INVESTIGATOR_REPORT:",
        "PLANNER_REPORT:",
        "CODER_REPORT:",
        "REVIEWER_REPORT:",
        "REVIEW_FEEDBACK:",
        "SELF_LEARNING:",
    ]
    .iter()
    .any(|prefix| content.starts_with(prefix))
}

#[allow(clippy::too_many_arguments)]
async fn run_tui_task(
    request: TaskRequest,
    cwd: PathBuf,
    runtimes: TaskAgentRuntimes,
    registry: Arc<ToolRegistry>,
    skills: Vec<Skill>,
    max_iterations: usize,
    cancel: Arc<AtomicBool>,
    tx: mpsc::Sender<TaskUiEvent>,
) {
    if cancel.load(Ordering::SeqCst) {
        let _ = tx.send(TaskUiEvent::Cancelled);
        return;
    }

    match run_tui_task_inner(
        request,
        cwd,
        runtimes,
        registry,
        skills,
        max_iterations,
        Arc::clone(&cancel),
        tx.clone(),
    )
    .await
    {
        Ok(()) => {}
        Err(error) => {
            let _ = tx.send(TaskUiEvent::Error(error.to_string()));
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_tui_task_inner(
    request: TaskRequest,
    _cwd: PathBuf,
    runtimes: TaskAgentRuntimes,
    registry: Arc<ToolRegistry>,
    skills: Vec<Skill>,
    max_iterations: usize,
    cancel: Arc<AtomicBool>,
    tx: mpsc::Sender<TaskUiEvent>,
) -> Result<()> {
    let task = request.prompt;
    let skills_prompt = SkillsLoader::build_relevant_system_prompt("", &skills, &task);
    let mut task_record = TaskRecord::new(generate_transient_task_id(), task.clone());
    seed_task_history(&mut task_record, request.history);
    let _ = tx.send(TaskUiEvent::Thinking(format!("Task `{}` started\n", task)));

    let graph = build_task_pipeline_with_limit(
        runtimes.map_event_sink(make_tui_tool_event_sink(tx.clone())),
        Arc::clone(&registry),
        skills_prompt,
        max_iterations,
    )
    .compile()?;

    let graph_result = graph
        .execute_with_events_and_cancel(&mut task_record.state, |event| {
            send_graph_event(&tx, event);
            cancel.load(Ordering::SeqCst)
        })
        .await;

    if cancel.load(Ordering::SeqCst) {
        task_record.status = TaskStatus::Interrupted;
        task_record.error = Some("Interrupted by user".to_string());
        let _ = tx.send(TaskUiEvent::Cancelled);
        return Ok(());
    }

    finish_task(graph_result, task, task_record, tx)
}

fn finish_task(
    graph_result: Result<zcode_orchestration::GraphOutput>,
    _task: String,
    mut task_record: TaskRecord,
    tx: mpsc::Sender<TaskUiEvent>,
) -> Result<()> {
    match graph_result {
        Ok(graph_out) => {
            let review_passed = task_record
                .state
                .metadata
                .get("review_passed")
                .or_else(|| task_record.state.metadata.get("test_passed"))
                .and_then(|value| value.as_bool())
                .unwrap_or(true);

            let answer = task_user_answer(&task_record.state.messages);
            if review_passed {
                task_record.status = TaskStatus::Completed;
                task_record.state.result =
                    Some(TaskResult::success(task_record.id.clone(), answer.clone()));
            } else {
                task_record.status = TaskStatus::Failed;
                task_record.error = Some("Review/tests failed after max retries".to_string());
                task_record.state.result = Some(TaskResult::failure(
                    task_record.id.clone(),
                    "Review/tests failed after max retries",
                ));
            }

            let status = if review_passed { "completed" } else { "failed" };
            let final_answer = format!(
                "{}\n\nTask `{}` {} after {} graph iteration(s).",
                answer, task_record.id, status, graph_out.total_iterations
            );
            let _ = tx.send(TaskUiEvent::Done(final_answer));
            Ok(())
        }
        Err(error) => {
            task_record.status = TaskStatus::Failed;
            task_record.error = Some(error.to_string());
            Err(error)
        }
    }
}

fn generate_transient_task_id() -> String {
    use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    let counter = COUNTER.fetch_add(1, AtomicOrdering::Relaxed);
    format!("tui-{:x}-{:x}", millis, counter)
}

fn send_graph_event(tx: &mpsc::Sender<TaskUiEvent>, event: GraphEvent) {
    match event {
        GraphEvent::NodeStart { node, iteration } => {
            let _ = tx.send(TaskUiEvent::AgentStart(node.clone()));
            let _ = tx.send(TaskUiEvent::Thinking(format!(
                "{} started (iteration {})\n",
                node, iteration
            )));
        }
        GraphEvent::StepStart { id, title, agent } => {
            let _ = tx.send(TaskUiEvent::StepStart { id, title, agent });
        }
        GraphEvent::StepComplete {
            id,
            title,
            agent,
            success,
        } => {
            let _ = tx.send(TaskUiEvent::StepComplete {
                id,
                title,
                agent,
                success,
            });
        }
        GraphEvent::NodeComplete { node, .. } => {
            let _ = tx.send(TaskUiEvent::AgentComplete(node.clone()));
            let _ = tx.send(TaskUiEvent::Thinking(format!("{} completed\n", node)));
        }
        GraphEvent::EdgeTraversed { from, to } => {
            let target = to.unwrap_or_else(|| "END".to_string());
            let _ = tx.send(TaskUiEvent::Thinking(format!("{} -> {}\n", from, target)));
        }
        GraphEvent::ToolStart {
            agent,
            tool_name,
            command,
        } => {
            let _ = tx.send(TaskUiEvent::ToolStart {
                agent: agent.clone(),
                tool_name: tool_name.clone(),
                command: command.clone(),
            });
            let _ = tx.send(TaskUiEvent::Thinking(format!(
                "{} {}: {}\n",
                agent, tool_name, command
            )));
        }
        GraphEvent::ToolComplete {
            agent,
            tool_name,
            success,
        } => {
            let _ = tx.send(TaskUiEvent::ToolComplete {
                agent: agent.clone(),
                tool_name: tool_name.clone(),
                success,
            });
            let _ = tx.send(TaskUiEvent::Thinking(format!(
                "{} {} {}\n",
                agent,
                tool_name,
                if success { "succeeded" } else { "failed" }
            )));
        }
        GraphEvent::End { reason, output } => {
            let _ = tx.send(TaskUiEvent::Thinking(format!(
                "graph ended: {} ({} iteration(s))\n",
                reason, output.total_iterations
            )));
        }
    }
}
