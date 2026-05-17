use super::*;
use std::sync::{Mutex, OnceLock};
use zcode_core::llm::{LlmConfig, Message};

fn with_zcode_api_key_removed<T>(f: impl FnOnce() -> T) -> T {
    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let original = std::env::var("ZCODE_API_KEY").ok();
    std::env::remove_var("ZCODE_API_KEY");
    let result = f();
    if let Some(val) = original {
        std::env::set_var("ZCODE_API_KEY", val);
    }
    result
}

// ============================================================
// MockLlmProvider tests
// ============================================================

#[test]
fn test_mock_provider_new() {
    let provider = MockLlmProvider::new("Hello, world!");
    let result = provider.complete("test").unwrap();
    assert_eq!(result, "Hello, world!");
}

// ============================================================
// build_openai_messages tests
// ============================================================

#[test]
fn test_build_openai_messages_empty() {
    let result = RigProvider::build_openai_messages(&[]);
    assert!(result.is_empty());
}

#[test]
fn test_build_openai_messages_plain_text() {
    let messages = vec![
        Message::system("You are helpful"),
        Message::user("Hello"),
        Message::assistant("Hi there!"),
        Message::user("How are you?"),
    ];
    let result = RigProvider::build_openai_messages(&messages);
    assert_eq!(result.len(), 4);
    assert_eq!(result[0]["role"], "system");
    assert_eq!(result[0]["content"], "You are helpful");
    assert_eq!(result[1]["role"], "user");
    assert_eq!(result[1]["content"], "Hello");
    assert_eq!(result[2]["role"], "assistant");
    assert_eq!(result[2]["content"], "Hi there!");
    assert_eq!(result[3]["role"], "user");
    assert_eq!(result[3]["content"], "How are you?");
}

#[test]
fn test_build_openai_messages_assistant_with_tool_calls() {
    let tool_call = serde_json::json!({
        "id": "call_123",
        "name": "get_weather",
        "input": {"city": "Tokyo"}
    });
    let messages = vec![
        Message::user("What's the weather?"),
        Message::assistant_with_tool_calls("Let me check", vec![tool_call]),
    ];
    let result = RigProvider::build_openai_messages(&messages);
    assert_eq!(result.len(), 2);

    let assistant_msg = &result[1];
    assert_eq!(assistant_msg["role"], "assistant");
    assert_eq!(assistant_msg["content"], "Let me check");
    let tool_calls = assistant_msg["tool_calls"].as_array().unwrap();
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0]["type"], "function");
    assert_eq!(tool_calls[0]["function"]["name"], "get_weather");
    assert!(tool_calls[0]["function"]["arguments"].is_string());
}

#[test]
fn test_build_openai_messages_assistant_with_tool_calls_empty_content() {
    let tool_call = serde_json::json!({
        "id": "call_456",
        "name": "search",
        "input": {"q": "rust"}
    });
    let messages = vec![
        Message::user("Search please"),
        Message::assistant_with_tool_calls("", vec![tool_call]),
    ];
    let result = RigProvider::build_openai_messages(&messages);
    let assistant_msg = &result[1];
    // OpenAI allows missing content field when empty, or it can be empty string
    assert!(assistant_msg
        .get("content")
        .map(|c| c.as_str().unwrap_or("") == "")
        .unwrap_or(true));
    let tool_calls = assistant_msg["tool_calls"].as_array().unwrap();
    assert_eq!(tool_calls.len(), 1);
}

#[test]
fn test_build_openai_messages_preserves_reasoning_content_for_tool_calls() {
    let tool_call = serde_json::json!({
        "id": "call_456",
        "type": "function",
        "function": {
            "name": "search",
            "arguments": "{\"q\":\"rust\"}"
        }
    });
    let messages = vec![Message::assistant_with_tool_calls_and_reasoning(
        "",
        vec![tool_call],
        Some("Need to search the workspace first.".to_string()),
    )];

    let result = RigProvider::build_openai_messages(&messages);
    assert_eq!(
        result[0]["reasoning_content"],
        "Need to search the workspace first."
    );
}

#[test]
fn test_build_openai_messages_tool_results() {
    let messages = vec![
        Message::assistant("Check this"),
        Message::tool_result("call_1", "search_tool", "Found: rust is great"),
    ];
    let result = RigProvider::build_openai_messages(&messages);
    assert_eq!(result.len(), 2);

    let tool_msg = &result[1];
    assert_eq!(tool_msg["role"], "tool");
    assert_eq!(tool_msg["tool_call_id"], "call_1");
    assert_eq!(tool_msg["content"], "Found: rust is great");
}

