//! Comprehensive LangGraph pipelines for Zcode
//!
//! Two pipelines are provided:
//!
//! - `build_task_pipeline()`: per-task graph (`planner → coder → tester`).
//!   After the tester the graph either ends (PASS or retries exhausted) or
//!   loops back to the coder (FAIL, retries remaining).  Maximum 3 coder
//!   retries per task.
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
/// planner → coder → tester
///                     ├─ PASS ──────────────────────────→ END
///                     └─ FAIL + retries < 3 ── → coder (with failure context)
///                     └─ FAIL + retries >= 3 ─→ END (force-stop)
/// ```
///
/// The reviewer is **not** part of this graph — it is invoked separately via
/// `build_reviewer_pipeline()` after all tasks have finished.
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
    let mut g = StateGraph::new("planner");

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
        // Retrieve the plan or test-failure feedback from the last message
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
                     If you received a TEST FAILURE REPORT, you MUST fix the reported issues before finishing.\n\
                     Always verify your changes before reporting completion.\n\n\
                     {}",
                     active_model, sp
                )
            };

            let user_prompt = if retries == 0 {
                format!("Original Task: {}\n\nExecution Plan:\n{}", task, last_msg)
            } else {
                format!(
                    "Original Task: {}\n\nTEST FAILURE REPORT (attempt {}/{}):\n{}\n\nPlease fix the issues described above.",
                    task, new_retries, 3, last_msg
                )
            };

            let loop_engine = AgentLoop::new(config, r);
            let result = loop_engine.run(&user_prompt, &state_msgs, &[], move |msgs, tools| {
                let p_inner = if is_simple { Arc::clone(&fp) } else { Arc::clone(&p) };
                async move { call_llm(p_inner, msgs, tools).await }
            }).await?;

            tracing::info!("[coder] Coder completed (attempt {}), output: {} chars", new_retries, result.answer.len());
            // Update retry counter; reset test_passed so tester re-evaluates
            Ok(NodeOutput::Multiple(vec![
                NodeOutput::Custom("coder_retries".into(), serde_json::json!(new_retries)),
                NodeOutput::Custom("test_passed".into(), serde_json::Value::Null),
                NodeOutput::Messages(vec![ConversationMessage::assistant_text(format!("CODER_REPORT:\n{}", result.answer))]),
            ]))
        }
    }));

    // ── Tester Node ───────────────────────────────────────────────────────────
    let p_test = Arc::clone(&provider);
    let fp_test = Arc::clone(&fast_provider);
    let r_test = Arc::clone(&registry);
    let m_test = model.clone();
    let fm_test = fast_model.clone();
    let sp_test = skills_prompt.clone();
    g.add_node(AsyncFnNode::new("tester", move |state| {
        let p = Arc::clone(&p_test);
        let fp = Arc::clone(&fp_test);
        let r = Arc::clone(&r_test);
        let m = m_test.clone();
        let fm = fm_test.clone();
        let sp = sp_test.clone();

        let task = state.task.clone().map(|t| t.description).unwrap_or_default();
        let coder_report = state.messages.last().and_then(|m| m.content.clone()).unwrap_or_default();

        state.agent_state = AgentState::Executing;
        let state_msgs = state.messages.clone();
        let is_simple = state.metadata.get("is_simple").and_then(|v| v.as_bool()).unwrap_or(false);

        async move {
            tracing::info!("[tester] Starting test verification for this task...");
            let active_model = if is_simple { fm.clone() } else { m.clone() };
            let config = LoopConfig {
                max_iterations: 15,
                system_prompt: format!(
                    "You are zcode Tester Agent (Model: {}).\n\
                     Your job is to test ONLY the code changes made for the current task against its requirements.\n\
                     Run the relevant tests, build commands, or inspect outputs to verify correctness.\n\
                     Produce a Test Report detailing what was checked, the commands run, and the outcomes.\n\
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
                "[tester] Test verdict: {} (output: {} chars)",
                if is_pass { "PASS ✅" } else { "FAIL ❌" },
                result.answer.len()
            );

            Ok(NodeOutput::Multiple(vec![
                NodeOutput::Custom("test_passed".into(), serde_json::json!(is_pass)),
                NodeOutput::Messages(vec![ConversationMessage::assistant_text(format!("TEST_REPORT:\n{}", result.answer))]),
            ]))
        }
    }));

    // ── Edges ─────────────────────────────────────────────────────────────────
    g.add_edge("planner", "coder");
    g.add_edge("coder", "tester");

    // Tester → PASS: END | FAIL + retries < 3: back to coder | FAIL + exhausted: force END
    g.add_conditional_edge(
        "tester",
        routers::task_test_router("coder", 3),
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
                            Some(Message::assistant_with_tool_calls(content, tool_calls.clone()))
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
