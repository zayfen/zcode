use std::sync::Arc;

use zcode_core::{LlmConfig, ProjectConfig, Result, Settings, ZcodeError};
#[cfg(test)]
use zcode_llm_provider::MockLlmProvider;
#[cfg(not(test))]
use zcode_llm_provider::RigProvider;
use zcode_llm_provider::{LlmProvider, Message};
use zcode_orchestration::{
    AgentModelLabels, AgentRuntime, GraphEvent, LlmResponse as AgentLlmResponse, TaskAgentRuntimes,
};

#[derive(Clone)]
pub(crate) struct AgentLlm {
    pub(crate) provider: Arc<dyn LlmProvider>,
    pub(crate) config: LlmConfig,
    explicit_model: bool,
}

#[derive(Clone)]
pub(crate) struct AgentProviders {
    pub(crate) fast: AgentLlm,
    pub(crate) supervisor: AgentLlm,
    pub(crate) planner: AgentLlm,
    pub(crate) coder: AgentLlm,
    pub(crate) reviewer: AgentLlm,
    pub(crate) investigator: AgentLlm,
    pub(crate) docs: AgentLlm,
}

impl AgentProviders {
    pub(crate) fn task_models(&self) -> AgentModelLabels {
        AgentModelLabels {
            supervisor: self.supervisor.config.model.clone(),
            investigator: self.investigator.config.model.clone(),
            planner: self.planner.config.model.clone(),
            coder: self.coder.config.model.clone(),
            reviewer: self.reviewer.config.model.clone(),
            fast: self.fast.config.model.clone(),
        }
    }

    pub(crate) fn task_runtimes(
        &self,
        event_sink: Option<Arc<dyn Fn(GraphEvent) + Send + Sync>>,
    ) -> TaskAgentRuntimes {
        let attach_sink = |runtime: AgentRuntime| {
            if let Some(event_sink) = &event_sink {
                runtime.with_event_sink(Arc::clone(event_sink))
            } else {
                runtime
            }
        };

        TaskAgentRuntimes {
            supervisor: attach_sink(AgentRuntime::new(
                Arc::clone(&self.supervisor.provider),
                self.supervisor.config.model.clone(),
                self.supervisor.explicit_model,
            )),
            investigator: attach_sink(AgentRuntime::new(
                Arc::clone(&self.investigator.provider),
                self.investigator.config.model.clone(),
                self.investigator.explicit_model,
            )),
            planner: attach_sink(AgentRuntime::new(
                Arc::clone(&self.planner.provider),
                self.planner.config.model.clone(),
                self.planner.explicit_model,
            )),
            coder: attach_sink(AgentRuntime::new(
                Arc::clone(&self.coder.provider),
                self.coder.config.model.clone(),
                self.coder.explicit_model,
            )),
            reviewer: attach_sink(AgentRuntime::new(
                Arc::clone(&self.reviewer.provider),
                self.reviewer.config.model.clone(),
                self.reviewer.explicit_model,
            )),
            fast: AgentRuntime::new(
                Arc::clone(&self.fast.provider),
                self.fast.config.model.clone(),
                self.fast.explicit_model,
            ),
        }
    }

    pub(crate) fn reviewer_runtime(
        &self,
        event_sink: Option<Arc<dyn Fn(GraphEvent) + Send + Sync>>,
    ) -> AgentRuntime {
        let runtime = AgentRuntime::new(
            Arc::clone(&self.reviewer.provider),
            self.reviewer.config.model.clone(),
            self.reviewer.explicit_model,
        );
        if let Some(event_sink) = event_sink {
            runtime.with_event_sink(event_sink)
        } else {
            runtime
        }
    }
}

pub(crate) fn build_agent_providers(
    settings: &Settings,
    project_config: &ProjectConfig,
) -> Result<AgentProviders> {
    let mut default_config = llm_config_from_settings(settings);
    apply_project_llm_overrides(&mut default_config, project_config);
    let fast_config = fast_llm_config(&default_config);

    let fast = AgentLlm {
        provider: make_llm_provider(&fast_config),
        config: fast_config,
        explicit_model: false,
    };

    let make_agent = |agent: &str| -> Result<AgentLlm> {
        let (config, explicit_model) =
            resolve_agent_config(agent, &default_config, project_config)?;
        Ok(AgentLlm {
            provider: make_llm_provider(&config),
            config,
            explicit_model,
        })
    };

    Ok(AgentProviders {
        fast,
        supervisor: make_agent("supervisor")?,
        planner: make_agent("planner")?,
        coder: make_agent("coder")?,
        reviewer: make_agent("reviewer")?,
        investigator: make_agent("investigator")?,
        docs: make_agent("docs")?,
    })
}

pub(crate) async fn feed_call_llm(
    provider: Arc<dyn LlmProvider>,
    msgs: Vec<serde_json::Value>,
    tools: Vec<serde_json::Value>,
) -> Result<AgentLlmResponse> {
    let llm_messages: Vec<Message> = msgs
        .iter()
        .filter_map(|value| provider_message_from_value(value))
        .collect();

    match provider.chat(&llm_messages, &tools) {
        Ok(resp) => AgentLlmResponse::from_openai_response(&resp.raw_response)
            .or_else(|_| Ok(AgentLlmResponse::Text(resp.content))),
        Err(error) => Err(error),
    }
}

