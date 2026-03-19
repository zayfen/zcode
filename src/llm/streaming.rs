//! Streaming support for LLM responses
//!
//! This module provides streaming response handling for real-time display.

use futures::Stream;
use std::pin::Pin;

/// Type alias for boxed streaming response
pub type StreamingResponse = Pin<Box<dyn Stream<Item = crate::error::Result<String>> + Send>>;

/// Trait for handling streaming responses
pub trait StreamHandler: Send + Sync {
    /// Called when a new chunk is received
    fn on_chunk(&mut self, chunk: &str);

    /// Called when streaming is complete
    fn on_complete(&mut self);

    /// Called when an error occurs
    fn on_error(&mut self, error: &crate::error::ZcodeError);
}

/// Default stream handler that collects chunks into a string
#[derive(Debug, Default)]
pub struct CollectingHandler {
    /// Collected content
    pub content: String,
    /// Whether streaming is complete
    pub complete: bool,
    /// Last error if any
    pub error: Option<String>,
}

impl CollectingHandler {
    /// Create a new collecting handler
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the collected content
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Check if streaming is complete
    pub fn is_complete(&self) -> bool {
        self.complete
    }
}

impl StreamHandler for CollectingHandler {
    fn on_chunk(&mut self, chunk: &str) {
        self.content.push_str(chunk);
    }

    fn on_complete(&mut self) {
        self.complete = true;
    }

    fn on_error(&mut self, error: &crate::error::ZcodeError) {
        self.error = Some(error.to_string());
    }
}

/// Callback-based stream handler
pub struct CallbackHandler<F>
where
    F: FnMut(&str) + Send + Sync,
{
    callback: F,
}

impl<F> CallbackHandler<F>
where
    F: FnMut(&str) + Send + Sync,
{
    /// Create a new callback handler
    pub fn new(callback: F) -> Self {
        Self { callback }
    }
}

impl<F> StreamHandler for CallbackHandler<F>
where
    F: FnMut(&str) + Send + Sync,
{
    fn on_chunk(&mut self, chunk: &str) {
        (self.callback)(chunk);
    }

    fn on_complete(&mut self) {
        // No-op by default
        let _ = self;
    }

    fn on_error(&mut self, error: &crate::error::ZcodeError) {
        // Log error by default
        eprintln!("Stream error: {}", error);
    }
}

/// Process a streaming response with a handler
pub async fn process_stream(
    mut stream: StreamingResponse,
    handler: &mut dyn StreamHandler,
) -> crate::error::Result<String> {
    use futures::StreamExt;

    let mut content = String::new();

    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                content.push_str(&chunk);
                handler.on_chunk(&chunk);
            }
            Err(e) => {
                handler.on_error(&e);
                return Err(e);
            }
        }
    }

    handler.on_complete();
    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ZcodeError;

    #[test]
    fn test_collecting_handler() {
        let mut handler = CollectingHandler::new();
        handler.on_chunk("Hello ");
        handler.on_chunk("world!");
        handler.on_complete();

        assert_eq!(handler.content(), "Hello world!");
        assert!(handler.is_complete());
        assert!(handler.error.is_none());
    }

    #[test]
    fn test_collecting_handler_error() {
        let mut handler = CollectingHandler::new();
        handler.on_chunk("Partial");
        handler.on_error(&ZcodeError::LlmApiError("Connection lost".to_string()));

        assert_eq!(handler.content(), "Partial");
        assert!(!handler.is_complete());
        assert!(handler.error.is_some());
    }

    #[test]
    fn test_callback_handler() {
        let mut collected = String::new();
        {
            let mut handler = CallbackHandler::new(|chunk: &str| {
                collected.push_str(chunk);
            });
            handler.on_chunk("Test");
            handler.on_chunk(" content");
        }
        assert_eq!(collected, "Test content");
    }

    #[tokio::test]
    async fn test_process_stream() {
        let stream = futures::stream::iter(vec![
            Ok("Hello ".to_string()),
            Ok("world!".to_string()),
        ]);
        let boxed_stream: StreamingResponse = Box::pin(stream);

        let mut handler = CollectingHandler::new();
        let result = process_stream(boxed_stream, &mut handler).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello world!");
        assert!(handler.is_complete());
    }
}
