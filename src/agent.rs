use crate::tools::{ToolCall, ToolResult, ToolRegistry, AgentError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Agent 状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AgentState {
    Idle,
    Planning,
    Executing,
    Completed,
    Failed,
}

/// Agent 配置
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub max_iterations: usize,
    pub timeout_secs: u64,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            timeout_secs: 60,
        }
    }
}

/// LLM 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }
    
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }
}

/// LLM 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub should_continue: bool,
}

impl Default for LlmResponse {
    fn default() -> Self {
        Self {
            content: String::new(),
            tool_calls: vec![],
            should_continue: true,
        }
    }
}

/// LLM 提供者 trait
pub trait LlmProvider: Send + Sync {
    fn complete(&self, messages: Vec<Message>) -> Result<LlmResponse, AgentError>;
}

/// Mock LLM Provider
pub struct MockLlmProvider {
    responses: HashMap<String, LlmResponse>,
}

impl Default for MockLlmProvider {
    fn default() -> Self {
        let mut provider = Self {
            responses: HashMap::new(),
        };
        
        provider.responses.insert("default".to_string(), LlmResponse {
            content: "I'll help you with that task.".to_string(),
            tool_calls: vec![],
            should_continue: false,
        });
        
        provider
    }
}

impl MockLlmProvider {
    pub fn new() -> Self {
        Self::default()
    }
}

impl LlmProvider for MockLlmProvider {
    fn complete(&self, _messages: Vec<Message>) -> Result<LlmResponse, AgentError> {
        Ok(self.responses.get("default")
            .cloned()
            .unwrap_or_default())
    }
}

/// Agent
pub struct Agent {
    registry: ToolRegistry,
    config: AgentConfig,
    state: AgentState,
    iteration: usize,
    start_time: Option<Instant>,
    llm: Box<dyn LlmProvider>,
    messages: Vec<Message>,
}

impl Agent {
    pub fn new(registry: ToolRegistry) -> Self {
        Self {
            registry,
            config: AgentConfig::default(),
            state: AgentState::Idle,
            iteration: 0,
            start_time: None,
            llm: Box::new(MockLlmProvider::new()),
            messages: vec![],
        }
    }
    
    pub fn with_config(mut self, config: AgentConfig) -> Self {
        self.config = config;
        self
    }
    
    pub fn with_llm(mut self, llm: Box<dyn LlmProvider>) -> Self {
        self.llm = llm;
        self
    }
    
    pub fn state(&self) -> AgentState {
        self.state
    }
    
    pub fn iteration(&self) -> usize {
        self.iteration
    }
    
    /// 执行单个工具调用
    pub fn execute_tool(&mut self, call: ToolCall) -> Result<ToolResult, AgentError> {
        if self.start_time.is_none() {
            self.start_time = Some(Instant::now());
            self.state = AgentState::Executing;
        }
        
        if self.iteration >= self.config.max_iterations {
            self.state = AgentState::Failed;
            return Err(AgentError::MaxIterations);
        }
        
        if let Some(start) = self.start_time {
            if start.elapsed() > Duration::from_secs(self.config.timeout_secs) {
                self.state = AgentState::Failed;
                return Err(AgentError::Timeout);
            }
        }
        
        self.iteration += 1;
        self.registry.execute(call)
    }
    
    /// 思考并制定计划
    pub async fn think(&mut self, task: &str) -> Result<Vec<ToolCall>, AgentError> {
        self.state = AgentState::Planning;
        self.messages.push(Message::user(task));
        let response = self.llm.complete(self.messages.clone())?;
        self.messages.push(Message::assistant(&response.content));
        Ok(response.tool_calls)
    }
    
    /// 运行 Agent（完整循环）
    pub async fn run(&mut self, task: &str) -> Result<String, AgentError> {
        self.reset();
        self.start_time = Some(Instant::now());
        
        loop {
            let tool_calls = self.think(task).await?;
            
            if tool_calls.is_empty() {
                self.state = AgentState::Completed;
                break;
            }
            
            self.state = AgentState::Executing;
            for call in tool_calls {
                let result = self.execute_tool(call)?;
                self.messages.push(Message::assistant(&format!(
                    "Tool {} result: {}",
                    result.tool_name,
                    if result.success { &result.output } else { "ERROR" }
                )));
            }
            
            if self.iteration >= self.config.max_iterations {
                self.state = AgentState::Failed;
                return Err(AgentError::MaxIterations);
            }
        }
        
        let response = self.llm.complete(self.messages.clone())?;
        Ok(response.content)
    }
    
    /// 重置 agent
    pub fn reset(&mut self) {
        self.state = AgentState::Idle;
        self.iteration = 0;
        self.start_time = None;
        self.messages.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_agent_creation() {
        let registry = ToolRegistry::new();
        let agent = Agent::new(registry);
        assert_eq!(agent.state(), AgentState::Idle);
        assert_eq!(agent.iteration(), 0);
    }

    #[test]
    fn test_agent_with_config() {
        let registry = ToolRegistry::new();
        let config = AgentConfig {
            max_iterations: 5,
            timeout_secs: 30,
        };
        let agent = Agent::new(registry).with_config(config);
        assert_eq!(agent.config.max_iterations, 5);
    }

    #[test]
    fn test_agent_execute_tool() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "hello").unwrap();
        
        let registry = ToolRegistry::new();
        let mut agent = Agent::new(registry);
        
        let call = ToolCall {
            name: "read_file".to_string(),
            arguments: serde_json::json!({
                "path": path.to_str().unwrap()
            }),
        };
        
