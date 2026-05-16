//! Ask-user types for agent ↔ TUI communication.

/// A request from the agent to ask the user a clarification question.
///
/// The tool sends this through a shared channel; the TUI renders the options,
/// waits for user selection, then sends the chosen option back via `response_tx`.
pub struct AskRequest {
    pub question: String,
    pub options: Vec<String>,
    pub response_tx: std::sync::mpsc::Sender<String>,
}

/// Shared sender side used by `AskUserTool`.
pub type AskUserSender = std::sync::Arc<std::sync::Mutex<std::sync::mpsc::Sender<AskRequest>>>;
