//! TUI module for zcode
//!
//! This module provides a terminal user interface with chat capabilities using ratatui.

pub mod chat;
mod markdown;

pub use chat::{ChatInterface, PendingAsk};

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::collections::{HashMap, VecDeque};
use std::io::{self, Stdout};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use zcode_core::agent::ConversationMessage;
use zcode_core::ZcodeError;
use zcode_session::{ContextPolicy, Session, SessionManager, SessionTurn};

/// Type alias for the terminal backend
pub type TuiBackend = CrosstermBackend<Stdout>;

/// Type alias for the terminal
pub type TuiTerminal = Terminal<TuiBackend>;

/// Event emitted by an agentic task executor for the TUI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskUiEvent {
    /// A named agent started work.
    AgentStart(String),
    /// A named agent finished work.
    AgentComplete(String),
    /// Progress/thinking text to show in the collapsible thinking stream.
    Thinking(String),
    /// A supervisor task step started.
    StepStart {
        id: String,
        title: String,
        agent: String,
    },
    /// A supervisor task step completed.
    StepComplete {
        id: String,
        title: String,
        agent: String,
        success: bool,
    },
    /// A tool started inside a named agent.
    ToolStart {
        agent: String,
        tool_name: String,
        command: String,
    },
    /// A tool completed inside a named agent.
    ToolComplete {
        agent: String,
        tool_name: String,
        success: bool,
    },
    /// Final assistant-visible response.
    Done(String),
    /// Task was interrupted.
    Cancelled,
    /// Task failed.
    Error(String),
}

/// Executes a user prompt as an agentic coding task.
#[derive(Debug, Clone)]
pub struct TaskRequest {
    /// Current user prompt to execute.
    pub prompt: String,
    /// User-visible conversation context from previous turns.
    pub history: Vec<ConversationMessage>,
}

impl TaskRequest {
    pub fn new(prompt: impl Into<String>, history: Vec<ConversationMessage>) -> Self {
        Self {
            prompt: prompt.into(),
            history,
        }
    }
}

/// Executes a user prompt as an agentic coding task.
pub type TaskExecutor =
    Arc<dyn Fn(TaskRequest, Arc<AtomicBool>, mpsc::Sender<TaskUiEvent>) + Send + Sync>;

/// Initialize the terminal for TUI mode
pub fn init_terminal() -> zcode_core::Result<TuiTerminal> {
    enable_raw_mode()
        .map_err(|e| ZcodeError::InternalError(format!("Failed to enable raw mode: {}", e)))?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).map_err(|e| {
        ZcodeError::InternalError(format!("Failed to enter alternate screen: {}", e))
    })?;

    // Try to enable keyboard enhancement protocol (kitty protocol).
    // This lets terminals like iTerm2/Ghostty send Shift+Enter as a distinct event.
    // If unsupported, silently skip — Alt+Enter / Ctrl+J are always available.
    let _ = execute!(
        stdout,
        crossterm::event::PushKeyboardEnhancementFlags(
            crossterm::event::KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
        )
    );

    let backend = CrosstermBackend::new(io::stdout());
    Terminal::new(backend)
        .map_err(|e| ZcodeError::InternalError(format!("Failed to create terminal: {}", e)))
}

/// Restore the terminal to normal mode
pub fn restore_terminal(terminal: &mut TuiTerminal) -> zcode_core::Result<()> {
    disable_raw_mode()
        .map_err(|e| ZcodeError::InternalError(format!("Failed to disable raw mode: {}", e)))?;

    // Pop keyboard enhancement flags if we pushed them
    let _ = execute!(
        terminal.backend_mut(),
        crossterm::event::PopKeyboardEnhancementFlags
    );

    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .map_err(|e| ZcodeError::InternalError(format!("Failed to leave alternate screen: {}", e)))?;

    terminal
        .show_cursor()
        .map_err(|e| ZcodeError::InternalError(format!("Failed to show cursor: {}", e)))
}

