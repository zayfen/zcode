use crate::agent::loop_exec::LlmResponse;
use std::sync::Arc;
use zcode_core::Result;
use zcode_llm_provider::provider::LlmProvider;
use zcode_llm_provider::Message;

pub(super) async fn call_llm(
    provider: Arc<dyn LlmProvider>,
    msgs: Vec<serde_json::Value>,
    tools: Vec<serde_json::Value>,
) -> Result<LlmResponse> {
    let llm_messages: Vec<Message> = msgs
        .iter()
        .filter_map(|value| provider_message_from_value(value))
        .collect();

    match provider.chat(&llm_messages, &tools) {
        Ok(resp) => LlmResponse::from_openai_response(&resp.raw_response)
            .or_else(|_| Ok(LlmResponse::Text(resp.content))),
        Err(zcode_core::ZcodeError::MissingApiKey(provider)) => Ok(LlmResponse::Text(format!(
            "Task acknowledged. No API key found for '{}'. \
                 Set ZCODE_API_KEY to enable LLM responses.",
            provider
        ))),
        Err(error) => Err(error),
    }
}

fn provider_message_from_value(value: &serde_json::Value) -> Option<Message> {
    let role = value.get("role")?.as_str()?;
    let content = value
        .get("content")
        .and_then(|content| content.as_str())
        .unwrap_or("")
        .to_string();

    match role {
        "system" => Some(Message::system(content)),
        "assistant" => Some(assistant_message_from_value(value, content)),
        "tool" => Some(Message::tool_result(
            value
                .get("tool_call_id")
                .and_then(|id| id.as_str())
                .unwrap_or("")
                .to_string(),
            value
                .get("name")
                .and_then(|name| name.as_str())
                .unwrap_or("")
                .to_string(),
            content,
        )),
        _ => Some(Message::user(content)),
    }
}

fn assistant_message_from_value(value: &serde_json::Value, content: String) -> Message {
    let Some(tool_calls) = value.get("tool_calls").and_then(|calls| calls.as_array()) else {
        return Message::assistant(content);
    };
    if tool_calls.is_empty() {
        return Message::assistant(content);
    }

    let reasoning_content = value
        .get("reasoning_content")
        .and_then(|content| content.as_str())
        .map(str::to_string);
    Message::assistant_with_tool_calls_and_reasoning(content, tool_calls.clone(), reasoning_content)
}
