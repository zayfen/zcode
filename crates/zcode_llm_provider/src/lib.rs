//! OpenAI-compatible LLM provider layer.

pub mod provider;
pub mod streaming;

pub use provider::{
    ChatStreamingResponse, LlmProvider, LlmStreamEvent, MockLlmProvider, RigProvider,
};
pub use streaming::{StreamHandler, StreamingResponse};
pub use zcode_core::llm::{LlmConfig, LlmResponse, Message, MessageRole, UsageStats};
