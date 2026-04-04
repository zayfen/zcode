//! Comprehensive LangGraph pipeline for Zcode

use std::sync::Arc;
use crate::agent::graph::graph::StateGraph;
use crate::agent::graph::node::AsyncFnNode;
use crate::agent::graph::state::{DefaultState, NodeOutput};
use crate::agent::graph::edge::routers;
use crate::agent::loop_exec::{AgentLoop, LoopConfig, ConversationMessage, LlmResponse};
use crate::agent::types::AgentState;
use crate::llm::provider::LlmProvider;
use crate::llm::{Message, MessageRole};
use crate::tools::ToolRegistry;
use crate::error::Result;

/// Build the main 4-stage agentic workflow: Planner -> Coder -> Tester -> Reviewer -> (loop back to Coder if failed)
pub fn build_zcode_pipeline(
    provider: Arc<dyn LlmProvider>,
    registry: Arc<ToolRegistry>,
    model: String,
    skills_prompt: String,
) -> StateGraph {
    let mut g = StateGraph::new("planner");

    // Planner Node
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
            let config = LoopConfig {
                max_iterations: 10,
                system_prompt: format!(
                    "You are zcode Planner Agent (Model: {}).\n\
                     Your job is to read the user's task, inspect the codebase using read/search tools, \
                     and formulate a concrete technical plan. \n\
                     Do NOT attempt to write code. Output a step-by-step Execution Plan.\n\n\
                     {}",
                     m, sp
                ),
            };
            
            let mut loop_engine = AgentLoop::new(config, r);
            let result = loop_engine.run(&task, &[], move |msgs, tools| {
                let p2 = Arc::clone(&p);
                async move { call_llm(p2, msgs, tools).await }
            }).await?;
            
            Ok(NodeOutput::Messages(vec![ConversationMessage::assistant_text(format!("PLAN:\n{}", result.answer))]))
        }
    }));

    // Coder Node
    let p2 = Arc::clone(&provider);
    let r2 = Arc::clone(&registry);
    let m2 = model.clone();
    let sp2 = skills_prompt.clone();
    g.add_node(AsyncFnNode::new("coder", move |state| {
        let p = Arc::clone(&p2);
        let r = Arc::clone(&r2);
        let m = m2.clone();
        let sp = sp2.clone();
        
        let task = state.task.clone().map(|t| t.description).unwrap_or_default();
        // Retrieve the plan or feedback from previous nodes
        let last_msg = state.messages.last().and_then(|m| m.content.clone()).unwrap_or_default();
        state.agent_state = AgentState::Executing;
        
        async move {
            // Clear review status whenever coder restarts
            // Since we must return a NodeOutput, we just map it out
            let _ = NodeOutput::Custom("review_passed".into(), serde_json::Value::Null);

            let config = LoopConfig { 
                max_iterations: 20, 
                system_prompt: format!(
                    "You are zcode Coder Agent (Model: {}).\n\
                     Execute the provided technical plan to fulfill the original task. \n\
                     You have full write access to the workspace.\n\n\
                     {}",
                     m, sp
                ) 
            };
            
            let user_prompt = format!("Original Task: {}\n\nInput Context (Plan/Feedback):\n{}", task, last_msg);
            let mut loop_engine = AgentLoop::new(config, r);
            let result = loop_engine.run(&user_prompt, &[], move |msgs, tools| {
                let p_inner = Arc::clone(&p);
                async move { call_llm(p_inner, msgs, tools).await }
            }).await?;
            
            // Return both clearing of review_passed and the new messages
            Ok(NodeOutput::Multiple(vec![
                NodeOutput::Custom("review_passed".into(), serde_json::Value::Null),
                NodeOutput::Messages(vec![ConversationMessage::assistant_text(format!("CODER_REPORT:\n{}", result.answer))])
            ]))
        }
    }));

    // Tester Node
    let p_test = Arc::clone(&provider);
    let r_test = Arc::clone(&registry);
    let m_test = model.clone();
    let sp_test = skills_prompt.clone();
    g.add_node(AsyncFnNode::new("tester", move |state| {
        let p = Arc::clone(&p_test);
        let r = Arc::clone(&r_test);
        let m = m_test.clone();
        let sp = sp_test.clone();
        
        let task = state.task.clone().map(|t| t.description).unwrap_or_default();
        let coder_report = state.messages.last().and_then(|m| m.content.clone()).unwrap_or_default();
        
        state.agent_state = AgentState::Executing;

        async move {
            let config = LoopConfig {
                max_iterations: 15,
                system_prompt: format!(
                    "You are zcode Tester Agent (Model: {}).\n\
                     Your job is to test the code changes made by the Coder against the original task.\n\
                     You must run tests, shell commands, or manually inspect output to verify correctness.\n\
                     Output a Test Report detailing what was checked, the commands run, and the outcomes.\n\n\
                     {}",
                     m, sp
                ),
            };
            
            let user_prompt = format!("Original Task: {}\n\nCoder reported:\n{}\n\nPlease verify these changes by running necessary tests/builds.", task, coder_report);
            let mut loop_engine = AgentLoop::new(config, r);
            let result = loop_engine.run(&user_prompt, &[], move |msgs, tools| {
                let p_inner = Arc::clone(&p);
                async move { call_llm(p_inner, msgs, tools).await }
            }).await?;
            
            Ok(NodeOutput::Messages(vec![ConversationMessage::assistant_text(format!("TEST_REPORT:\n{}", result.answer))]))
        }
    }));

    // Reviewer Node
    let p3 = Arc::clone(&provider);
    let r3 = Arc::clone(&registry);
    let m3 = model.clone();
    let sp3 = skills_prompt.clone();
    g.add_node(AsyncFnNode::new("reviewer", move |state| {
        let p = Arc::clone(&p3);
        let r = Arc::clone(&r3);
        let m = m3.clone();
        let sp = sp3.clone();
        
        let task = state.task.clone().map(|t| t.description).unwrap_or_default();
        let coder_report = state.messages.last().and_then(|m| m.content.clone()).unwrap_or_default();
        
        state.agent_state = AgentState::Reviewing;

        async move {
            let config = LoopConfig {
                max_iterations: 10,
                system_prompt: format!(
                    "You are zcode Reviewer Agent (Model: {}).\n\
                     Inspect the test report and the coder's work against the original task. \n\
                     If everything works and passes, you MUST include the exact word 'PASS' in your final answer. \n\
                     If there are failed tests or missing requirements, list the exact files and lines that need fixing.\n\n\
                     {}",
                     m, sp
                ),
            };
            
            let user_prompt = format!(
                "Task: {}\n\nTester/Coder reported:\n{}\n\nVerify the outcomes. Reply PASS if good, or list required fixes.",
                task, coder_report
            );
            let mut loop_engine = AgentLoop::new(config, r);
            let result = loop_engine.run(&user_prompt, &[], move |msgs, tools| {
                let p_inner = Arc::clone(&p);
                async move { call_llm(p_inner, msgs, tools).await }
            }).await?;
            
            let is_pass = result.answer.contains("PASS");
            
            // Return BOTH metadata flag and message feedback
            Ok(NodeOutput::Multiple(vec![
                NodeOutput::Custom("review_passed".into(), serde_json::json!(is_pass)),
                NodeOutput::Messages(vec![ConversationMessage::assistant_text(format!("REVIEW_FEEDBACK:\n{}", result.answer))])
            ]))
        }
    }));
    
    // Edges
    g.add_edge("planner", "coder");
    g.add_edge("coder", "tester");
    g.add_edge("tester", "reviewer");
    
    g.add_conditional_edge(
        "reviewer",
        routers::review_router("coder"),
        vec!["coder", "__end__"],
    );

    g
}

async fn call_llm(p: Arc<dyn LlmProvider>, msgs: Vec<serde_json::Value>, tools: Vec<serde_json::Value>) -> Result<LlmResponse> {
    let llm_messages: Vec<Message> = msgs.iter()
        .filter_map(|v| {
            let role = v.get("role")?.as_str()?;
            let content = v.get("content")?.as_str().unwrap_or("").to_string();
            let role = match role {
                "system" => MessageRole::System,
                "assistant" => MessageRole::Assistant,
                _ => MessageRole::User,
            };
            Some(Message { role, content })
        })
        .collect();

    match p.chat(&llm_messages, &tools) {
        Ok(resp) => {
            if let Ok(agent_resp) = LlmResponse::from_anthropic_response(&resp.raw_response) {
                Ok(agent_resp)
            } else {
                Ok(LlmResponse::Text(resp.content))
            }
        }
        Err(crate::error::ZcodeError::MissingApiKey(provider)) => {
            Ok(LlmResponse::Text(format!(
                "Task acknowledged. No API key found for provider '{}'. \
                 Set the corresponding env variable to enable LLM responses.",
                provider
            )))
        }
        Err(e) => Err(e),
    }
}
