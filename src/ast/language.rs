//! AST language registry — extensible plugin architecture
//!
//! This module provides the `LanguageProvider` trait and `LanguageRegistry` for
//! registering tree-sitter grammars at runtime. Grammars can be added in three ways:
//!
//! 1. **Code registration**: `registry.register(my_provider)`
//! 2. **Dynamic library loading**: `registry.load_dynamic("lang.so")`  
//! 3. **Directory discovery**: `registry.discover_grammars(dir)`

use crate::error::{Result, ZcodeError};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tree_sitter::Language;

// ─── LanguageProvider trait ────────────────────────────────────────────────────

/// Trait for providing a tree-sitter language grammar
pub trait LanguageProvider: Send + Sync {
    /// The canonical name of this language (e.g. "rust", "python")
    fn name(&self) -> &str;

    /// File extensions associated with this language (e.g. [".rs"])
    fn extensions(&self) -> &[&str];

    /// Return the tree-sitter Language object for this grammar
    fn language(&self) -> Language;
}

// ─── LanguageRegistry ─────────────────────────────────────────────────────────

/// Registry that maps language names and file extensions to tree-sitter grammars
pub struct LanguageRegistry {
    /// Map of language name -> provider
    by_name: HashMap<String, Arc<dyn LanguageProvider>>,
    /// Map of file extension (with dot) -> language name
    by_extension: HashMap<String, String>,
}

impl LanguageRegistry {
    /// Create an empty registry
    pub fn new() -> Self {
        Self {
            by_name: HashMap::new(),
            by_extension: HashMap::new(),
        }
    }

    /// Register a language provider
    pub fn register<P: LanguageProvider + 'static>(&mut self, provider: P) {
        let name = provider.name().to_lowercase();
        for ext in provider.extensions() {
            let ext = if ext.starts_with('.') {
                ext.to_string()
            } else {
                format!(".{}", ext)
            };
            self.by_extension.insert(ext, name.clone());
        }
        self.by_name.insert(name, Arc::new(provider));
    }

    /// Look up a language by its canonical name (case-insensitive)
    pub fn from_name(&self, name: &str) -> Option<Language> {
        self.by_name
            .get(&name.to_lowercase())
            .map(|p| p.language())
    }

    /// Look up a language by file extension (e.g. ".rs" or "rs")
    pub fn from_extension(&self, ext: &str) -> Option<Language> {
        let ext = if ext.starts_with('.') {
            ext.to_string()
        } else {
            format!(".{}", ext)
        };
        let name = self.by_extension.get(&ext)?;
        self.from_name(name)
    }

    /// Look up a language from a file path (uses the file extension)
    pub fn from_path(&self, path: &Path) -> Option<Language> {
        let ext = path.extension()?.to_str()?;
        self.from_extension(ext)
    }

    /// Get the canonical name for a file extension
    pub fn language_name_for_extension(&self, ext: &str) -> Option<&str> {
        let ext = if ext.starts_with('.') {
            ext.to_string()
        } else {
            format!(".{}", ext)
        };
        self.by_extension.get(&ext).map(|s| s.as_str())
    }

    /// List all registered language names
    pub fn registered_languages(&self) -> Vec<&str> {
        self.by_name.keys().map(|s| s.as_str()).collect()
    }

    /// Number of registered providers
    pub fn len(&self) -> usize {
        self.by_name.len()
    }

    /// Whether the registry is empty
    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }

    /// Load a dynamic library (.so / .dylib) that exports a tree-sitter language.
    ///
    /// The library must export a C function named `tree_sitter_<name>` that returns
    /// a `TSLanguage*`. A `DynamicLanguageProvider` is constructed from the library.
    ///
    /// # Safety
    /// This function loads external native code. Only load libraries from trusted sources.
    pub fn load_dynamic(&mut self, path: &Path) -> Result<()> {
        use libloading::Library;

        // Infer language name from file stem (e.g. "tree_sitter_rust.so" -> "rust")
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| ZcodeError::InvalidToolInput(
                format!("Cannot determine language name from path: {}", path.display())
            ))?;

        let lang_name = stem
            .trim_start_matches("tree_sitter_")
            .trim_start_matches("tree-sitter-")
            .to_string();

        let symbol_name = format!("tree_sitter_{}\0", lang_name);

        // SAFETY: We're loading external code — caller is responsible for trust.
        let lib = unsafe {
            Library::new(path).map_err(|e| ZcodeError::InternalError(
                format!("Failed to load dynamic library {}: {}", path.display(), e)
            ))?
        };

        let language = unsafe {
            let func: libloading::Symbol<unsafe extern "C" fn() -> *const tree_sitter::ffi::TSLanguage> =
                lib.get(symbol_name.as_bytes()).map_err(|e| ZcodeError::InternalError(
                    format!("Symbol '{}' not found in {}: {}", symbol_name.trim_end_matches('\0'), path.display(), e)
                ))?;
            Language::from_raw(func())
        };

        let provider = DynamicLanguageProvider {
            lang_name: lang_name.clone(),
            language,
            extensions: infer_extensions_for_language(&lang_name),
            _lib: Arc::new(lib),
        };

        self.register(provider);
        Ok(())
    }

    /// Scan a directory and load all `.so` / `.dylib` / `.dll` grammar files found.
    /// Returns the number of successfully loaded grammars.
    pub fn discover_grammars(&mut self, dir: &Path) -> Result<usize> {
        if !dir.exists() {
            return Ok(0);
        }

        let mut count = 0;
        let entries = std::fs::read_dir(dir)
            .map_err(|e| ZcodeError::InternalError(format!("Cannot read grammar dir: {}", e)))?;

        for entry in entries.flatten() {
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if matches!(ext, "so" | "dylib" | "dll") {
                if let Err(e) = self.load_dynamic(&path) {
                    tracing::warn!("Failed to load grammar {}: {}", path.display(), e);
                } else {
                    count += 1;
                }
            }
        }

        Ok(count)
    }
}