/// TUI application state
pub struct TuiApp {
    /// Whether the app should quit
    pub should_quit: bool,
    /// Chat interface
    pub chat: ChatInterface,
    /// Agentic task executor (None in test/no-api mode)
    task_executor: Option<TaskExecutor>,
    /// System prompt
    pub system_prompt: String,
    /// Current agents statuses [(Name, Status)]
    pub agent_statuses: Vec<(String, String)>,
    /// Currently running supervisor step title by display agent name.
    active_steps: HashMap<String, String>,
    /// Loaded skill names
    pub active_skills: Vec<String>,
    /// Loaded MCP server names
    pub active_mcps: Vec<String>,
    /// Prompts queued while an LLM response is in flight.
    pending_prompts: VecDeque<String>,
    /// Whether a streaming LLM response is in flight.
    llm_in_flight: bool,
    /// Task event receiver from the worker thread.
    task_rx: Option<Receiver<TaskUiEvent>>,
    /// Cancellation flag for the active task worker.
    cancel_flag: Option<Arc<AtomicBool>>,
    /// Optional session manager for slash commands.
    session_manager: Option<SessionManager>,
    /// Project root used to lazily initialize session storage.
    project_root: PathBuf,
    /// Current session id, if the visible chat came from a saved session.
    current_session_id: Option<String>,
    /// Current in-flight user prompt, saved with the assistant response when it completes.
    active_user_prompt: Option<String>,
    /// Receiver for ask-user requests from the agent worker thread.
    ask_rx: Option<std::sync::mpsc::Receiver<zcode_core::AskRequest>>,
}

impl TuiApp {
    /// Create a new TUI application without an LLM provider
    pub fn new() -> Self {
        Self {
            should_quit: false,
            chat: ChatInterface::new(),
            task_executor: None,
            system_prompt: "You are zcode, a helpful AI coding agent. \
                Use your knowledge to assist with code, architecture, and development tasks."
                .to_string(),
            agent_statuses: vec![
                ("Supervisor".to_string(), "Idle".to_string()),
                ("Planner".to_string(), "Idle".to_string()),
                ("Coder".to_string(), "Idle".to_string()),
                ("Reviewer".to_string(), "Idle".to_string()),
                ("Investigator".to_string(), "Idle".to_string()),
            ],
            active_steps: HashMap::new(),
            active_skills: Vec::new(),
            active_mcps: Vec::new(),
            pending_prompts: VecDeque::new(),
            llm_in_flight: false,
            task_rx: None,
            cancel_flag: None,
            session_manager: None,
            project_root: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            current_session_id: None,
            active_user_prompt: None,
            ask_rx: None,
        }
    }

    /// Create a TUI application with an agentic task executor.
    pub fn with_task_executor(task_executor: TaskExecutor) -> Self {
        let mut app = Self::new();
        app.task_executor = Some(task_executor);
        app.chat.add_message(chat::ChatMessage::system(
            "zcode agent ready. Connected to orchestrator graph.",
        ));
        app
    }

    /// Set the ask-user channel receiver for agent clarification requests.
    pub fn set_ask_receiver(&mut self, rx: std::sync::mpsc::Receiver<zcode_core::AskRequest>) {
        self.ask_rx = Some(rx);
    }

    /// Compatibility constructor used by tests and callers that want a simple
    /// text-only executor.
    pub fn with_provider(provider: Arc<dyn zcode_llm_provider::LlmProvider>) -> Self {
        let executor: TaskExecutor = Arc::new(move |request, cancel, tx| {
            if cancel.load(Ordering::SeqCst) {
                let _ = tx.send(TaskUiEvent::Cancelled);
                return;
            }
            let _ = tx.send(TaskUiEvent::AgentStart("orchestrator".to_string()));
            let mut messages = vec![zcode_llm_provider::Message::system(
                "You are zcode, a helpful AI coding agent. Use prior conversation only as context. Answer the current user task directly, and do not repeat unrelated previous results.",
            )];
            messages.extend(
                request
                    .history
                    .iter()
                    .filter_map(provider_message_from_history),
            );
            messages.push(zcode_llm_provider::Message::user(request.prompt));
            match provider.chat(&messages, &[]) {
                Ok(response) => {
                    let _ = tx.send(TaskUiEvent::AgentComplete("orchestrator".to_string()));
                    let _ = tx.send(TaskUiEvent::Done(response.content));
                }
                Err(error) => {
                    let _ = tx.send(TaskUiEvent::Error(error.to_string()));
                }
            }
        });
        Self::with_task_executor(executor)
    }

