//! AST module for zcode
//!
//! Provides extensible AST parsing via `LanguageRegistry` + `LanguageProvider`.
//! No specific language grammars are bundled — register them at runtime.

pub mod language;
pub mod parser;
pub mod grammar;

pub use language::{LanguageProvider, LanguageRegistry};
pub use parser::{AstParser, AstTree, NodeInfo};
pub use grammar::{GrammarRegistry, GrammarLoadResult, LoadedGrammar};
