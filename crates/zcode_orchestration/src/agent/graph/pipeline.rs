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

use std::sync::Arc;
use crate::agent::graph::graph::StateGraph;
use crate::agent::graph::node::AsyncFnNode;
use crate::agent::graph::state::NodeOutput;
use crate::agent::graph::edge::routers;
use crate::agent::loop_exec::{AgentLoop, LoopConfig, ConversationMessage, LlmResponse};
use crate::agent::self_learning::SelfLearningAgent;
use crate::agent::types::AgentState;
use zcode_llm_provider::provider::LlmProvider;
use zcode_llm_provider::Message;
use zcode_capabilities::ToolRegistry;
use zcode_core::Result;

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
    build_task_pipeline_with_limit(
        provider,
        fast_provider,
        registry,
        model,
        fast_model,
        skills_prompt,
        50,
    )
}

/// Build the per-task workflow with an explicit graph iteration limit.
pub fn build_task_pipeline_with_limit(
    provider: Arc<dyn LlmProvider>,
    fast_provider: Arc<dyn LlmProvider>,
    registry: Arc<ToolRegistry>,
    model: String,
    fast_model: String,
    skills_prompt: String,
    max_iterations: usize,
) -> StateGraph {
    let mut g = StateGraph::new("orchestrator");

    // ── Root Orchestrator Node ───────────────────────────────────────────────
    g.add_node(AsyncFnNode::new("orchestrator", move |state| {
        let task = state.task.clone().map(|t| t.description).unwrap_or_default();
        state.agent_state = AgentState::Planning;

        async move {
            tracing::info!(
                "[orchestrator] Scheduling task pipeline for: {}...",
                task.chars().take(80).collect::<String>()
            );

            Ok(NodeOutput::Multiple(vec![
                NodeOutput::Custom("root_agent".into(), serde_json::json!("orchestrator")),
                NodeOutput::Custom("next_agent".into(), serde_json::json!("planner")),
            ]))
        }
    }));

    // ── Planner Node ──────────────────────────────────────────────────────────
    let p1 = Arc::clone(&provider);
    let r1 = Arc::clone(&registry);
    let m1 = model.clone();
    let sp1 = skills_prompt.clone();
    g.add_node(AsyncFnNode::new("planner", move |state| {
        let p = Arc::clone(&p1);
        let r = Arc::clone(&r1);
        let m = m1.clone();
        let sp = sp1.clone();
        let task = state.task.clone().map(|t| t.description).unwrap_or_default();
        state.agent_state = AgentState::Planning;

        async move {
            tracing::info!("[planner] Starting planning phase for task: {}...", task.chars().take(80).collect::<String>());
            let config = LoopConfig {
                max_iterations: 10,
                system_prompt: format!(
                    "You are zcode Planner Agent (Model: {}).\n\
                     Your job is to read the user's task, inspect the codebase through available MCP/capability tools, \
                     and formulate a concrete technical plan.\n\
                     If the task is extremely simple (e.g. small bugfix, single function change), append `[FAST_PATH]` to your plan.\n\
                     Do NOT attempt to write code. Output a step-by-step Execution Plan.\n\n\
                     {}",
                     m, sp
                ),
            };

            let loop_engine = AgentLoop::new(config, r);
            let result = loop_engine.run(&task, &[], &[], move |msgs, tools| {
                let p2 = Arc::clone(&p);
                async move { call_llm(p2, msgs, tools).await }
            }).await?;

            let is_simple = result.answer.contains("[FAST_PATH]");
            tracing::info!("[planner] Plan generated ({} chars, fast_path={})", result.answer.len(), is_simple);
            Ok(NodeOutput::Multiple(vec![
                NodeOutput::Custom("is_simple".into(), serde_json::json!(is_simple)),
                NodeOutput::Messages(vec![ConversationMessage::assistant_text(format!("PLAN:\n{}", result.answer))])
            ]))
        }
    }));

    // ── Coder Node ────────────────────────────────────────────────────────────
    let p2 = Arc::clone(&provider);
    let fp2 = Arc::clone(&fast_provider);
    let r2 = Arc::clone(&registry);
    let m2 = model.clone();
    let fm2 = fast_model.clone();
    let sp2 = skills_prompt.clone();
    g.add_node(AsyncFnNode::new("coder", move |state| {
        let p = Arc::clone(&p2);
        let fp = Arc::clone(&fp2);
        let r = Arc::clone(&r2);
        let m = m2.clone();
        let fm = fm2.clone();
        let sp = sp2.clone();

        let task = state.task.clone().map(|t| t.description).unwrap_or_default();
        // Retrieve the plan or review-failure feedback from the last message
        let last_msg = state.messages.last().and_then(|m| m.content.clone()).unwrap_or_default();
        state.agent_state = AgentState::Executing;

        // Increment coder retry counter
        let retries = state.metadata
            .get("coder_retries")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let new_retries = retries + 1;
        let state_msgs = state.messages.clone();
        let is_simple = state.metadata.get("is_simple").and_then(|v| v.as_bool()).unwrap_or(false);

        async move {
            tracing::info!("[coder] Starting coder (attempt {}/3) for task: {}...", new_retries, task.chars().take(80).collect::<String>());
            let active_model = if is_simple { fm.clone() } else { m.clone() };
            let config = LoopConfig {
                max_iterations: 20,
                system_prompt: format!(
                    "You are zcode Coder Agent (Model: {}).\n\
                     Execute the provided technical plan through a ReAct workflow: reason, call available MCP/capability tools, observe, and continue.\n\
                     You may only use tools exposed in this session; do not assume local built-in file or shell tools exist.\n\
                     If you received a REVIEW FAILURE REPORT, you MUST fix the reported issues before finishing.\n\
                     Always verify your changes before reporting completion.\n\n\
                     {}",
                     active_model, sp
                )
            };

            let user_prompt = if retries == 0 {
                format!("Original Task: {}\n\nExecution Plan:\n{}", task, last_msg)
            } else {
                format!(
                    "Original Task: {}\n\nREVIEW FAILURE REPORT (attempt {}/{}):\n{}\n\nPlease fix the issues described above.",
                    task, new_retries, 3, last_msg
                )
            };

            let loop_engine = AgentLoop::new(config, r);
            let result = loop_engine.run(&user_prompt, &state_msgs, &[], move |msgs, tools| {
                let p_inner = if is_simple { Arc::clone(&fp) } else { Arc::clone(&p) };
                async move { call_llm(p_inner, msgs, tools).await }
            }).await?;

            tracing::info!("[coder] Coder completed (attempt {}), output: {} chars", new_retries, result.answer.len());
            // Update retry counter; reset review/test metadata so reviewer re-evaluates
            Ok(NodeOutput::Multiple(vec![
                NodeOutput::Custom("coder_retries".into(), serde_json::json!(new_retries)),
                NodeOutput::Custom("review_passed".into(), serde_json::Value::Null),
                NodeOutput::Custom("test_passed".into(), serde_json::Value::Null),
                NodeOutput::Messages(vec![ConversationMessage::assistant_text(format!("CODER_REPORT:\n{}", result.answer))]),
            ]))
        }
    }));

    // ── Reviewer/Test Gate Node ──────────────────────────────────────────────
    let p_review = Arc::clone(&provider);
    let fp_review = Arc::clone(&fast_provider);
    let r_review = Arc::clone(&registry);
    let m_review = model.clone();
    let fm_review = fast_model.clone();
    let sp_review = skills_prompt.clone();
    g.add_node(AsyncFnNode::new("reviewer", move |state| {
        let p = Arc::clone(&p_review);
        let fp = Arc::clone(&fp_review);
        let r = Arc::clone(&r_review);
        let m = m_review.clone();
        let fm = fm_review.clone();
        let sp = sp_review.clone();

        let task = state.task.clone().map(|t| t.description).unwrap_or_default();
        let coder_report = state.messages.last().and_then(|m| m.content.clone()).unwrap_or_default();

        state.agent_state = AgentState::Reviewing;
        let state_msgs = state.messages.clone();
        let is_simple = state.metadata.get("is_simple").and_then(|v| v.as_bool()).unwrap_or(false);

        async move {
            tracing::info!("[reviewer] Starting red/green review and test verification for this task...");
            let active_model = if is_simple { fm.clone() } else { m.clone() };
            let config = LoopConfig {
                max_iterations: 15,
                system_prompt: format!(
                    "You are zcode Reviewer Agent (Model: {}).\n\
                     Your job is to review and test ONLY the code changes made for the current task against its requirements.\n\
                     Run the relevant tests, build commands, or inspect outputs to verify correctness.\n\
                     Produce a red/green Review Report detailing what was checked, the commands run, and the outcomes.\n\
                     If all checks pass and the task is fulfilled, you MUST include the exact word 'PASS' in your final answer.\n\
                     If any check fails or the task is not fulfilled, do NOT include 'PASS' — \
                     describe the failures in detail so the Coder can fix them.\n\n\
                     {}",
                     active_model, sp
                ),
            };

            let user_prompt = format!(
                "Original Task: {}\n\nCoder reported:\n{}\n\n\
                 Please verify these changes by running the relevant tests/builds for this specific task.",
                task, coder_report
            );
            let loop_engine = AgentLoop::new(config, r);
            let result = loop_engine.run(&user_prompt, &state_msgs, &[], move |msgs, tools| {
                let p_inner = if is_simple { Arc::clone(&fp) } else { Arc::clone(&p) };
                async move { call_llm(p_inner, msgs, tools).await }
            }).await?;

            let is_pass = result.answer.contains("PASS");
            tracing::info!(
                "[reviewer] Review verdict: {} (output: {} chars)",
                if is_pass { "PASS ✅" } else { "FAIL ❌" },
                result.answer.len()
            );

            Ok(NodeOutput::Multiple(vec![
                NodeOutput::Custom("review_passed".into(), serde_json::json!(is_pass)),
                NodeOutput::Custom("test_passed".into(), serde_json::json!(is_pass)),
                NodeOutput::Messages(vec![ConversationMessage::assistant_text(format!("REVIEW_REPORT:\n{}", result.answer))]),
            ]))
        }
    }));

    // ── Edges ─────────────────────────────────────────────────────────────────
    g.add_edge("orchestrator", "planner");
    g.add_edge("planner", "coder");
    g.add_edge("coder", "reviewer");

    // Reviewer → PASS: END | FAIL + retries < 3: back to coder | FAIL + exhausted: force END
    g.add_conditional_edge(
        "reviewer",
        routers::review_router_with_limit("coder", 3),
        vec!["coder", "__end__"],
    );

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
    let mut g = StateGraph::new("reviewer");

    let p = Arc::clone(&provider);
    let r = Arc::clone(&registry);
    let m = model.clone();
    let sp = skills_prompt.clone();
    g.add_node(AsyncFnNode::new("reviewer", move |state| {
        let p = Arc::clone(&p);
        let r = Arc::clone(&r);
        let m = m.clone();
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
            let result = loop_engine.run(&user_prompt, &state_msgs, &[], move |msgs, tools| {
                let p_inner = Arc::clone(&p);
                async move { call_llm(p_inner, msgs, tools).await }
            }).await?;

            let is_pass = result.answer.contains("PASS");
            tracing::info!(
                "[reviewer] Global review verdict: {} (output: {} chars)",
                if is_pass { "PASS ✅" } else { "FAIL ❌" },
                result.answer.len()
            );

            Ok(NodeOutput::Multiple(vec![
                NodeOutput::Custom("review_passed".into(), serde_json::json!(is_pass)),
                NodeOutput::Messages(vec![ConversationMessage::assistant_text(format!("REVIEW_FEEDBACK:\n{}", result.answer))]),
            ]))
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
            Ok(NodeOutput::Messages(vec![ConversationMessage::assistant_text(content)]))
        }
    }));

    g.add_edge("reviewer", "self_learning");
    g
}

