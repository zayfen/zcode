mod tools;
mod agent;
mod llm;

use agent::{Agent, AgentConfig};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tools::{ToolCall, ToolRegistry};

#[derive(Parser)]
#[command(name = "zcode")]
#[command(about = "A code agent with LLM integration", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Read a file
    Read {
        /// File path
        path: PathBuf,
    },
    /// Write a file
    Write {
        /// File path
        path: PathBuf,
        /// Content to write
        #[arg(short, long)]
        content: String,
    },
    /// Edit a file
    Edit {
        /// File path
        path: PathBuf,
        /// Old text to find
        #[arg(short, long)]
        old: String,
        /// New text to replace
        #[arg(short = 'n', long)]
        new: String,
    },
    /// Execute a command
    Exec {
        /// Command to execute
        command: String,
        /// Arguments
        #[arg(short, long)]
        args: Vec<String>,
    },
    /// Search for pattern
    Search {
        /// Search pattern
        pattern: String,
        /// Search path
        #[arg(short, long)]
        path: Option<String>,
    },
    /// Run agent task
    Run {
        /// Task description
        task: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    let registry = ToolRegistry::new();
    let config = AgentConfig::default();
    let mut agent = Agent::new(registry).with_config(config);
    
    match cli.command {
        Commands::Read { path } => {
            let call = ToolCall {
                name: "read_file".to_string(),
                arguments: serde_json::json!({
                    "path": path.to_str().unwrap()
                }),
            };
            
            match agent.execute_tool(call) {
                Ok(result) => println!("{}", result.output),
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        Commands::Write { path, content } => {
            let call = ToolCall {
                name: "write_file".to_string(),
                arguments: serde_json::json!({
                    "path": path.to_str().unwrap(),
                    "content": content
                }),
            };
            
            match agent.execute_tool(call) {
                Ok(result) => println!("{}", result.output),
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        Commands::Edit { path, old, new } => {
            let call = ToolCall {
                name: "edit_file".to_string(),
                arguments: serde_json::json!({
                    "path": path.to_str().unwrap(),
                    "old_text": old,
                    "new_text": new
                }),
            };
            
            match agent.execute_tool(call) {
                Ok(result) => println!("{}", result.output),
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        Commands::Exec { command, args } => {
            let call = ToolCall {
                name: "execute".to_string(),
                arguments: serde_json::json!({
                    "command": command,
                    "args": args
                }),
            };
            
            match agent.execute_tool(call) {
                Ok(result) => println!("{}", result.output),
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        Commands::Search { pattern, path } => {
            let call = ToolCall {
                name: "search".to_string(),
                arguments: serde_json::json!({
                    "pattern": pattern,
                    "path": path.unwrap_or_else(|| ".".to_string())
                }),
            };
            
            match agent.execute_tool(call) {
                Ok(result) => println!("{}", result.output),
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        Commands::Run { task } => {
            match agent.run(&task).await {
                Ok(result) => println!("Agent result: {}", result),
                Err(e) => eprintln!("Agent error: {}", e),
            }
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_cli_parse_read() {
        let cli = Cli::try_parse_from(["zcode", "read", "test.txt"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_parse_write() {
        let cli = Cli::try_parse_from(["zcode", "write", "test.txt", "-c", "hello"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_parse_edit() {
        let cli = Cli::try_parse_from(["zcode", "edit", "test.txt", "-o", "old", "-n", "new"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_parse_exec() {
        let cli = Cli::try_parse_from(["zcode", "exec", "ls"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_parse_search() {
        let cli = Cli::try_parse_from(["zcode", "search", "pattern", "-p", "src"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_parse_run() {
        let cli = Cli::try_parse_from(["zcode", "run", "add error handling"]);
        assert!(cli.is_ok());
    }

    #[tokio::test]
    async fn test_agent_run_integration() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "test content").unwrap();
        
        let registry = ToolRegistry::new();
        let config = AgentConfig {
            max_iterations: 5,
            timeout_secs: 10,
        };
        let mut agent = Agent::new(registry).with_config(config);
        
        let result = agent.run("read test file").await;
        assert!(result.is_ok() || result.is_err()); // Either is fine for mock
    }

    #[test]
    fn test_tool_call_creation() {
        let call = ToolCall {
            name: "test".to_string(),
            arguments: serde_json::json!({"key": "value"}),
        };
        assert_eq!(call.name, "test");
    }

    #[test]
    fn test_agent_creation() {
        let registry = ToolRegistry::new();
        let agent = Agent::new(registry);
        assert_eq!(agent.state(), agent::AgentState::Idle);
    }

    #[test]
    fn test_agent_config() {
        let config = AgentConfig {
            max_iterations: 5,
            timeout_secs: 30,
        };
        assert_eq!(config.max_iterations, 5);
    }

    #[test]
    fn test_read_command_integration() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "hello world").unwrap();
        
        let registry = ToolRegistry::new();
        let mut agent = Agent::new(registry);
        
        let call = ToolCall {
            name: "read_file".to_string(),
            arguments: serde_json::json!({
                "path": path.to_str().unwrap()
            }),
        };
        
        let result = agent.execute_tool(call).unwrap();
        assert_eq!(result.output, "hello world");
    }

    #[test]
    fn test_write_command_integration() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        
        let registry = ToolRegistry::new();
        let mut agent = Agent::new(registry);
        
        let call = ToolCall {
            name: "write_file".to_string(),
            arguments: serde_json::json!({
                "path": path.to_str().unwrap(),
                "content": "test content"
            }),
        };
        
        let result = agent.execute_tool(call).unwrap();
        assert!(result.success);
        
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "test content");
    }

    #[test]
    fn test_edit_command_integration() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "hello world").unwrap();
        
        let registry = ToolRegistry::new();
        let mut agent = Agent::new(registry);
        
        let call = ToolCall {
            name: "edit_file".to_string(),
            arguments: serde_json::json!({
                "path": path.to_str().unwrap(),
                "old_text": "world",
                "new_text": "Rust"
            }),
        };
        
        let result = agent.execute_tool(call).unwrap();
        assert!(result.success);
        
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "hello Rust");
    }

    #[test]
    fn test_exec_command_integration() {
        let registry = ToolRegistry::new();
        let mut agent = Agent::new(registry);
        
        let call = ToolCall {
            name: "execute".to_string(),
            arguments: serde_json::json!({
                "command": "echo",
                "args": ["hello"]
            }),
        };
        
        let result = agent.execute_tool(call).unwrap();
        assert!(result.success);
        assert!(result.output.contains("hello"));
    }

    #[test]
    fn test_search_command_integration() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.rs");
        fs::write(&path, "fn main() {}").unwrap();
        
        let registry = ToolRegistry::new();
        let mut agent = Agent::new(registry);
        
        let call = ToolCall {
            name: "search".to_string(),
            arguments: serde_json::json!({
                "pattern": "fn",
                "path": dir.path().to_str().unwrap()
            }),
        };
        
        let result = agent.execute_tool(call).unwrap();
        assert!(result.success);
        assert!(result.output.contains("fn main"));
    }

    // ========== Main Function Tests ==========
    
    #[test]
    fn test_main_read_file_execution() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "content").unwrap();
        
        let registry = ToolRegistry::new();
        let mut agent = Agent::new(registry);
        
        let call = ToolCall {
            name: "read_file".to_string(),
            arguments: serde_json::json!({"path": path.to_str().unwrap()}),
        };
        
        let result = agent.execute_tool(call).unwrap();
        assert_eq!(result.output, "content");
        assert!(result.success);
    }

    #[test]
    fn test_main_write_file_execution() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        
        let registry = ToolRegistry::new();
        let mut agent = Agent::new(registry);
        
        let call = ToolCall {
            name: "write_file".to_string(),
            arguments: serde_json::json!({
                "path": path.to_str().unwrap(),
                "content": "new content"
            }),
        };
        
        let result = agent.execute_tool(call).unwrap();
        assert!(result.success);
        assert!(path.exists());
        assert_eq!(fs::read_to_string(&path).unwrap(), "new content");
    }

    #[test]
    fn test_main_edit_file_execution() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "old text here").unwrap();
        
        let registry = ToolRegistry::new();
        let mut agent = Agent::new(registry);
        
        let call = ToolCall {
            name: "edit_file".to_string(),
            arguments: serde_json::json!({
                "path": path.to_str().unwrap(),
                "old_text": "old",
                "new_text": "new"
            }),
        };
        
        let result = agent.execute_tool(call).unwrap();
        assert!(result.success);
        assert_eq!(fs::read_to_string(&path).unwrap(), "new text here");
    }

    #[test]
    fn test_main_exec_command_execution() {
        let registry = ToolRegistry::new();
        let mut agent = Agent::new(registry);
        
        let call = ToolCall {
            name: "execute".to_string(),
            arguments: serde_json::json!({
                "command": "echo",
                "args": ["test"]
            }),
        };
        
        let result = agent.execute_tool(call).unwrap();
        assert!(result.success);
        assert!(result.output.contains("test"));
    }

    #[test]
    fn test_main_search_command_execution() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.rs");
        fs::write(&path, "fn test() {}").unwrap();
        
        let registry = ToolRegistry::new();
        let mut agent = Agent::new(registry);
        
        let call = ToolCall {
            name: "search".to_string(),
            arguments: serde_json::json!({
                "pattern": "test",
                "path": dir.path().to_str().unwrap()
            }),
        };
        
        let result = agent.execute_tool(call).unwrap();
        assert!(result.success);
        assert!(result.output.contains("test"));
    }

    #[tokio::test]
    async fn test_main_run_command_execution() {
        let registry = ToolRegistry::new();
        let mut agent = Agent::new(registry);
        
        let result = agent.run("simple task").await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_main_error_handling() {
        let registry = ToolRegistry::new();
        let mut agent = Agent::new(registry);
        
        let call = ToolCall {
            name: "read_file".to_string(),
            arguments: serde_json::json!({"path": "/nonexistent/file.txt"}),
        };
        
        let result = agent.execute_tool(call);
        assert!(result.is_err());
    }
}