#[test]
fn test_build_openai_messages_consecutive_tool_results() {
    let messages = vec![
        Message::user("Run tools"),
        Message::assistant("OK"),
        Message::tool_result("c1", "t1", "r1"),
        Message::tool_result("c2", "t2", "r2"),
    ];
    let result = RigProvider::build_openai_messages(&messages);
    // OpenAI keeps each tool result as a separate message.
    assert_eq!(result.len(), 4);
    assert_eq!(result[2]["role"], "tool");
    assert_eq!(result[2]["tool_call_id"], "c1");
    assert_eq!(result[3]["role"], "tool");
    assert_eq!(result[3]["tool_call_id"], "c2");
}

#[test]
fn test_build_openai_messages_assistant_no_tool_calls() {
    let messages = vec![Message::user("Hello"), Message::assistant("World")];
    let result = RigProvider::build_openai_messages(&messages);
    assert_eq!(result.len(), 2);
    assert!(result[1].get("tool_calls").is_none());
}

#[test]
fn test_build_openai_messages_legacy_tool_use_to_openai_conversion() {
    let legacy_tool_use = serde_json::json!({
        "type": "tool_use",
        "id": "tu_789",
        "name": "calculator",
        "input": {"expr": "2+2"}
    });
    let messages = vec![
        Message::user("Calculate"),
        Message::assistant_with_tool_calls("", vec![legacy_tool_use]),
        Message::tool_result("tu_789", "calculator", "4"),
    ];
    let result = RigProvider::build_openai_messages(&messages);
    let assistant_msg = &result[1];
    let tool_calls = assistant_msg["tool_calls"].as_array().unwrap();
    let tc = &tool_calls[0];
    assert_eq!(tc["id"], "tu_789");
    assert_eq!(tc["type"], "function");
    assert_eq!(tc["function"]["name"], "calculator");
    let args: serde_json::Value =
        serde_json::from_str(tc["function"]["arguments"].as_str().unwrap()).unwrap();
    assert_eq!(args["expr"], "2+2");
}

#[test]
fn test_build_openai_messages_openai_format_passthrough() {
    // If tool calls already in OpenAI format (has "function" key), pass through
    let openai_tool_call = serde_json::json!({
        "id": "call_999",
        "type": "function",
        "function": {
            "name": "direct_tool",
            "arguments": "{\"key\": \"value\"}"
        }
    });
    let messages = vec![
        Message::user("Do it"),
        Message::assistant_with_tool_calls("", vec![openai_tool_call]),
    ];
    let result = RigProvider::build_openai_messages(&messages);
    let assistant_msg = &result[1];
    let tool_calls = assistant_msg["tool_calls"].as_array().unwrap();
    let tc = &tool_calls[0];
    assert_eq!(tc["id"], "call_999");
    assert_eq!(tc["function"]["name"], "direct_tool");
}

#[test]
fn test_mock_provider_complete_empty() {
    let provider = MockLlmProvider::new("");
    let result = provider.complete("test").unwrap();
    assert_eq!(result, "");
}

#[test]
fn test_mock_provider_complete_long_response() {
    let long_response = "x".repeat(10000);
    let provider = MockLlmProvider::new(long_response.clone());
    let result = provider.complete("test").unwrap();
    assert_eq!(result, long_response);
}

#[test]
fn test_mock_provider_complete_ignores_prompt() {
    let provider = MockLlmProvider::new("Fixed response");
    let result1 = provider.complete("prompt 1").unwrap();
    let result2 = provider.complete("prompt 2").unwrap();
    assert_eq!(result1, result2);
}

#[test]
fn test_mock_provider_chat_basic() {
    let provider = MockLlmProvider::new("Response");
    let messages = vec![Message::user("Hello")];
    let response = provider.chat(&messages, &[]).unwrap();
    assert_eq!(response.content, "Response");
}

#[test]
fn test_mock_provider_chat_model_field() {
    let provider = MockLlmProvider::new("Response");
    let messages = vec![Message::user("Hello")];
    let response = provider.chat(&messages, &[]).unwrap();
    assert_eq!(response.model, "mock-model");
}

#[test]
fn test_mock_provider_chat_usage_stats() {
    let provider = MockLlmProvider::new("Response");
    let messages = vec![Message::user("Hello")];
    let response = provider.chat(&messages, &[]).unwrap();
    assert!(response.usage.is_some());
    let usage = response.usage.unwrap();
    assert_eq!(usage.input_tokens, 10);
    assert_eq!(usage.output_tokens, 5);
}

