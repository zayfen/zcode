//! Terminal UI layer.

pub mod tui;

pub use tui::{init_terminal, restore_terminal, ChatInterface, TuiApp, TuiBackend, TuiTerminal};

