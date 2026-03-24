//! Integration tests: GrammarRegistry + custom grammar loading

use std::io::Write;
use tempfile::NamedTempFile;
use zcode::ast::GrammarRegistry;
use zcode::config::GrammarConfig;
use std::path::{Path, PathBuf};

// ─── Built-in grammar tests ───────────────────────────────────────────────────

#[test]
fn test_registry_all_builtin_languages_present() {
    let r = GrammarRegistry::new();
    let expected = [
        "rust", "python", "javascript", "typescript",
        "java", "c", "cpp", "go", "ruby", "lua",
        "bash", "toml", "json", "yaml", "markdown", "html", "css",
    ];
    for lang in &expected {
        assert!(r.supports(lang), "Missing builtin language: {}", lang);
    }
}

#[test]
fn test_registry_extension_to_language_complete() {
    let r = GrammarRegistry::new();
    let pairs = [
        ("rs", "rust"), ("py", "python"), ("js", "javascript"),
        ("ts", "typescript"), ("go", "go"), ("java", "java"),
        ("c", "c"), ("h", "c"), ("cpp", "cpp"), ("lua", "lua"),
        ("sh", "bash"), ("toml", "toml"), ("json", "json"),
        ("yaml", "yaml"), ("yml", "yaml"), ("md", "markdown"),
        ("html", "html"), ("css", "css"), ("rb", "ruby"),
    ];
    for (ext, expected_lang) in &pairs {
        assert_eq!(
            r.language_for_extension(ext),
            Some(*expected_lang),
            "Extension .{} should map to {}", ext, expected_lang
        );
    }
}

#[test]
fn test_registry_file_path_resolution() {
    let r = GrammarRegistry::new();
    let cases = [
        ("src/main.rs", "rust"),
        ("app/views.py", "python"),
        ("index.ts", "typescript"),
        ("main.go", "go"),
        ("README.md", "markdown"),
        ("config.toml", "toml"),
        ("data.json", "json"),
        ("deploy.sh", "bash"),
    ];
    for (path_str, expected_lang) in &cases {
        let path = PathBuf::from(path_str);
        assert_eq!(
            r.language_for_file(&path),
            Some(*expected_lang),
            "Path '{}' should resolve to '{}'", path_str, expected_lang
        );
    }
}

#[test]
fn test_registry_unsupported_extension() {
    let r = GrammarRegistry::new();
    assert_eq!(r.language_for_extension("zig"), None);
    assert_eq!(r.language_for_extension("gleam"), None);
    assert_eq!(r.language_for_extension("elm"), None);
}

#[test]
fn test_registry_language_summary_format() {
    let r = GrammarRegistry::new();
    let summary = r.language_summary("rust");
    assert!(summary.contains("rust"), "Summary should contain language name");
    assert!(summary.contains("rs"), "Summary should contain file extension");
}

// ─── Custom grammar registration ──────────────────────────────────────────────

#[test]
fn test_custom_grammar_via_temp_file() {
    // Create a temporary "library" file to simulate a grammar .so
    let lib_file = NamedTempFile::new().unwrap();

    let mut r = GrammarRegistry::new();
    let result = r.register_from_path(
        "gleam",
        lib_file.path(),
        &["gleam"],
    );
    assert!(result.is_ok());
    assert!(r.supports("gleam"));
    assert_eq!(r.language_for_extension("gleam"), Some("gleam"));
}

#[test]
fn test_custom_grammar_overwrites_builtin_extension() {
    let lib_file = NamedTempFile::new().unwrap();
    let mut r = GrammarRegistry::new();

    // Override .py to a custom parser
    r.register_from_path("custom_python", lib_file.path(), &["py"]).unwrap();
    assert_eq!(r.language_for_extension("py"), Some("custom_python"));
}

#[test]
fn test_load_custom_configs_batch() {
    let lib1 = NamedTempFile::new().unwrap();
    let lib2 = NamedTempFile::new().unwrap();

    let configs = vec![
        GrammarConfig {
            language: "zig".to_string(),
            library_path: lib1.path().to_str().unwrap().to_string(),
            extensions: vec!["zig".to_string()],
        },
        GrammarConfig {
            language: "gleam".to_string(),
            library_path: lib2.path().to_str().unwrap().to_string(),
            extensions: vec!["gleam".to_string()],
        },
    ];

    let mut r = GrammarRegistry::new();
    let results = r.load_custom(&configs);

    assert_eq!(results.len(), 2);
    assert!(results[0].success, "zig should load successfully");
    assert!(results[1].success, "gleam should load successfully");
    assert!(r.supports("zig"));
    assert!(r.supports("gleam"));
}

#[test]
fn test_load_custom_config_missing_file() {
    let configs = vec![GrammarConfig {
        language: "phantom".to_string(),
        library_path: "/nonexistent/grammar.so".to_string(),
        extensions: vec!["ph".to_string()],
    }];

    let mut r = GrammarRegistry::new();
    let results = r.load_custom(&configs);
    assert_eq!(results.len(), 1);
    assert!(!results[0].success);
    assert!(results[0].error.is_some());
    // Extension still registered even if library missing
    assert_eq!(r.language_for_extension("ph"), Some("phantom"));
}

// ─── Grammar-aware context building ──────────────────────────────────────────

#[test]
fn test_grammar_registry_with_workspace_files() {
    let r = GrammarRegistry::new();
    let zcode_files = [
        "src/main.rs",
        "src/lib.rs",
        "Cargo.toml",
        "README.md",
        "scripts/build.sh",
    ];

    for file_path in &zcode_files {
        let path = PathBuf::from(file_path);
        let lang = r.language_for_file(&path);
        assert!(lang.is_some(), "Should detect language for: {}", file_path);
    }
}

#[test]
fn test_grammar_registry_filter_files_by_language() {
    let r = GrammarRegistry::new();
    let files = vec![
        PathBuf::from("main.rs"),
        PathBuf::from("utils.py"),
        PathBuf::from("app.js"),
        PathBuf::from("config.toml"),
        PathBuf::from("build.sh"),
    ];

    // Filter only Rust files
    let rust_files: Vec<_> = files.iter()
        .filter(|p| r.language_for_file(p) == Some("rust"))
        .collect();
    assert_eq!(rust_files.len(), 1);
    assert_eq!(rust_files[0], &PathBuf::from("main.rs"));

    // Filter non-script files
    let non_script: Vec<_> = files.iter()
        .filter(|p| r.language_for_file(p) != Some("bash"))
        .collect();
    assert_eq!(non_script.len(), 4);
}

#[test]
fn test_all_grammars_have_extensions() {
    let r = GrammarRegistry::new();
    for grammar in r.all() {
        assert!(
            !grammar.extensions.is_empty(),
            "Grammar '{}' should have at least one extension",
            grammar.language
        );
    }
}

#[test]
fn test_grammar_registry_total_extension_count() {
    let r = GrammarRegistry::new();
    let total_extensions: usize = r.all().iter().map(|g| g.extensions.len()).sum();
    // 17 languages, many have 2+ extensions (c/h, cpp/cc/cxx/hpp/hxx, etc.)
    assert!(total_extensions >= 20, "Should have at least 20 file extension mappings");
}