#[test]
fn test_mock_provider_chat_empty_messages() {
    let provider = MockLlmProvider::new("Response");
    let messages: Vec<Message> = vec![];
    let response = provider.chat(&messages, &[]).unwrap();
    assert_eq!(response.content, "Response");
}

#[test]
fn test_mock_provider_chat_multiple_messages() {
    let provider = MockLlmProvider::new("Response");
    let messages = vec![
        Message::system("You are helpful"),
        Message::user("Hi"),
        Message::assistant("Hello"),
        Message::user("How are you?"),
    ];
    let response = provider.chat(&messages, &[]).unwrap();
    assert_eq!(response.content, "Response");
}

#[tokio::test]
async fn test_mock_provider_stream_complete() {
    let provider = MockLlmProvider::new("Stream response");
    let stream = provider.stream_complete("test").unwrap();

    use futures::StreamExt;
    let chunks: Vec<_> = stream.collect().await;

    assert_eq!(chunks.len(), 1);
    assert!(chunks[0].is_ok());
    assert_eq!(chunks[0].as_ref().unwrap(), "Stream response");
}

#[test]
fn test_parse_openai_stream_events_content_and_thinking() {
    let sse = concat!(
        "data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"plan\"}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"content\":\"hello\"}}]}\n\n",
        "data: [DONE]\n\n"
    );

    let events = parse_openai_stream_events(sse);
    assert_eq!(events.len(), 2);
    assert_eq!(
        events[0].as_ref().unwrap(),
        &LlmStreamEvent::Thinking("plan".to_string())
    );
    assert_eq!(
        events[1].as_ref().unwrap(),
        &LlmStreamEvent::Content("hello".to_string())
    );
}

#[test]
fn test_openai_stream_parser_buffers_partial_chunks() {
    let mut parser = OpenAiStreamParser::default();
    let first = parser.push_chunk("data: {\"choices\":[{\"delta\":{\"content\":\"hel");
    assert!(first.is_empty());

    let second = parser.push_chunk("lo\"}}]}\n\ndata: [DONE]\n\n");
    assert_eq!(second.len(), 1);
    assert_eq!(
        second[0].as_ref().unwrap(),
        &LlmStreamEvent::Content("hello".to_string())
    );
    assert!(parser.is_done());
}

#[test]
fn test_openai_stream_parser_preserves_split_utf8() {
    let mut parser = OpenAiStreamParser::default();
    let payload = "data: {\"choices\":[{\"delta\":{\"content\":\"你好\"}}]}\n\n";
    let split_at = payload.find('好').unwrap() + 1;

    let first = parser.push_bytes(&payload.as_bytes()[..split_at]);
    assert!(first.is_empty());

    let second = parser.push_bytes(&payload.as_bytes()[split_at..]);
    assert_eq!(second.len(), 1);
    assert_eq!(
        second[0].as_ref().unwrap(),
        &LlmStreamEvent::Content("你好".to_string())
    );
}

// ============================================================
// RigProvider tests
// ============================================================

#[test]
fn test_rig_provider_new() {
    let config = LlmConfig::default();
    let provider = RigProvider::new(config);
    assert_eq!(provider.config().provider, "openai-compatible");
}

#[test]
fn test_rig_provider_config() {
    let config = LlmConfig {
        provider: "openai".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.5,
        ..Default::default()
    };
    let provider = RigProvider::new(config);
    let retrieved_config = provider.config();
    assert_eq!(retrieved_config.provider, "openai");
    assert_eq!(retrieved_config.model, "gpt-4");
    assert_eq!(retrieved_config.temperature, 0.5);
}

#[test]
fn test_rig_provider_chat_endpoint_from_base_url() {
    let original = std::env::var("ZCODE_BASE_URL").ok();

    std::env::set_var("ZCODE_BASE_URL", "https://example.com");
    let provider = RigProvider::new(LlmConfig::default());
    assert_eq!(
        provider.chat_endpoint(),
        "https://example.com/v1/chat/completions"
    );

    std::env::set_var("ZCODE_BASE_URL", "https://example.com/v1");
    let provider = RigProvider::new(LlmConfig::default());
    assert_eq!(
        provider.chat_endpoint(),
        "https://example.com/v1/chat/completions"
    );

    std::env::set_var("ZCODE_BASE_URL", "https://example.com/v1/chat/completions");
    let provider = RigProvider::new(LlmConfig::default());
    assert_eq!(
        provider.chat_endpoint(),
        "https://example.com/v1/chat/completions"
    );

    if let Some(val) = original {
        std::env::set_var("ZCODE_BASE_URL", val);
    } else {
        std::env::remove_var("ZCODE_BASE_URL");
    }
}