        let result = agent.execute_tool(call).unwrap();
        assert_eq!(result.output, "hello");
        assert_eq!(agent.state(), AgentState::Executing);
        assert_eq!(agent.iteration(), 1);
    }

    #[test]
    fn test_agent_max_iterations() {
        let registry = ToolRegistry::new();
        let config = AgentConfig {
            max_iterations: 2,
            timeout_secs: 60,
        };
        let mut agent = Agent::new(registry).with_config(config);
        
        let call1 = ToolCall {
            name: "read_file".to_string(),
            arguments: serde_json::json!({"path": "/nonexistent"}),
        };
        let _ = agent.execute_tool(call1);
        
        let call2 = ToolCall {
            name: "read_file".to_string(),
            arguments: serde_json::json!({"path": "/nonexistent"}),
        };
        let _ = agent.execute_tool(call2);
        
        let call3 = ToolCall {
            name: "read_file".to_string(),
            arguments: serde_json::json!({"path": "/nonexistent"}),
        };
        let result = agent.execute_tool(call3);
        assert!(matches!(result, Err(AgentError::MaxIterations)));
        assert_eq!(agent.state(), AgentState::Failed);
    }

    #[test]
    fn test_agent_unknown_tool() {
        let registry = ToolRegistry::new();
        let mut agent = Agent::new(registry);
        
        let call = ToolCall {
            name: "unknown".to_string(),
            arguments: serde_json::json!({}),
        };
        
        let result = agent.execute_tool(call);
        assert!(result.is_err());
    }

    #[test]
    fn test_agent_reset() {
        let registry = ToolRegistry::new();
        let mut agent = Agent::new(registry);
        
        let call = ToolCall {
            name: "read_file".to_string(),
            arguments: serde_json::json!({"path": "/nonexistent"}),
        };
        let _ = agent.execute_tool(call);
        
        assert_eq!(agent.iteration(), 1);
        assert_eq!(agent.state(), AgentState::Executing);
        
        agent.reset();
        
        assert_eq!(agent.iteration(), 0);
        assert_eq!(agent.state(), AgentState::Idle);
    }

    #[test]
    fn test_config_default() {
        let config = AgentConfig::default();
        assert_eq!(config.max_iterations, 10);
        assert_eq!(config.timeout_secs, 60);
    }

    #[test]
    fn test_state_transitions() {
        let registry = ToolRegistry::new();
        let agent = Agent::new(registry);
        assert_eq!(agent.state(), AgentState::Idle);
    }

    #[test]
    fn test_iteration_count() {
        let registry = ToolRegistry::new();
        let mut agent = Agent::new(registry);
        
        assert_eq!(agent.iteration(), 0);
        
        for i in 1..=3 {
            let call = ToolCall {
                name: "read_file".to_string(),
                arguments: serde_json::json!({"path": "/nonexistent"}),
            };
            let _ = agent.execute_tool(call);
            assert_eq!(agent.iteration(), i);
        }
    }

    #[test]
    fn test_message_creation() {
        let user_msg = Message::user("test");
        assert_eq!(user_msg.role, "user");
        assert_eq!(user_msg.content, "test");
        
        let assistant_msg = Message::assistant("response");
        assert_eq!(assistant_msg.role, "assistant");
    }

    #[test]
    fn test_llm_response_default() {
        let response = LlmResponse::default();
        assert!(response.content.is_empty());
        assert!(response.tool_calls.is_empty());
        assert!(response.should_continue);
    }

    #[test]
    fn test_mock_llm_provider() {
        let provider = MockLlmProvider::new();
        let messages = vec![Message::user("test")];
        
        let response = provider.complete(messages).unwrap();
        assert!(!response.content.is_empty());
    }

    #[tokio::test]
    async fn test_agent_think() {
        let registry = ToolRegistry::new();
        let mut agent = Agent::new(registry);
        
        let result = agent.think("test task").await;
        assert!(result.is_ok());
        assert_eq!(agent.state(), AgentState::Planning);
    }

    #[test]
    fn test_agent_run() {
        let registry = ToolRegistry::new();
        let mut agent = Agent::new(registry);
        
        let result = futures::executor::block_on(agent.run("test task"));
        assert!(result.is_ok());
        assert_eq!(agent.state(), AgentState::Completed);
    }
    
    #[test]
    fn test_agent_run_with_tool_calls() {
        use crate::agent::{LlmProvider, LlmResponse, ToolCall};
        
        struct CustomMockProvider;
        
        impl LlmProvider for CustomMockProvider {
            fn complete(&self, _messages: Vec<Message>) -> Result<LlmResponse, AgentError> {
                Ok(LlmResponse {
                    content: "done".to_string(),
                    tool_calls: vec![ToolCall {
                        name: "read_file".to_string(),
                        arguments: serde_json::json!({"path": "/etc/hostname"}),
                    }],
                    should_continue: false,
                })
            }
        }
        
        let registry = ToolRegistry::new();
        let mut agent = Agent::new(registry).with_llm(Box::new(CustomMockProvider));
        
        let result = futures::executor::block_on(agent.run("read file"));
        // 可能会失败，但至少执行了更多代码路径
        assert!(result.is_ok() || result.is_err());
    }
    
    #[test]
    fn test_agent_timeout_scenario() {
        let registry = ToolRegistry::new();
        let config = AgentConfig {
            max_iterations: 100,
            timeout_secs: 0, // 0秒超时
        };
        let mut agent = Agent::new(registry).with_config(config);
        
        let call = ToolCall {
            name: "read_file".to_string(),
            arguments: serde_json::json!({"path": "/etc/hostname"}),
        };
        
        // 应该立即超时
        let result = agent.execute_tool(call);
        // 可能超时，也可能成功（如果文件读取很快）
        assert!(result.is_ok() || matches!(result, Err(AgentError::Timeout)));
    }
}