fn llm_config_from_settings(settings: &Settings) -> LlmConfig {
    LlmConfig {
        provider: settings.llm.provider.clone(),
        model: settings.llm.model.clone(),
        fast_model: settings.llm.fast_model.clone(),
        base_url: settings.llm.base_url.clone(),
        api_key: settings.llm.api_key.clone(),
        api_key_env: settings.llm.api_key_env.clone(),
        temperature: settings.llm.temperature,
        max_tokens: settings.llm.max_tokens,
    }
}

fn fast_llm_config(config: &LlmConfig) -> LlmConfig {
    let mut fast = config.clone();
    fast.model = config
        .fast_model
        .clone()
        .unwrap_or_else(|| config.model.clone());
    fast
}

fn apply_project_llm_overrides(config: &mut LlmConfig, project_config: &ProjectConfig) {
    if let Some(llm) = &project_config.llm {
        if let Some(provider) = &llm.provider {
            config.provider = provider.clone();
        }
        if let Some(model) = &llm.model {
            config.model = model.clone();
        }
        if let Some(fast_model) = &llm.fast_model {
            config.fast_model = Some(fast_model.clone());
        }
        if let Some(base_url) = &llm.base_url {
            config.base_url = Some(base_url.clone());
        }
        if let Some(api_key) = &llm.api_key {
            config.api_key = Some(api_key.clone());
        }
        if let Some(api_key_env) = &llm.api_key_env {
            config.api_key_env = Some(api_key_env.clone());
        }
        if let Some(temperature) = llm.temperature {
            config.temperature = temperature;
        }
        if let Some(max_tokens) = llm.max_tokens {
            config.max_tokens = max_tokens;
        }
    }
}

pub(crate) fn parse_provider_model(value: &str) -> Result<(&str, &str)> {
    let (provider, model) = value.split_once('/').ok_or_else(|| {
        ZcodeError::ConfigError(format!(
            "agent model `{}` must use `provider/model` format",
            value
        ))
    })?;
    if provider.trim().is_empty() || model.trim().is_empty() {
        return Err(ZcodeError::ConfigError(format!(
            "agent model `{}` must include both provider and model",
            value
        )));
    }
    Ok((provider.trim(), model.trim()))
}

pub(crate) fn resolve_agent_config(
    agent: &str,
    default_config: &LlmConfig,
    project_config: &ProjectConfig,
) -> Result<(LlmConfig, bool)> {
    let Some(value) = project_config.agent_models.get(agent) else {
        return Ok((default_config.clone(), false));
    };
    let (provider_id, model) = parse_provider_model(value)?;
    let provider = project_config
        .llm
        .as_ref()
        .and_then(|llm| llm.providers.get(provider_id))
        .ok_or_else(|| {
            ZcodeError::ConfigError(format!(
                "agent model `{}` references unknown provider `{}`",
                value, provider_id
            ))
        })?;

    let mut config = default_config.clone();
    config.provider = provider_id.to_string();
    config.model = model.to_string();
    config.fast_model = None;
    config.base_url = provider.base_url.clone();
    config.api_key = provider.api_key.clone();
    config.api_key_env = provider.api_key_env.clone();
    if let Some(temperature) = provider.temperature {
        config.temperature = temperature;
    }
    if let Some(max_tokens) = provider.max_tokens {
        config.max_tokens = max_tokens;
    }

    Ok((config, true))
}

#[cfg(not(test))]
fn make_llm_provider(config: &LlmConfig) -> Arc<dyn LlmProvider> {
    Arc::new(RigProvider::new(config.clone()))
}

#[cfg(test)]
fn make_llm_provider(_config: &LlmConfig) -> Arc<dyn LlmProvider> {
    Arc::new(MockLlmProvider::new("PASS"))
}

fn provider_message_from_value(value: &serde_json::Value) -> Option<Message> {
    let role = value.get("role")?.as_str()?;
    let content = value
        .get("content")
        .and_then(|content| content.as_str())
        .unwrap_or("")
        .to_string();

    match role {
        "system" => Some(Message::system(content)),
        "assistant" => Some(assistant_message_from_value(value, content)),
        "tool" => Some(Message::tool_result(
            value
                .get("tool_call_id")
                .and_then(|id| id.as_str())
                .unwrap_or("")
                .to_string(),
            value
                .get("name")
                .and_then(|name| name.as_str())
                .unwrap_or("")
                .to_string(),
            content,
        )),
        _ => Some(Message::user(content)),
    }
}

fn assistant_message_from_value(value: &serde_json::Value, content: String) -> Message {
    let Some(tool_calls) = value.get("tool_calls").and_then(|calls| calls.as_array()) else {
        return Message::assistant(content);
    };
    if tool_calls.is_empty() {
        return Message::assistant(content);
    }

    let reasoning_content = value
        .get("reasoning_content")
        .and_then(|content| content.as_str())
        .map(str::to_string);
    Message::assistant_with_tool_calls_and_reasoning(content, tool_calls.clone(), reasoning_content)
}
