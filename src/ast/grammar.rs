//! Custom Tree-sitter grammar loader
//!
//! Allows loading additional grammars at runtime by specifying shared libraries.

use crate::config::GrammarConfig;
use crate::error::{Result, ZcodeError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ─── Grammar registry ──────────────────────────────────────────────────────────

/// Loaded grammar metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadedGrammar {
    pub language: String,
    pub extensions: Vec<String>,
    pub library_path: PathBuf,
    pub loaded: bool,
}

/// Registry of all known grammars (built-in + custom)
pub struct GrammarRegistry {
    /// language → grammar metadata
    grammars: HashMap<String, LoadedGrammar>,
    /// file extension → language name
    extension_map: HashMap<String, String>,
}

impl GrammarRegistry {
    /// Create a registry with built-in language support (from tree-sitter)
    pub fn new() -> Self {
        let mut registry = Self {
            grammars: HashMap::new(),
            extension_map: HashMap::new(),
        };
        registry.register_builtins();
        registry
    }

    /// Register all built-in supported languages
    fn register_builtins(&mut self) {
        let builtin = vec![
            ("rust",        vec!["rs"]),
            ("python",      vec!["py", "pyw"]),
            ("javascript",  vec!["js", "mjs", "cjs"]),
            ("typescript",  vec!["ts", "mts", "cts"]),
            ("java",        vec!["java"]),
            ("c",           vec!["c", "h"]),
            ("cpp",         vec!["cpp", "cc", "cxx", "hpp", "hxx"]),
            ("go",          vec!["go"]),
            ("ruby",        vec!["rb"]),
            ("lua",         vec!["lua"]),
            ("bash",        vec!["sh", "bash"]),
            ("toml",        vec!["toml"]),
            ("json",        vec!["json"]),
            ("yaml",        vec!["yaml", "yml"]),
            ("markdown",    vec!["md", "markdown"]),
            ("html",        vec!["html", "htm"]),
            ("css",         vec!["css"]),
        ];

        for (lang, exts) in builtin {
            let exts: Vec<String> = exts.iter().map(|s| s.to_string()).collect();
            let grammar = LoadedGrammar {
                language: lang.to_string(),
                extensions: exts.clone(),
                library_path: PathBuf::new(), // built-in has no path
                loaded: true,
            };
            for ext in &exts {
                self.extension_map.insert(ext.clone(), lang.to_string());
            }
            self.grammars.insert(lang.to_string(), grammar);
        }
    }

    /// Load custom grammars from configuration
    pub fn load_custom(&mut self, configs: &[GrammarConfig]) -> Vec<GrammarLoadResult> {
        let mut results = Vec::new();

        for config in configs {
            let lib_path = PathBuf::from(&config.library_path);
            let exists = lib_path.exists();

            let grammar = LoadedGrammar {
                language: config.language.clone(),
                extensions: config.extensions.clone(),
                library_path: lib_path.clone(),
                loaded: exists,
            };

            // Register extension mappings
            for ext in &config.extensions {
                self.extension_map.insert(ext.clone(), config.language.clone());
            }
            self.grammars.insert(config.language.clone(), grammar);

            results.push(GrammarLoadResult {
                language: config.language.clone(),
                success: exists,
                error: if exists { None } else {
                    Some(format!("Library not found: {}", lib_path.display()))
                },
            });
        }

        results
    }

    /// Resolve language name from a file path (by extension)
    pub fn language_for_file(&self, path: &Path) -> Option<&str> {
        let ext = path.extension()?.to_str()?;
        self.extension_map.get(ext).map(|s| s.as_str())
    }

    /// Resolve language name from a file extension string
    pub fn language_for_extension(&self, ext: &str) -> Option<&str> {
        self.extension_map.get(ext).map(|s| s.as_str())
    }

    /// List all registered languages
    pub fn languages(&self) -> Vec<&str> {
        self.grammars.keys().map(|s| s.as_str()).collect()
    }

    /// Check if a language is supported
    pub fn supports(&self, language: &str) -> bool {
        self.grammars.contains_key(language)
    }

    /// Get grammar metadata for a language
    pub fn get(&self, language: &str) -> Option<&LoadedGrammar> {
        self.grammars.get(language)
    }

    /// Get all loaded grammars
    pub fn all(&self) -> Vec<&LoadedGrammar> {
        self.grammars.values().collect()
    }

    /// Register a grammar from a config path (returns an error if the library doesn't exist)
    pub fn register_from_path(
        &mut self,
        language: &str,
        library_path: &Path,
        extensions: &[&str],
    ) -> Result<()> {
        if !library_path.exists() {
            return Err(ZcodeError::InternalError(
                format!("Grammar library not found: {}", library_path.display())
            ));
        }

        let exts: Vec<String> = extensions.iter().map(|s| s.to_string()).collect();
        let grammar = LoadedGrammar {
            language: language.to_string(),
            extensions: exts.clone(),
            library_path: library_path.to_path_buf(),
            loaded: true,
        };

        for ext in &exts {
            self.extension_map.insert(ext.clone(), language.to_string());
        }
        self.grammars.insert(language.to_string(), grammar);
        Ok(())
    }

    /// Summary string for a language (useful for LLM prompts)
    pub fn language_summary(&self, language: &str) -> String {
        match self.grammars.get(language) {
            Some(g) => format!(
                "{} (extensions: {})",
                g.language,
                g.extensions.join(", ")
            ),
            None => format!("{} (unsupported)", language),
        }
    }
}

impl Default for GrammarRegistry {
    fn default() -> Self { Self::new() }
}