// ─── Internal LLM helper ──────────────────────────────────────────────────────

async fn call_llm(p: Arc<dyn LlmProvider>, msgs: Vec<serde_json::Value>, tools: Vec<serde_json::Value>) -> Result<LlmResponse> {
    let llm_messages: Vec<Message> = msgs.iter()
        .filter_map(|v| {
            let role = v.get("role")?.as_str()?;
            match role {
                "system" => {
                    let content = v.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
                    Some(Message::system(content))
                }
                "assistant" => {
                    if let Some(tool_calls) = v.get("tool_calls").and_then(|tc| tc.as_array()) {
                        if !tool_calls.is_empty() {
                            let content = v.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
                            let reasoning_content = v.get("reasoning_content").and_then(|c| c.as_str()).map(|s| s.to_string());
                            Some(Message::assistant_with_tool_calls_and_reasoning(content, tool_calls.clone(), reasoning_content))
                        } else {
                            let content = v.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
                            Some(Message::assistant(content))
                        }
                    } else {
                        let content = v.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
                        Some(Message::assistant(content))
                    }
                }
                "tool" => {
                    let tool_call_id = v.get("tool_call_id").and_then(|id| id.as_str()).unwrap_or("").to_string();
                    let name = v.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
                    let content = v.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
                    Some(Message::tool_result(tool_call_id, name, content))
                }
                _ => {
                    // "user" and anything else
                    let content = v.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
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
        Err(zcode_core::ZcodeError::MissingApiKey(provider)) => {
            Ok(LlmResponse::Text(format!(
                "Task acknowledged. No API key found for '{}'. \
                 Set ZCODE_API_KEY to enable LLM responses.",
                provider
            )))
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[tokio::test]
    async fn test_task_pipeline_starts_with_orchestrator_and_reviewer_gate() {
        let provider: Arc<dyn LlmProvider> = Arc::new(ScriptedProvider::new([
            "PLAN: do it",
            "CODER: done",
            "PASS",
        ]));
        let graph = build_task_pipeline_with_limit(
            Arc::clone(&provider),
            Arc::clone(&provider),
            empty_registry(),
            "model".to_string(),
            "fast-model".to_string(),
            String::new(),
            10,
        )
        .compile()
        .unwrap();

        let mut state = DefaultState::new(Task::new("implement task"));
        let output = graph.execute(&mut state).await.unwrap();

        assert_eq!(
            output.nodes_executed,
            vec!["orchestrator", "planner", "coder", "reviewer"]
        );
        assert_eq!(
            state.metadata.get("root_agent").and_then(|v| v.as_str()),
            Some("orchestrator")
        );
        assert_eq!(
            state.metadata.get("review_passed").and_then(|v| v.as_bool()),
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
            Arc::clone(&provider),
            Arc::clone(&provider),
            empty_registry(),
            "model".to_string(),
            "fast-model".to_string(),
            String::new(),
            10,
        )
        .compile()
        .unwrap();

        let mut state = DefaultState::new(Task::new("implement task"));
        let output = graph.execute(&mut state).await.unwrap();

        assert_eq!(
            output.nodes_executed,
            vec!["orchestrator", "planner", "coder", "reviewer", "coder", "reviewer"]
        );
        assert_eq!(
            state.metadata.get("coder_retries").and_then(|v| v.as_u64()),
            Some(2)
        );
        assert_eq!(
            state.metadata.get("review_passed").and_then(|v| v.as_bool()),
            Some(true)
        );
    }
}
