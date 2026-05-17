//! Conversation context selection for new task requests.
//!
//! The policy is fresh-by-default: previous turns are attached only when local
//! intent-vector retrieval finds related turns. If there are no related turns,
//! a bare continuation prompt can fall back to the most recent turn.

use crate::store::{select_related_context, Session, SessionContextConfig};
use zcode_core::agent::ConversationMessage;

pub const OPTIONAL_CONTEXT_GUARD: &str = "Previous conversation is optional background only. The current task is the only request to answer. Ignore unrelated prior results, tools, files, plans, and summaries unless the current task explicitly asks to continue or refer back to them. Do not repeat or continue prior work for a new unrelated task.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextIntent {
    FreshTask,
    FollowUp,
}

#[derive(Debug, Clone)]
pub struct SelectedContext {
    pub intent: ContextIntent,
    pub messages: Vec<ConversationMessage>,
}

#[derive(Debug, Clone)]
pub struct ContextPolicy {
    recent_user_turns: usize,
    context: SessionContextConfig,
}

impl ContextPolicy {
    pub fn fresh_by_default() -> Self {
        Self {
            recent_user_turns: 1,
            context: SessionContextConfig::default(),
        }
    }

    pub fn with_recent_user_turns(mut self, recent_user_turns: usize) -> Self {
        self.recent_user_turns = recent_user_turns.max(1);
        self
    }

    pub fn with_similarity_threshold(mut self, threshold: f32) -> Self {
        self.context.similarity_threshold = threshold;
        self
    }

    pub fn with_max_turns(mut self, max_turns: usize) -> Self {
        self.context.max_turns = max_turns.max(1);
        self
    }

    pub fn classify(&self, prompt: &str) -> ContextIntent {
        let lower = prompt.trim().to_lowercase();
        if lower.is_empty() {
            return ContextIntent::FreshTask;
        }

        if FOLLOW_UP_MARKERS
            .iter()
            .any(|marker| lower.contains(marker))
        {
            ContextIntent::FollowUp
        } else {
            ContextIntent::FreshTask
        }
    }

    pub fn select(&self, prompt: &str, history: &[ConversationMessage]) -> SelectedContext {
        let mut session = Session::new("in-memory");
        session.messages = history.to_vec();
        let context = select_related_context(&session, prompt, &self.context);
        if !context.messages.is_empty() {
            return SelectedContext {
                intent: ContextIntent::FollowUp,
                messages: context.messages,
            };
        }

        if self.classify(prompt) == ContextIntent::FollowUp {
            let messages = recent_turn_history(history, self.recent_user_turns);
            if !messages.is_empty() {
                return SelectedContext {
                    intent: ContextIntent::FollowUp,
                    messages,
                };
            }
        }

        SelectedContext {
            intent: ContextIntent::FreshTask,
            messages: Vec::new(),
        }
    }
}

impl Default for ContextPolicy {
    fn default() -> Self {
        Self::fresh_by_default()
    }
}

const FOLLOW_UP_MARKERS: &[&str] = &[
    "继续",
    "接着",
    "刚才",
    "上面",
    "前面",
    "上一",
    "上次",
    "之前",
    "上述",
    "上文",
    "前文",
    "沿用",
    "continue",
    "previous",
    "above",
    "earlier",
    "last answer",
    "last response",
    "last message",
    "last turn",
    "follow up",
    "follow-up",
    "as before",
    "same as before",
];

fn recent_turn_history(
    history: &[ConversationMessage],
    recent_user_turns: usize,
) -> Vec<ConversationMessage> {
    let mut seen_users = 0usize;
    let mut start = None;

    for (index, message) in history.iter().enumerate().rev() {
        if message.role == "user" {
            seen_users += 1;
            start = Some(index);
            if seen_users >= recent_user_turns {
                break;
            }
        }
    }

    let Some(start) = start else {
        return Vec::new();
    };

    history[start..]
        .iter()
        .filter(|message| matches!(message.role.as_str(), "user" | "assistant"))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_task_omits_previous_history() {
        let history = vec![
            ConversationMessage::user("介绍 zcode 工程"),
            ConversationMessage::assistant_text("工程介绍"),
        ];

        let selected = ContextPolicy::default().select("午餐推荐", &history);

        assert_eq!(selected.intent, ContextIntent::FreshTask);
        assert!(selected.messages.is_empty());
    }

    #[test]
    fn continuation_words_do_not_force_history_when_new_topic_is_clear() {
        let history = vec![
            ConversationMessage::user("介绍 zcode 工程"),
            ConversationMessage::assistant_text("工程介绍"),
        ];

        let selected = ContextPolicy::default().select("再问一下午餐推荐", &history);

        assert_eq!(selected.intent, ContextIntent::FreshTask);
        assert!(selected.messages.is_empty());
    }

    #[test]
    fn follow_up_uses_recent_turn_only() {
        let history = vec![
            ConversationMessage::user("old question"),
            ConversationMessage::assistant_text("old answer"),
            ConversationMessage::user("介绍 zcode 工程结构"),
            ConversationMessage::assistant_text("zcode 是 Rust workspace"),
        ];

        let selected = ContextPolicy::default().select("继续解释 zcode 工程", &history);

        assert_eq!(selected.intent, ContextIntent::FollowUp);
        assert_eq!(selected.messages.len(), 2);
        assert_eq!(
            selected.messages[0].content.as_deref(),
            Some("介绍 zcode 工程结构")
        );
        assert_eq!(
            selected.messages[1].content.as_deref(),
            Some("zcode 是 Rust workspace")
        );
    }

    #[test]
    fn explicit_continuation_can_use_recent_turn() {
        let history = vec![
            ConversationMessage::user("list files"),
            ConversationMessage::assistant_text("Cargo.toml"),
        ];

        let selected = ContextPolicy::default().select("继续", &history);

        assert_eq!(selected.intent, ContextIntent::FollowUp);
        assert_eq!(selected.messages.len(), 2);
        assert_eq!(selected.messages[0].content.as_deref(), Some("list files"));
    }
}
