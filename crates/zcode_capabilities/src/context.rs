//! Global shared context and prompt composition for LLM providers.

use crate::skills::{Skill, SkillsLoader};

/// Shared prompt/context assembled from globally available capabilities.
#[derive(Debug, Clone, Default)]
pub struct GlobalSharedContext {
    base_prompt: String,
    context_blocks: Vec<(String, String)>,
    skills: Vec<Skill>,
}

impl GlobalSharedContext {
    pub fn new(base_prompt: impl Into<String>) -> Self {
        Self {
            base_prompt: base_prompt.into(),
            context_blocks: Vec::new(),
            skills: Vec::new(),
        }
    }

    pub fn set_base_prompt(&mut self, prompt: impl Into<String>) {
        self.base_prompt = prompt.into();
    }

    pub fn add_context_block(&mut self, title: impl Into<String>, content: impl Into<String>) {
        self.context_blocks.push((title.into(), content.into()));
    }

    pub fn set_skills(&mut self, skills: Vec<Skill>) {
        self.skills = skills;
    }

    pub fn skills(&self) -> &[Skill] {
        &self.skills
    }

    pub fn render_prompt(&self) -> String {
        let mut prompt = self.base_prompt.clone();

        for (title, content) in &self.context_blocks {
            if !content.trim().is_empty() {
                prompt.push_str(&format!("\n\n## {}\n{}", title, content));
            }
        }

        SkillsLoader::build_system_prompt(&prompt, &self.skills)
    }
}