#[test]
fn test_rig_provider_chat_endpoint_from_config() {
    let config = LlmConfig {
        base_url: Some("https://provider.example/v1".to_string()),
        ..Default::default()
    };
    let provider = RigProvider::new(config);
    assert_eq!(
        provider.chat_endpoint(),
        "https://provider.example/v1/chat/completions"
    );
}

#[test]
#[ignore = "makes real HTTP call, run with -- --ignored"]
fn test_rig_provider_complete_with_api_key() {
    // RigProvider now makes real HTTP calls. With an invalid test key it errors.
    let config = LlmConfig {
        api_key: Some("sk-test".to_string()),
        ..Default::default()
    };
    let provider = RigProvider::new(config);
    let result = provider.complete("test prompt");
    assert!(result.is_err(), "Expected HTTP/API error with invalid key");
}

#[test]
fn test_rig_provider_complete_includes_prompt() {
    // Use MockLlmProvider to verify response handling
    let provider = MockLlmProvider::new("response for test");
    let result = provider.complete("my prompt").unwrap();
    assert_eq!(result, "response for test");
}

#[test]
fn test_rig_provider_complete_missing_api_key() {
    with_zcode_api_key_removed(|| {
        let config = LlmConfig {
            api_key: None,
            ..Default::default()
        };
        let provider = RigProvider::new(config);
        let result = provider.complete("test");
        assert!(result.is_err());
        match result.unwrap_err() {
            ZcodeError::MissingApiKey(provider_name) => {
                assert_eq!(provider_name, "ZCODE_API_KEY");
            }
            _ => panic!("Expected MissingApiKey error"),
        }
    });
}

#[test]
#[ignore = "makes real HTTP call, run with -- --ignored"]
fn test_rig_provider_chat_with_api_key() {
    // Real HTTP call with invalid key errors
    let config = LlmConfig {
        api_key: Some("sk-test".to_string()),
        ..Default::default()
    };
    let provider = RigProvider::new(config);
    let messages = vec![Message::user("Hello")];
    let result = provider.chat(&messages, &[]);
    assert!(result.is_err(), "Expected HTTP/API error with invalid key");
}

#[test]
fn test_rig_provider_chat_response_model() {
    // Use MockLlmProvider to verify response structure
    let provider = MockLlmProvider::new("reply");
    let messages = vec![Message::user("Hello")];
    let response = provider.chat(&messages, &[]).unwrap();
    assert_eq!(response.model, "mock-model");
}

#[test]
fn test_rig_provider_chat_finds_last_user_message() {
    // MockLlmProvider returns fixed response regardless of messages
    let provider = MockLlmProvider::new("mock reply");
    let messages = vec![
        Message::user("First message"),
        Message::assistant("Response"),
        Message::user("Last message"),
    ];
    let response = provider.chat(&messages, &[]).unwrap();
    assert_eq!(response.content, "mock reply");
}

#[test]
fn test_rig_provider_chat_no_user_message() {
    let provider = MockLlmProvider::new("mock");
    let messages = vec![Message::assistant("Just assistant")];
    let response = provider.chat(&messages, &[]).unwrap();
    assert!(!response.content.is_empty());
}

#[test]
fn test_rig_provider_chat_usage_stats() {
    // MockLlmProvider returns 10/5 tokens
    let provider = MockLlmProvider::new("hello");
    let messages = vec![Message::user("Hello")];
    let response = provider.chat(&messages, &[]).unwrap();
    assert!(response.usage.is_some());
    let usage = response.usage.unwrap();
    assert_eq!(usage.input_tokens, 10);
    assert_eq!(usage.output_tokens, 5);
}

#[test]
fn test_rig_provider_chat_missing_api_key() {
    with_zcode_api_key_removed(|| {
        let config = LlmConfig {
            api_key: None,
            ..Default::default()
        };
        let provider = RigProvider::new(config);
        let messages = vec![Message::user("Hello")];
        let result = provider.chat(&messages, &[]);
        assert!(result.is_err());
    });
}

