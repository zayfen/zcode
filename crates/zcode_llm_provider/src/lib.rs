//! OpenAI-compatible LLM provider layer.

pub mod provider;
pub mod streaming;

pub use provider::{LlmProvider, MockLlmProvider, RigProvider};
pub use streaming::{StreamHandler, StreamingResponse};
pub use zcode_core::llm::{LlmConfig, LlmResponse, Message, MessageRole, UsageStats};