    /// Handle a terminal event
    pub fn handle_event(&mut self, event: Event) -> zcode_core::Result<()> {
        match event {
            Event::Key(key) => {
                // Ask-mode key handling takes priority when a question is pending
                if self.chat.pending_ask.is_some() {
                    self.handle_ask_key(key.modifiers, key.code);
                    return Ok(());
                }
                match (key.modifiers, key.code) {
                    // Quit
                    (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                        self.should_quit = true;
                    }
                    (KeyModifiers::NONE, KeyCode::Esc) => {
                        if self.llm_in_flight {
                            self.cancel_active_stream();
                        } else {
                            self.should_quit = true;
                        }
                    }
                    // --- Newline insertion ---
                    // Shift+Enter (requires keyboard enhancement / kitty protocol)
                    (KeyModifiers::SHIFT, KeyCode::Enter) => {
                        self.chat.input_newline();
                    }
                    // Alt+Enter — works in most terminals without keyboard enhancement
                    (KeyModifiers::ALT, KeyCode::Enter) => {
                        self.chat.input_newline();
                    }
                    // Ctrl+J — fallback for terminals that map Ctrl+J to \n
                    (KeyModifiers::CONTROL, KeyCode::Char('j')) => {
                        self.chat.input_newline();
                    }
                    (KeyModifiers::CONTROL, KeyCode::Char('o')) => {
                        self.chat.toggle_thinking();
                    }
                    // Ctrl+Enter is sometimes sent as Ctrl+M
                    (KeyModifiers::CONTROL, KeyCode::Enter) => {
                        self.chat.input_newline();
                    }
                    // Plain Enter: send message
                    (KeyModifiers::NONE, KeyCode::Enter) => {
                        if let Some(user_text) = self.chat.take_current_input() {
                            if user_text.trim_start().starts_with('/') {
                                self.handle_slash_command(&user_text);
                            } else if self.task_executor.is_some() {
                                self.pending_prompts.push_back(user_text);
                                self.chat.pending_count = self.pending_prompts.len();
                                self.start_next_prompt_if_idle();
                            } else {
                                self.chat.add_message(chat::ChatMessage::user(user_text));
                                self.chat.add_message(chat::ChatMessage::assistant(
                                    "⚠ No LLM provider configured. \
                                Set ZCODE_API_KEY environment variable and restart.",
                                ));
                            }
                        }
                    }
                    // --- Cursor movement ---
                    (KeyModifiers::NONE, KeyCode::Left) => {
                        self.chat.cursor_left();
                    }
                    (KeyModifiers::NONE, KeyCode::Right) => {
                        self.chat.cursor_right();
                    }
                    // --- Typing ---
                    (KeyModifiers::NONE, KeyCode::Char(c))
                    | (KeyModifiers::SHIFT, KeyCode::Char(c)) => {
                        self.chat.input_char(c);
                    }
                    (KeyModifiers::NONE, KeyCode::Backspace)
                    | (KeyModifiers::SHIFT, KeyCode::Backspace) => {
                        self.chat.backspace();
                    }
                    (KeyModifiers::NONE, KeyCode::Up) => {
                        self.chat.scroll_up();
                    }
                    (KeyModifiers::NONE, KeyCode::Down) => {
                        self.chat.scroll_down();
                    }
                    _ => {}
                }
            }
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollUp => self.chat.scroll_up(),
                MouseEventKind::ScrollDown => self.chat.scroll_down(),
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }

    /// Run the main event loop.
    pub fn run(&mut self, terminal: &mut TuiTerminal) -> zcode_core::Result<()> {
        while !self.should_quit {
            self.drain_task_events();
            self.drain_ask_requests();
            self.start_next_prompt_if_idle();
            self.chat.tick_loading();

            terminal
                .draw(|f| {
                    self.chat.render(
                        f,
                        &self.agent_statuses,
                        &self.active_skills,
                        &self.active_mcps,
                    )
                })
                .map_err(|e| ZcodeError::InternalError(format!("Failed to draw: {}", e)))?;

            if event::poll(std::time::Duration::from_millis(100))
                .map_err(|e| ZcodeError::InternalError(format!("Poll error: {}", e)))?
            {
                let event = event::read()
                    .map_err(|e| ZcodeError::InternalError(format!("Read error: {}", e)))?;
                self.handle_event(event)?;
            }
        }
        Ok(())
    }

    fn start_next_prompt_if_idle(&mut self) {
        if self.llm_in_flight {
            return;
        }
        let Some(user_text) = self.pending_prompts.pop_front() else {
            self.chat.pending_count = 0;
            return;
        };
        let Some(task_executor) = self.task_executor.as_ref().cloned() else {
            return;
        };

        self.ensure_current_session_id();
        let history = self.relevant_conversation_history(&user_text);
        self.chat
            .add_message(chat::ChatMessage::user(user_text.clone()));
        self.active_user_prompt = Some(user_text.clone());
        self.chat.pending_count = self.pending_prompts.len();
        self.chat.start_loading();
        if let Some(agent) = self.agent_statuses.iter_mut().find(|a| a.0 == "Supervisor") {
            agent.1 = "Thinking...".to_string();
        }
        self.llm_in_flight = true;

        let (tx, rx) = mpsc::channel();
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let worker_cancel_flag = Arc::clone(&cancel_flag);
        self.cancel_flag = Some(cancel_flag);
        self.task_rx = Some(rx);

        std::thread::spawn(move || {
            task_executor(
                TaskRequest::new(user_text, history),
                Arc::clone(&worker_cancel_flag),
                tx.clone(),
            );
            if worker_cancel_flag.load(Ordering::SeqCst) {
                let _ = tx.send(TaskUiEvent::Cancelled);
            }
        });
    }

    fn drain_task_events(&mut self) {
        let Some(rx) = self.task_rx.take() else {
            return;
        };
        let mut keep_rx = true;
        loop {
            match rx.try_recv() {
                Ok(TaskUiEvent::AgentStart(agent)) => {
                    if !is_internal_graph_node(&agent) {
                        self.set_agent_status(&agent, "Working...");
                    }
                    self.chat.append_thinking(&format!("{} started\n", agent));
                }
                Ok(TaskUiEvent::AgentComplete(agent)) => {
                    if !is_internal_graph_node(&agent) {
                        self.set_agent_status(&agent, "Done");
                    }
                    self.chat.append_thinking(&format!("{} completed\n", agent));
                }
                Ok(TaskUiEvent::Thinking(text)) => {
                    self.chat.append_thinking(&text);
                }
                Ok(TaskUiEvent::StepStart { title, agent, .. }) => {
                    self.set_active_step(&agent, &title);
                    self.set_agent_status("supervisor", &title);
                    self.set_agent_status(&agent, &title);
                    self.chat
                        .append_thinking(&format!("{} step started: {}\n", agent, title));
                }
                Ok(TaskUiEvent::StepComplete {
                    title,
                    agent,
                    success,
                    ..
                }) => {
                    self.clear_active_step(&agent);
                    self.set_agent_status(&agent, if success { "Done" } else { "Step failed" });
                    self.chat.append_thinking(&format!(
                        "{} step {}: {}\n",
                        agent,
                        if success { "completed" } else { "failed" },
                        title
                    ));
                }
                Ok(TaskUiEvent::ToolStart {
                    agent,
                    tool_name,
                    command,
                }) => {
                    self.set_agent_status(&agent, &format!("{}: {}", tool_name, command));
                    self.chat
                        .append_thinking(&format!("{} {}: {}\n", agent, tool_name, command));
                }
                Ok(TaskUiEvent::ToolComplete {
                    agent,
                    tool_name,
                    success,
                }) => {
                    let status = if success {
                        self.active_step_status(&agent)
                            .unwrap_or_else(|| "Working...".to_string())
                    } else {
                        "Tool failed".to_string()
                    };
                    self.set_agent_status(&agent, &status);
                    self.chat.append_thinking(&format!(
                        "{} {} {}\n",
                        agent,
                        tool_name,
                        if success { "succeeded" } else { "failed" }
                    ));
                }
                Ok(TaskUiEvent::Done(text)) => {
                    if !text.trim().is_empty() {
                        self.chat
                            .add_message(chat::ChatMessage::assistant(text.clone()));
                    }
                    self.save_completed_turn(text);
                    self.finish_stream(false);
                    keep_rx = false;
                    break;
                }
                Ok(TaskUiEvent::Cancelled) => {
                    self.chat
                        .add_message(chat::ChatMessage::system("Request interrupted."));
                    self.finish_stream(true);
                    keep_rx = false;
                    break;
                }
                Ok(TaskUiEvent::Error(error)) => {
                    self.chat.add_message(chat::ChatMessage::assistant(format!(
                        "⚠ Task error: {}",
                        error
                    )));
                    self.finish_stream(true);
                    keep_rx = false;
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.finish_stream(true);
                    keep_rx = false;
                    break;
                }
            }
        }
        if keep_rx {
            self.task_rx = Some(rx);
        }
    }

    fn finish_stream(&mut self, had_error: bool) {
        self.llm_in_flight = false;
        self.cancel_flag = None;
        if had_error {
            self.active_user_prompt = None;
        }
        self.chat.stop_loading();
        self.chat.pending_count = self.pending_prompts.len();
        if let Some(agent) = self.agent_statuses.iter_mut().find(|a| a.0 == "Supervisor") {
            agent.1 = if had_error {
                "Idle (Error)".to_string()
            } else {
                "Idle".to_string()
            };
        }
    }

    fn set_agent_status(&mut self, agent: &str, status: &str) {
        let display_name = display_agent_name(agent);
        if let Some(existing) = self
            .agent_statuses
            .iter_mut()
            .find(|(name, _)| name.eq_ignore_ascii_case(&display_name))
        {
            existing.1 = status.to_string();
        }
    }

    fn set_active_step(&mut self, agent: &str, title: &str) {
        self.active_steps
            .insert(display_agent_name(agent), title.to_string());
    }

    fn clear_active_step(&mut self, agent: &str) {
        self.active_steps.remove(&display_agent_name(agent));
    }

    fn active_step_status(&self, agent: &str) -> Option<String> {
        self.active_steps.get(&display_agent_name(agent)).cloned()
    }

    fn visible_conversation_history(&self) -> Vec<ConversationMessage> {
        self.chat
            .messages
            .iter()
            .filter(|message| matches!(message.role.as_str(), "user" | "assistant"))
            .map(conversation_message_from_chat)
            .collect()
    }

    fn relevant_conversation_history(&self, prompt: &str) -> Vec<ConversationMessage> {
        if let (Some(manager), Some(session_id)) = (
            self.session_manager.as_ref(),
            self.current_session_id.as_ref(),
        ) {
            if let Ok(context) = manager.select_related_context(session_id, prompt) {
                return context.messages;
            }
        }
        ContextPolicy::default()
            .select(prompt, &self.visible_conversation_history())
            .messages
    }

    fn cancel_active_stream(&mut self) {
        if let Some(flag) = &self.cancel_flag {
            flag.store(true, Ordering::SeqCst);
        }
        self.task_rx = None;
        self.pending_prompts.clear();
        self.finish_stream(true);
        self.chat
            .add_message(chat::ChatMessage::system("Request interrupted."));
        if let Some(agent) = self.agent_statuses.iter_mut().find(|a| a.0 == "Supervisor") {
            agent.1 = "Idle (Cancelled)".to_string();
        }
    }

    fn handle_ask_key(&mut self, modifiers: KeyModifiers, code: KeyCode) {
        match (modifiers, code) {
            (KeyModifiers::NONE, KeyCode::Up) => {
                self.chat.ask_select_up();
            }
            (KeyModifiers::NONE, KeyCode::Down) => {
                self.chat.ask_select_down();
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                if let Some((tx, answer)) = self.chat.ask_confirm() {
                    self.chat
                        .add_message(chat::ChatMessage::system(format!("You chose: {}", answer)));
                    let _ = tx.send(answer);
                }
            }
            (KeyModifiers::NONE, KeyCode::Esc) => {
                if let Some(tx) = self.chat.ask_cancel() {
                    self.chat
                        .add_message(chat::ChatMessage::system("Ask cancelled."));
                    let _ = tx.send("(cancelled)".to_string());
                }
            }
            _ => {}
        }
    }

    fn drain_ask_requests(&mut self) {
        let Some(rx) = self.ask_rx.take() else {
            return;
        };
        match rx.try_recv() {
            Ok(request) => {
                let question = request.question.clone();
                self.chat.add_message(chat::ChatMessage::system(format!(
                    "Agent asks: {}",
                    question
                )));
                self.chat.pending_ask = Some(chat::PendingAsk {
                    question: request.question,
                    options: request.options,
                    selected: 0,
                    response_tx: request.response_tx,
                });
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {}
        }
        self.ask_rx = Some(rx);
    }

    fn handle_slash_command(&mut self, input: &str) {
        let trimmed = input.trim();
        let mut parts = trimmed.split_whitespace();
        let command = parts.next().unwrap_or("");
        match command {
            "/undo" => {
                self.pending_prompts.pop_back();
                self.chat.pending_count = self.pending_prompts.len();
                if self.chat.undo_last_turn() {
                    self.chat
                        .add_message(chat::ChatMessage::system("Undid the last turn."));
                    self.save_current_session();
                } else {
                    self.chat
                        .add_message(chat::ChatMessage::system("Nothing to undo."));
                }
            }
            "/compact" => {
                if self.chat.compact_messages(20, 8_000) {
                    self.chat
                        .add_message(chat::ChatMessage::system("Conversation compacted."));
                } else {
                    self.chat.add_message(chat::ChatMessage::system(
                        "Conversation is already compact enough.",
                    ));
                }
                self.save_current_session();
            }
            "/clear" => {
                self.chat.messages.clear();
                self.chat.thinking_log.clear();
                self.chat.latest_thinking.clear();
                self.active_steps.clear();
                self.pending_prompts.clear();
                self.active_user_prompt = None;
                self.chat.pending_count = 0;
                self.current_session_id = Some(generate_session_id());
                self.chat.add_message(chat::ChatMessage::system(
                    "Conversation cleared. New prompts will start with fresh context.",
                ));
                self.save_current_session();
            }
            "/resume" => {
                let requested = parts.next();
                match self.resume_session(requested) {
                    Ok(message) => self.chat.add_message(chat::ChatMessage::system(message)),
                    Err(error) => self.chat.add_message(chat::ChatMessage::system(format!(
                        "Resume failed: {}",
                        error
                    ))),
                }
            }
            "/help" => {
                self.chat.add_message(chat::ChatMessage::system(
                    "Commands: `/resume [id]`, `/undo`, `/compact`, `/clear`, `/help`.",
                ));
            }
            _ => {
                self.chat.add_message(chat::ChatMessage::system(
                    "Unknown command. Try `/help`, `/resume`, `/undo`, `/compact`, or `/clear`.",
                ));
            }
        }
    }

    fn resume_session(&mut self, requested_id: Option<&str>) -> zcode_core::Result<String> {
        let project_root = self.project_root.clone();
        let manager = self
            .session_manager
            .get_or_insert(SessionManager::new(project_root)?);
        let session = if let Some(id) = requested_id {
            manager.load(id)?
        } else {
            manager
                .list()?
                .into_iter()
                .next()
                .ok_or_else(|| ZcodeError::FileNotFound {
                    path: ".zcode/sessions/*.jsonl".to_string(),
                })?
        };

        let id = session.id.clone();
        let message_count = session.messages.len();
        self.current_session_id = Some(id.clone());
        self.chat
            .replace_messages(chat_messages_from_session(session));
        Ok(format!(
            "Resumed session `{}` ({} messages).",
            id, message_count
        ))
    }

    fn save_current_session(&mut self) {
        let id = self.ensure_current_session_id();
        let project_root = self.project_root.clone();
        if self.session_manager.is_none() {
            let Ok(manager) = SessionManager::new(project_root) else {
                return;
            };
            self.session_manager = Some(manager);
        }

        let Some(manager) = self.session_manager.as_ref() else {
            return;
        };
        let mut session = Session::new(id);
        for message in &self.chat.messages {
            session.push(conversation_message_from_chat(message));
        }
        let _ = manager.save(&mut session);
    }

    fn save_completed_turn(&mut self, assistant_text: String) {
        let Some(user_text) = self.active_user_prompt.take() else {
            return;
        };
        let id = self.ensure_current_session_id();
        let project_root = self.project_root.clone();
        if self.session_manager.is_none() {
            let Ok(manager) = SessionManager::new(project_root) else {
                return;
            };
            self.session_manager = Some(manager);
        }

        let Some(manager) = self.session_manager.as_ref() else {
            return;
        };
        let turn = SessionTurn {
            user: ConversationMessage::user(user_text),
            assistant: (!assistant_text.trim().is_empty())
                .then(|| ConversationMessage::assistant_text(assistant_text)),
        };
        let _ = manager.append_turn(&id, turn);
    }

    fn ensure_current_session_id(&mut self) -> String {
        if let Some(id) = &self.current_session_id {
            return id.clone();
        }
        let id = generate_session_id();
        self.current_session_id = Some(id.clone());
        if self.session_manager.is_none() {
            if let Ok(manager) = SessionManager::new(self.project_root.clone()) {
                self.session_manager = Some(manager);
            }
        }
        id
    }
}

fn generate_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("session-{}", millis)
}

fn display_agent_name(agent: &str) -> String {
    match agent {
        "supervisor" => "Supervisor".to_string(),
        "orchestrator" => "Supervisor".to_string(),
        "planner" => "Planner".to_string(),
        "coder" => "Coder".to_string(),
        "reviewer" => "Reviewer".to_string(),
        "investigator" => "Investigator".to_string(),
        "self_learning" => "Self Learning".to_string(),
        other => {
            let mut chars = other.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => other.to_string(),
            }
        }
    }
}

