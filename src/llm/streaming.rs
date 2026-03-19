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

    // ============================================================
    // CollectingHandler tests
    // ============================================================

    #[test]
    fn test_collecting_handler_new() {
        let handler = CollectingHandler::new();
        assert!(handler.content().is_empty());
        assert!(!handler.is_complete());
        assert!(handler.error.is_none());
    }

    #[test]
    fn test_collecting_handler_default() {
        let handler = CollectingHandler::default();
        assert!(handler.content().is_empty());
        assert!(!handler.is_complete());
    }

    #[test]
    fn test_collecting_handler_single_chunk() {
        let mut handler = CollectingHandler::new();
        handler.on_chunk("Hello");
        assert_eq!(handler.content(), "Hello");
    }

    #[test]
    fn test_collecting_handler_multiple_chunks() {
        let mut handler = CollectingHandler::new();
        handler.on_chunk("Hello ");
        handler.on_chunk("world!");
        assert_eq!(handler.content(), "Hello world!");
    }

    #[test]
    fn test_collecting_handler_empty_chunk() {
        let mut handler = CollectingHandler::new();
        handler.on_chunk("Hello");
        handler.on_chunk("");
        handler.on_chunk(" world");
        assert_eq!(handler.content(), "Hello world");
    }

    #[test]
    fn test_collecting_handler_complete() {
        let mut handler = CollectingHandler::new();
        handler.on_chunk("Test");
        handler.on_complete();

        assert!(handler.is_complete());
        assert_eq!(handler.content(), "Test");
    }

    #[test]
    fn test_collecting_handler_error() {
        let mut handler = CollectingHandler::new();
        handler.on_chunk("Partial");
        handler.on_error(&ZcodeError::LlmApiError("Connection lost".to_string()));

        assert_eq!(handler.content(), "Partial");
        assert!(!handler.is_complete());
        assert!(handler.error.is_some());
        assert!(handler.error.unwrap().contains("Connection lost"));
    }

    #[test]
    fn test_collecting_handler_error_types() {
        // Test with different error types
        let mut handler = CollectingHandler::new();

        handler.on_error(&ZcodeError::ToolNotFound { name: "test".into() });
        assert!(handler.error.is_some());

        handler.error = None;
        handler.on_error(&ZcodeError::ConfigError("test".into()));
        assert!(handler.error.is_some());

        handler.error = None;
        handler.on_error(&ZcodeError::Cancelled);
        assert!(handler.error.is_some());
    }

    #[test]
    fn test_collecting_handler_content_method() {
        let mut handler = CollectingHandler::new();
        handler.on_chunk("Test content");
        assert_eq!(handler.content(), "Test content");
    }

    #[test]
    fn test_collecting_handler_is_complete_method() {
        let handler = CollectingHandler::new();
        assert!(!handler.is_complete());

        let mut handler = CollectingHandler::new();
        handler.on_complete();
        assert!(handler.is_complete());
    }

    #[test]
    fn test_collecting_handler_debug() {
        let handler = CollectingHandler::new();
        let debug_str = format!("{:?}", handler);
        assert!(debug_str.contains("CollectingHandler"));
    }

    #[test]
    fn test_collecting_handler_unicode() {
        let mut handler = CollectingHandler::new();
        handler.on_chunk("Hello ");
        handler.on_chunk("你好 ");
        handler.on_chunk("🎉");
        assert_eq!(handler.content(), "Hello 你好 🎉");
    }

    #[test]
    fn test_collecting_handler_newlines() {
        let mut handler = CollectingHandler::new();
        handler.on_chunk("Line 1\n");
        handler.on_chunk("Line 2\n");
        handler.on_chunk("Line 3");
        assert_eq!(handler.content(), "Line 1\nLine 2\nLine 3");
    }

    #[test]
    fn test_collecting_handler_large_content() {
        let mut handler = CollectingHandler::new();
        for i in 0..1000 {
            handler.on_chunk(&format!("Chunk {} ", i));
        }
        assert!(handler.content().contains("Chunk 999"));
    }

    // ============================================================
    // CallbackHandler tests
    // ============================================================

    #[test]
    fn test_callback_handler_new() {
        // Just verify CallbackHandler can be created
        let _handler = CallbackHandler::new(|_chunk: &str| {});
    }

    #[test]
    fn test_callback_handler_multiple_chunks() {
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

    #[test]
    fn test_callback_handler_on_complete() {
        let mut handler = CallbackHandler::new(|_chunk: &str| {});
        handler.on_complete();
        // on_complete is a no-op for CallbackHandler, just verify it doesn't panic
    }

    #[test]
    fn test_callback_handler_on_error() {
        // CallbackHandler prints to stderr on error, we just verify it doesn't panic
        let mut handler = CallbackHandler::new(|_chunk: &str| {});
        handler.on_error(&ZcodeError::LlmApiError("test error".to_string()));
        // Should not panic
    }

    #[test]
    fn test_callback_handler_empty_chunks() {
        let mut collected = String::new();
        {
            let mut handler = CallbackHandler::new(|chunk: &str| {
                collected.push_str(chunk);
            });
            handler.on_chunk("");
            handler.on_chunk("a");
            handler.on_chunk("");
        }
        assert_eq!(collected, "a");
    }

    #[test]
    fn test_callback_handler_counter() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let count = Arc::new(AtomicUsize::new(0));
        let count_clone = Arc::clone(&count);

        let mut handler = CallbackHandler::new(move |_chunk: &str| {
            count_clone.fetch_add(1, Ordering::SeqCst);
        });

        handler.on_chunk("a");
        handler.on_chunk("b");
        handler.on_chunk("c");

        assert_eq!(count.load(Ordering::SeqCst), 3);
    }

    // ============================================================
    // process_stream tests
    // ============================================================

    #[tokio::test]
    async fn test_process_stream_empty() {
        let stream = futures::stream::iter(vec![]);
        let boxed_stream: StreamingResponse = Box::pin(stream);

        let mut handler = CollectingHandler::new();
        let result = process_stream(boxed_stream, &mut handler).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "");
        assert!(handler.is_complete());
    }

    #[tokio::test]
    async fn test_process_stream_single_chunk() {
        let stream = futures::stream::iter(vec![Ok("Hello".to_string())]);
        let boxed_stream: StreamingResponse = Box::pin(stream);

        let mut handler = CollectingHandler::new();
        let result = process_stream(boxed_stream, &mut handler).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello");
        assert!(handler.is_complete());
    }

    #[tokio::test]
    async fn test_process_stream_multiple_chunks() {
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

    #[tokio::test]
    async fn test_process_stream_with_error() {
        let stream = futures::stream::iter(vec![
            Ok("Hello ".to_string()),
            Err(ZcodeError::LlmApiError("Stream error".to_string())),
        ]);
        let boxed_stream: StreamingResponse = Box::pin(stream);

        let mut handler = CollectingHandler::new();
        let result = process_stream(boxed_stream, &mut handler).await;

        assert!(result.is_err());
        assert!(!handler.is_complete());
        assert!(handler.error.is_some());
        assert_eq!(handler.content(), "Hello ");
    }

    #[tokio::test]
    async fn test_process_stream_error_first() {
        let stream = futures::stream::iter(vec![
            Err(ZcodeError::LlmApiError("Immediate error".to_string())),
            Ok("Never reached".to_string()),
        ]);
        let boxed_stream: StreamingResponse = Box::pin(stream);

        let mut handler = CollectingHandler::new();
        let result = process_stream(boxed_stream, &mut handler).await;

        assert!(result.is_err());
        assert!(handler.content().is_empty());
    }

    #[tokio::test]
    async fn test_process_stream_calls_handler_on_chunk() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let count = Arc::new(AtomicUsize::new(0));
        let count_clone = Arc::clone(&count);

        struct CountingHandler {
            count: Arc<AtomicUsize>,
        }

        impl StreamHandler for CountingHandler {
            fn on_chunk(&mut self, _chunk: &str) {
                self.count.fetch_add(1, Ordering::SeqCst);
            }
            fn on_complete(&mut self) {}
            fn on_error(&mut self, _error: &ZcodeError) {}
        }

        let stream = futures::stream::iter(vec![
            Ok("a".to_string()),
            Ok("b".to_string()),
            Ok("c".to_string()),
        ]);
        let boxed_stream: StreamingResponse = Box::pin(stream);

        let mut handler = CountingHandler { count: count_clone };
        let _ = process_stream(boxed_stream, &mut handler).await;

        assert_eq!(count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_process_stream_calls_handler_on_complete() {
        struct CompleteHandler {
            completed: bool,
        }

        impl StreamHandler for CompleteHandler {
            fn on_chunk(&mut self, _chunk: &str) {}
            fn on_complete(&mut self) {
                self.completed = true;
            }
            fn on_error(&mut self, _error: &ZcodeError) {}
        }

        let stream = futures::stream::iter(vec![Ok("test".to_string())]);
        let boxed_stream: StreamingResponse = Box::pin(stream);

        let mut handler = CompleteHandler { completed: false };
        let _ = process_stream(boxed_stream, &mut handler).await;

        assert!(handler.completed);
    }

    #[tokio::test]
    async fn test_process_stream_unicode() {
        let stream = futures::stream::iter(vec![
            Ok("Hello ".to_string()),
            Ok("你好 ".to_string()),
            Ok("🎉".to_string()),
        ]);
        let boxed_stream: StreamingResponse = Box::pin(stream);

        let mut handler = CollectingHandler::new();
        let result = process_stream(boxed_stream, &mut handler).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello 你好 🎉");
    }

    #[tokio::test]
    async fn test_process_stream_large_chunks() {
        let large_chunk = "x".repeat(10000);
        let stream = futures::stream::iter(vec![
            Ok(large_chunk.clone()),
            Ok(large_chunk.clone()),
        ]);
        let boxed_stream: StreamingResponse = Box::pin(stream);

        let mut handler = CollectingHandler::new();
        let result = process_stream(boxed_stream, &mut handler).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 20000);
    }

    // ============================================================
    // StreamHandler trait tests
    // ============================================================

    #[test]
    fn test_stream_handler_trait_collecting() {
        let mut handler = CollectingHandler::new();
        handler.on_chunk("test");
        handler.on_complete();
        handler.on_error(&ZcodeError::Cancelled);
    }

    #[test]
    fn test_stream_handler_trait_callback() {
        let mut handler = CallbackHandler::new(|_| {});
        handler.on_chunk("test");
        handler.on_complete();
        handler.on_error(&ZcodeError::Cancelled);
    }

    // ============================================================
    // StreamingResponse type tests
    // ============================================================

    #[tokio::test]
    async fn test_streaming_response_type() {
        let chunks = vec![
            Ok("a".to_string()),
            Ok("b".to_string()),
            Ok("c".to_string()),
        ];
        let stream: StreamingResponse = Box::pin(futures::stream::iter(chunks));

        use futures::StreamExt;
        let collected: Vec<_> = stream.collect().await;
        assert_eq!(collected.len(), 3);
    }
}