/// Result of loading a custom grammar
#[derive(Debug, Clone)]
pub struct GrammarLoadResult {
    pub language: String,
    pub success: bool,
    pub error: Option<String>,
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GrammarConfig;
    use std::path::PathBuf;

    fn registry() -> GrammarRegistry { GrammarRegistry::new() }

    #[test]
    fn test_builtin_rust_detected() {
        let r = registry();
        assert!(r.supports("rust"));
    }

    #[test]
    fn test_builtin_python_detected() {
        assert!(registry().supports("python"));
    }

    #[test]
    fn test_builtin_javascript_detected() {
        assert!(registry().supports("javascript"));
    }

    #[test]
    fn test_builtin_typescript_detected() {
        assert!(registry().supports("typescript"));
    }

    #[test]
    fn test_builtin_lua_detected() {
        assert!(registry().supports("lua"));
    }

    #[test]
    fn test_builtin_bash_detected() {
        assert!(registry().supports("bash"));
    }

    #[test]
    fn test_language_for_rs_extension() {
        let r = registry();
        assert_eq!(r.language_for_extension("rs"), Some("rust"));
    }

    #[test]
    fn test_language_for_py_extension() {
        assert_eq!(registry().language_for_extension("py"), Some("python"));
    }

    #[test]
    fn test_language_for_ts_extension() {
        assert_eq!(registry().language_for_extension("ts"), Some("typescript"));
    }

    #[test]
    fn test_language_for_unknown_extension() {
        assert_eq!(registry().language_for_extension("xyz"), None);
    }

    #[test]
    fn test_language_for_file_path() {
        let r = registry();
        let path = PathBuf::from("src/main.rs");
        assert_eq!(r.language_for_file(&path), Some("rust"));
    }

    #[test]
    fn test_language_for_file_no_extension() {
        let r = registry();
        let path = PathBuf::from("Makefile");
        assert_eq!(r.language_for_file(&path), None);
    }

    #[test]
    fn test_languages_list_contains_builtins() {
        let r = registry();
        let langs = r.languages();
        assert!(langs.contains(&"rust"));
        assert!(langs.contains(&"python"));
        assert!(langs.contains(&"go"));
    }

    #[test]
    fn test_builtin_grammar_not_loaded_from_path() {
        let r = registry();
        let g = r.get("rust").unwrap();
        assert!(g.library_path.as_os_str().is_empty()); // built-in has no path
    }

    #[test]
    fn test_custom_grammar_nonexistent_lib_fails_gracefully() {
        let mut r = registry();
        let configs = vec![GrammarConfig {
            language: "zig".to_string(),
            library_path: "/nonexistent/zig.so".to_string(),
            extensions: vec!["zig".to_string()],
        }];
        let results = r.load_custom(&configs);
        assert_eq!(results.len(), 1);
        assert!(!results[0].success);
        assert!(results[0].error.is_some());
    }

    #[test]
    fn test_custom_grammar_registers_extension_even_if_missing() {
        // Extension should still be registered (language resolved) even if lib is missing
        let mut r = registry();
        let configs = vec![GrammarConfig {
            language: "zig".to_string(),
            library_path: "/nonexistent/zig.so".to_string(),
            extensions: vec!["zig".to_string()],
        }];
        r.load_custom(&configs);
        // Extension still mapped
        assert_eq!(r.language_for_extension("zig"), Some("zig"));
    }

    #[test]
    fn test_custom_grammar_overwrites_existing_extension() {
        let mut r = registry();
        let configs = vec![GrammarConfig {
            language: "my_js".to_string(),
            library_path: "/nonexistent/myjs.so".to_string(),
            extensions: vec!["js".to_string()], // override .js
        }];
        r.load_custom(&configs);
        assert_eq!(r.language_for_extension("js"), Some("my_js"));
    }

    #[test]
    fn test_language_summary_known() {
        let r = registry();
        let s = r.language_summary("rust");
        assert!(s.contains("rust"));
        assert!(s.contains("rs"));
    }

    #[test]
    fn test_language_summary_unknown() {
        let r = registry();
        let s = r.language_summary("unknown_lang");
        assert!(s.contains("unsupported"));
    }

    #[test]
    fn test_all_grammars_list() {
        let r = registry();
        let all = r.all();
        assert!(all.len() >= 10); // At least 10 built-ins
    }

    #[test]
    fn test_register_from_path_nonexistent() {
        let mut r = registry();
        let result = r.register_from_path("gleam", Path::new("/nonexistent.so"), &["gleam"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_register_from_path_existing() {
        use tempfile::NamedTempFile;
        let f = NamedTempFile::new().unwrap();
        let mut r = registry();
        let result = r.register_from_path("fake_lang", f.path(), &["fk"]);
        assert!(result.is_ok());
        assert!(r.supports("fake_lang"));
        assert_eq!(r.language_for_extension("fk"), Some("fake_lang"));
    }

    #[test]
    fn test_multiple_extensions_for_language() {
        let r = registry();
        // C has both .c and .h
        assert_eq!(r.language_for_extension("c"), Some("c"));
        assert_eq!(r.language_for_extension("h"), Some("c"));
    }

    #[test]
    fn test_grammar_registry_is_default() {
        let r = GrammarRegistry::default();
        assert!(r.supports("rust")); // default registers builtins
    }

    #[test]
    fn test_default_extension_html() {
        let r = registry();
        assert_eq!(r.language_for_extension("html"), Some("html"));
    }

    #[test]
    fn test_default_extension_css() {
        let r = registry();
        assert_eq!(r.language_for_extension("css"), Some("css"));
    }
}