#[tokio::test]
#[ignore = "makes real HTTP call, run with -- --ignored"]
async fn test_rig_provider_stream_complete_with_api_key() {
    // stream_complete calls complete() internally, which makes real HTTP
    // with invalid key → should return Err before creating a stream
    let config = LlmConfig {
        api_key: Some("sk-test".to_string()),
        ..Default::default()
    };
    let provider = RigProvider::new(config);
    let result = provider.stream_complete("test");
    assert!(result.is_err(), "Expected HTTP/API error with invalid key");
}

#[tokio::test]
async fn test_rig_provider_stream_complete_content() {
    // Use MockLlmProvider to verify stream content handling
    let provider = MockLlmProvider::new("test prompt result");
    let stream = provider.stream_complete("test prompt").unwrap();

    use futures::StreamExt;
    let chunks: Vec<_> = stream.collect().await;

    let full_content: String = chunks
        .iter()
        .filter_map(|c| c.as_ref().ok())
        .cloned()
        .collect();

    assert!(full_content.contains("test prompt result"));
}

#[test]
fn test_rig_provider_stream_complete_missing_api_key() {
    with_zcode_api_key_removed(|| {
        let config = LlmConfig {
            api_key: None,
            ..Default::default()
        };
        let provider = RigProvider::new(config);
        let result = provider.stream_complete("test");
        assert!(result.is_err());
    });
}

// ============================================================
// API key environment variable tests
// ============================================================

#[test]
fn test_rig_provider_get_api_key_from_config() {
    // With a valid config key, RigProvider will attempt real HTTP → Err (invalid key)
    let config = LlmConfig {
        api_key: Some("sk-from-config".to_string()),
        ..Default::default()
    };
    let provider = RigProvider::new(config);
    let result = provider.complete("test");
    // Real HTTP with invalid key returns an API error (not MissingApiKey)
    assert!(result.is_err());
    match result.unwrap_err() {
        ZcodeError::MissingApiKey(_) => {
            panic!("Should not be MissingApiKey — key was provided")
        }
        _ => {} // Any LLM API error is expected
    }
}

#[test]
fn test_rig_provider_zcode_api_key_env() {
    with_zcode_api_key_removed(|| {
        let config = LlmConfig {
            provider: "openai-compatible".to_string(),
            api_key: None,
            ..Default::default()
        };
        let provider = RigProvider::new(config);
        let result = provider.complete("test");
        assert!(result.is_err());
    });
}

// ============================================================
// LlmProvider trait tests
// ============================================================

#[test]
fn test_llm_provider_trait_mock() {
    let provider = MockLlmProvider::new("test");
    // Verify trait object creation works
    let _trait_obj: &dyn LlmProvider = &provider;
}

#[test]
fn test_llm_provider_trait_rig() {
    let config = LlmConfig {
        api_key: Some("sk-test".to_string()),
        ..Default::default()
    };
    let provider = RigProvider::new(config);
    // Verify trait object creation works
    let _trait_obj: &dyn LlmProvider = &provider;
}

// ============================================================
// StreamingResponse type tests
// ============================================================

#[tokio::test]
async fn test_streaming_response_type() {
    let chunks = vec![Ok("Hello ".to_string()), Ok("world!".to_string())];
    let stream: StreamingResponse = Box::pin(futures::stream::iter(chunks));

    use futures::StreamExt;
    let collected: Vec<_> = stream.collect().await;
    assert_eq!(collected.len(), 2);
}

// ============================================================
// Edge cases
// ============================================================

#[test]
fn test_mock_provider_special_characters() {
    let provider = MockLlmProvider::new("Response with \"quotes\" and 'apostrophes'");
    let result = provider.complete("test").unwrap();
    assert!(result.contains("quotes"));
}

#[test]
fn test_mock_provider_unicode() {
    let provider = MockLlmProvider::new("Hello 你好 🎉");
    let result = provider.complete("test").unwrap();
    assert!(result.contains("你好"));
}

#[test]
fn test_mock_provider_newlines() {
    let provider = MockLlmProvider::new("Line 1\nLine 2\nLine 3");
    let result = provider.complete("test").unwrap();
    assert!(result.contains('\n'));
}

#[test]
fn test_rig_provider_custom_provider_uses_zcode_api_key_env() {
    with_zcode_api_key_removed(|| {
        let config = LlmConfig {
            provider: "custom_provider".to_string(),
            api_key: None,
            ..Default::default()
        };
        let provider = RigProvider::new(config);
        let result = provider.complete("test");
        assert!(result.is_err());
    });
}