fn is_internal_graph_node(agent: &str) -> bool {
    matches!(agent, "supervisor" | "execute_step")
}

fn chat_messages_from_session(session: Session) -> Vec<chat::ChatMessage> {
    let mut messages = Vec::new();
    if let Some(summary) = session.summary {
        if !summary.trim().is_empty() {
            messages.push(chat::ChatMessage::system(summary));
        }
    }
    messages.extend(session.messages.into_iter().filter_map(|message| {
        let content = message.content.unwrap_or_default();
        match message.role.as_str() {
            "user" => Some(chat::ChatMessage::user(content)),
            "assistant" => Some(chat::ChatMessage::assistant(content)),
            "system" => Some(chat::ChatMessage::system(content)),
            _ => None,
        }
    }));
    messages
}

fn conversation_message_from_chat(message: &chat::ChatMessage) -> ConversationMessage {
    match message.role.as_str() {
        "user" => ConversationMessage::user(&message.content),
        "assistant" => ConversationMessage::assistant_text(&message.content),
        _ => ConversationMessage::system(&message.content),
    }
}

fn provider_message_from_history(
    message: &ConversationMessage,
) -> Option<zcode_llm_provider::Message> {
    let content = message.content.as_deref().unwrap_or_default();
    match message.role.as_str() {
        "user" => Some(zcode_llm_provider::Message::user(content)),
        "assistant" => Some(zcode_llm_provider::Message::assistant(content)),
        _ => None,
    }
}

impl Default for TuiApp {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod app_tests;