impl Default for LanguageRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ─── DynamicLanguageProvider ───────────────────────────────────────────────────

/// A language provider loaded from a dynamic library at runtime
struct DynamicLanguageProvider {
    lang_name: String,
    language: Language,
    extensions: Vec<String>,
    // Keep the library alive as long as this provider exists
    _lib: Arc<libloading::Library>,
}

impl LanguageProvider for DynamicLanguageProvider {
    fn name(&self) -> &str {
        &self.lang_name
    }

    fn extensions(&self) -> &[&str] {
        // We need to return &[&'static str], but our extensions are owned Strings.
        // Leak each String to get a 'static &str — this is intentional:
        // dynamic grammars live for the program lifetime once loaded.
        // This is called at most once per provider (during register()).
        let leaked: Vec<&'static str> = self.extensions
            .iter()
            .map(|s| Box::leak(s.clone().into_boxed_str()) as &str)
            .collect();
        let leaked_slice = Box::leak(leaked.into_boxed_slice());
        leaked_slice
    }

    fn language(&self) -> Language {
        self.language.clone()
    }
}

/// Heuristic: guess common file extensions for well-known language names
fn infer_extensions_for_language(name: &str) -> Vec<String> {
    match name.to_lowercase().as_str() {
        "rust" => vec![".rs".into()],
        "python" => vec![".py".into()],
        "javascript" => vec![".js".into(), ".mjs".into(), ".cjs".into()],
        "typescript" => vec![".ts".into(), ".tsx".into()],
        "go" => vec![".go".into()],
        "c" => vec![".c".into(), ".h".into()],
        "cpp" | "c++" => vec![".cpp".into(), ".cc".into(), ".cxx".into(), ".hpp".into()],
        "java" => vec![".java".into()],
        "ruby" => vec![".rb".into()],
        "bash" | "sh" => vec![".sh".into()],
        "json" => vec![".json".into()],
        "toml" => vec![".toml".into()],
        "yaml" | "yml" => vec![".yaml".into(), ".yml".into()],
        _ => vec![],
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // A minimal test provider using a mock language
    struct MockLangProvider {
        name: &'static str,
        exts: &'static [&'static str],
    }

    impl LanguageProvider for MockLangProvider {
        fn name(&self) -> &str {
            self.name
        }
        fn extensions(&self) -> &[&str] {
            self.exts
        }
        fn language(&self) -> Language {
            // We can't easily create a real Language in tests without a grammar crate.
            // We test the registry structure without actually getting a Language object.
            // Return a dummy value only in test configurations.
            unreachable!("language() should not be called in registry structural tests")
        }
    }

    #[test]
    fn test_registry_new_is_empty() {
        let reg = LanguageRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
    }

    #[test]
    fn test_registry_default_is_empty() {
        let reg = LanguageRegistry::default();
        assert!(reg.is_empty());
    }

    #[test]
    fn test_registry_register_increments_count() {
        let mut reg = LanguageRegistry::new();
        reg.register(MockLangProvider { name: "test_lang", exts: &[".tl"] });
        assert_eq!(reg.len(), 1);
        assert!(!reg.is_empty());
    }

    #[test]
    fn test_registry_from_name_registered() {
        let mut reg = LanguageRegistry::new();
        reg.register(MockLangProvider { name: "testlang", exts: &[".tl"] });
        // We can verify the provider exists by checking it is in by_name
        assert!(reg.by_name.contains_key("testlang"));
    }

    #[test]
    fn test_registry_from_name_not_found() {
        let reg = LanguageRegistry::new();
        assert!(reg.from_name("nonexistent").is_none());
    }

    #[test]
    fn test_registry_extension_mapping() {
        let mut reg = LanguageRegistry::new();
        reg.register(MockLangProvider { name: "myrust", exts: &[".rs", ".rsx"] });
        assert_eq!(reg.language_name_for_extension(".rs"), Some("myrust"));
        assert_eq!(reg.language_name_for_extension(".rsx"), Some("myrust"));
        assert_eq!(reg.language_name_for_extension("rs"), Some("myrust")); // without dot
    }

    #[test]
    fn test_registry_extension_not_found() {
        let reg = LanguageRegistry::new();
        assert!(reg.from_extension(".rs").is_none());
        assert!(reg.language_name_for_extension(".rs").is_none());
    }

    #[test]
    fn test_registry_case_insensitive_name() {
        let mut reg = LanguageRegistry::new();
        reg.register(MockLangProvider { name: "Rust", exts: &[".rs"] });
        // Stored as lowercase
        assert!(reg.by_name.contains_key("rust"));
    }

    #[test]
    fn test_registry_registered_languages() {
        let mut reg = LanguageRegistry::new();
        reg.register(MockLangProvider { name: "lang_a", exts: &[".a"] });
        reg.register(MockLangProvider { name: "lang_b", exts: &[".b"] });
        let langs = reg.registered_languages();
        assert_eq!(langs.len(), 2);
        assert!(langs.contains(&"lang_a"));
        assert!(langs.contains(&"lang_b"));
    }

    #[test]
    fn test_registry_overwrite_same_name() {
        let mut reg = LanguageRegistry::new();
        reg.register(MockLangProvider { name: "rust", exts: &[".rs"] });
        reg.register(MockLangProvider { name: "rust", exts: &[".rs"] });
        // Still one entry
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn test_registry_from_path() {
        let mut reg = LanguageRegistry::new();
        reg.register(MockLangProvider { name: "mynix", exts: &[".nix"] });
        // from_path -> None, but check extension mapping exists:
        assert_eq!(reg.language_name_for_extension(".nix"), Some("mynix"));
    }

    #[test]
    fn test_infer_extensions_known_languages() {
        assert!(infer_extensions_for_language("rust").contains(&".rs".to_string()));
        assert!(infer_extensions_for_language("python").contains(&".py".to_string()));
        assert!(infer_extensions_for_language("javascript").contains(&".js".to_string()));
    }

    #[test]
    fn test_infer_extensions_unknown_language() {
        let exts = infer_extensions_for_language("unknown_lang_xyz");
        assert!(exts.is_empty());
    }

    #[test]
    fn test_discover_grammars_empty_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut reg = LanguageRegistry::new();
        let count = reg.discover_grammars(dir.path()).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_discover_grammars_nonexistent_dir() {
        let mut reg = LanguageRegistry::new();
        let count = reg.discover_grammars(Path::new("/nonexistent/grammar/dir")).unwrap();
        assert_eq!(count, 0);
    }
}
