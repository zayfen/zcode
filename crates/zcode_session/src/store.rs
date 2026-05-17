//! Session message storage, history loading, deletion, and compression.
//!
//! Each session is stored as one JSONL file under `.zcode/sessions/`. A line is
//! either metadata or one conversation message. This keeps appends cheap while
//! preserving a single durable file per interactive session.

use crate::intent::{
    intent_text_from_parts, HashedIntentVectorizer, IntentDocument, IntentVector, IntentVectorIndex,
};
use crate::lance_index::{LanceIntentDocument, LanceIntentIndex};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use zcode_core::agent::ConversationMessage;
use zcode_core::{Result, ZcodeError};

const SESSION_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub summary: Option<String>,
    pub messages: Vec<ConversationMessage>,
}

impl Session {
    pub fn new(id: impl Into<String>) -> Self {
        let now = now_secs();
        Self {
            id: id.into(),
            created_at: now,
            updated_at: now,
            summary: None,
            messages: Vec::new(),
        }
    }

    pub fn push(&mut self, message: ConversationMessage) {
        self.messages.push(message);
        self.updated_at = now_secs();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTurn {
    pub user: ConversationMessage,
    pub assistant: Option<ConversationMessage>,
}

#[derive(Debug, Clone)]
pub struct SessionContextConfig {
    pub similarity_threshold: f32,
    pub max_turns: usize,
}

impl Default for SessionContextConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.18,
            max_turns: 3,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SessionContext {
    pub messages: Vec<ConversationMessage>,
    pub matched_turns: Vec<MatchedSessionTurn>,
}

#[derive(Debug, Clone)]
pub struct MatchedSessionTurn {
    pub turn_id: u64,
    pub score: f32,
}

#[derive(Debug, Clone)]
pub struct CompressionConfig {
    pub retain_recent: usize,
    pub summary_max_chars: usize,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            retain_recent: 20,
            summary_max_chars: 8_000,
        }
    }
}

pub struct SessionManager {
    sessions_dir: PathBuf,
    index_dir: PathBuf,
    compression: CompressionConfig,
    context: SessionContextConfig,
}

impl SessionManager {
    pub fn new(project_root: impl AsRef<Path>) -> Result<Self> {
        let zcode_dir = project_root.as_ref().join(".zcode");
        let sessions_dir = zcode_dir.join("sessions");
        let index_dir = zcode_dir.join("session-index");
        std::fs::create_dir_all(&sessions_dir)?;
        std::fs::create_dir_all(&index_dir)?;
        Ok(Self {
            sessions_dir,
            index_dir,
            compression: CompressionConfig::default(),
            context: SessionContextConfig::default(),
        })
    }

    pub fn with_compression(mut self, compression: CompressionConfig) -> Self {
        self.compression = compression;
        self
    }

    pub fn with_context_config(mut self, context: SessionContextConfig) -> Self {
        self.context = context;
        self
    }

    pub fn create(&self, id: impl Into<String>) -> Session {
        Session::new(id)
    }

    pub fn save(&self, session: &mut Session) -> Result<()> {
        session.updated_at = now_secs();
        let path = self.session_path(&session.id);
        let mut lines = Vec::new();
        lines.push(serde_json::to_string(&SessionRecord::Meta(SessionMeta {
            schema_version: SESSION_SCHEMA_VERSION,
            id: session.id.clone(),
            created_at: session.created_at,
            updated_at: session.updated_at,
            summary: session.summary.clone(),
        }))?);

        let mut turn_id = 0u64;
        for message in &session.messages {
            let record = match message.role.as_str() {
                "user" => {
                    turn_id += 1;
                    SessionRecord::Message(SessionMessageRecord {
                        turn_id: Some(turn_id),
                        message: message.clone(),
                        intent_vector: Some(intent_vector_for_messages([message])),
                    })
                }
                "assistant" => SessionRecord::Message(SessionMessageRecord {
                    turn_id: Some(turn_id),
                    message: message.clone(),
                    intent_vector: None,
                }),
                _ => SessionRecord::Message(SessionMessageRecord {
                    turn_id: None,
                    message: message.clone(),
                    intent_vector: None,
                }),
            };
            lines.push(serde_json::to_string(&record)?);
        }

        atomic_write(&path, lines.join("\n") + "\n")
    }

    pub fn append_turn(&self, session_id: &str, turn: SessionTurn) -> Result<()> {
        let mut session = match self.load(session_id) {
            Ok(session) => session,
            Err(ZcodeError::FileNotFound { .. }) => Session::new(session_id),
            Err(error) => return Err(error),
        };
        session.push(turn.user);
        if let Some(assistant) = turn.assistant {
            session.push(assistant);
        }
        self.save(&mut session)
    }

    pub fn load(&self, id: &str) -> Result<Session> {
        let path = self.find_session_path(id);
        if !path.exists() {
            return Err(ZcodeError::FileNotFound {
                path: path.display().to_string(),
            });
        }
        let content = std::fs::read_to_string(&path)?;
        if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            return Ok(serde_json::from_str(&content)?);
        }
        session_from_jsonl(&content, id)
    }

    pub fn list(&self) -> Result<Vec<Session>> {
        let mut sessions = Vec::new();
        for entry in std::fs::read_dir(&self.sessions_dir)? {
            let entry = entry?;
            let path = entry.path();
            let extension = path.extension().and_then(|ext| ext.to_str());
            if !matches!(extension, Some("jsonl") | Some("json")) {
                continue;
            }
            if let Some(session) = self.load_session_from_path(&path) {
                sessions.push(session);
            }
        }
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(sessions)
    }

    pub fn delete(&self, id: &str) -> Result<bool> {
        let mut deleted = false;
        for path in [self.session_jsonl_path(id), self.session_json_path(id)] {
            if path.exists() {
                std::fs::remove_file(path)?;
                deleted = true;
            }
        }
        let index_path = self.session_index_path(id);
        if index_path.exists() {
            std::fs::remove_dir_all(index_path)?;
        }
        Ok(deleted)
    }

    pub fn select_related_context(&self, id: &str, prompt: &str) -> Result<SessionContext> {
        self.select_related_context_with_config(id, prompt, &self.context)
    }

    pub fn select_related_context_with_config(
        &self,
        id: &str,
        prompt: &str,
        config: &SessionContextConfig,
    ) -> Result<SessionContext> {
        let session = self.load(id)?;
        select_related_context_with_lancedb(&session, prompt, config, self.session_index_path(id))
    }

    /// Compress the session by summarizing older messages and retaining key
    /// recent messages. This is deterministic and LLM-free; callers can replace
    /// the generated summary with an LLM summary before saving if desired.
    pub fn compress(&self, session: &mut Session) {
        if session.messages.len() <= self.compression.retain_recent {
            return;
        }

        let split_at = session.messages.len() - self.compression.retain_recent;
        let older = session.messages[..split_at].to_vec();
        let recent = session.messages[split_at..].to_vec();
        let generated = summarize_messages(&older, self.compression.summary_max_chars);

        session.summary = match &session.summary {
            Some(existing) if !existing.trim().is_empty() => {
                Some(format!("{}\n\n{}", existing, generated))
            }
            _ => Some(generated),
        };
        session.messages = recent;
        session.updated_at = now_secs();
    }

    pub fn session_file_path(&self, id: &str) -> PathBuf {
        self.session_jsonl_path(id)
    }

    fn load_session_from_path(&self, path: &Path) -> Option<Session> {
        let id = path.file_stem()?.to_str()?;
        let content = std::fs::read_to_string(path).ok()?;
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("jsonl") => session_from_jsonl(&content, id).ok(),
            Some("json") => serde_json::from_str::<Session>(&content).ok(),
            _ => None,
        }
    }

    fn find_session_path(&self, id: &str) -> PathBuf {
        let jsonl = self.session_jsonl_path(id);
        if jsonl.exists() {
            return jsonl;
        }
        self.session_json_path(id)
    }

    fn session_path(&self, id: &str) -> PathBuf {
        self.session_jsonl_path(id)
    }

    fn session_jsonl_path(&self, id: &str) -> PathBuf {
        self.sessions_dir.join(format!("{}.jsonl", id))
    }

    fn session_json_path(&self, id: &str) -> PathBuf {
        self.sessions_dir.join(format!("{}.json", id))
    }

    fn session_index_path(&self, id: &str) -> PathBuf {
        self.index_dir.join(sanitize_index_name(id))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum SessionRecord {
    #[serde(rename = "meta")]
    Meta(SessionMeta),
    #[serde(rename = "message")]
    Message(SessionMessageRecord),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionMeta {
    schema_version: u32,
    id: String,
    created_at: u64,
    updated_at: u64,
    summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionMessageRecord {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    turn_id: Option<u64>,
    message: ConversationMessage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    intent_vector: Option<IntentVector>,
}

pub fn select_related_context(
    session: &Session,
    prompt: &str,
    config: &SessionContextConfig,
) -> SessionContext {
    let turns = session_turns(&session.messages);
    let documents = turns
        .iter()
        .enumerate()
        .map(|(index, turn)| IntentDocument::from_text(index, &intent_text_for_turn(turn)))
        .collect();
    let index = IntentVectorIndex::new(documents);
    let matches = index.search(prompt, config.similarity_threshold, config.max_turns);
    context_from_matches(&turns, &matches)
}

fn select_related_context_with_lancedb(
    session: &Session,
    prompt: &str,
    config: &SessionContextConfig,
    index_dir: PathBuf,
) -> Result<SessionContext> {
    let turns = session_turns(&session.messages);
    let documents = turns
        .iter()
        .enumerate()
        .map(|(index, turn)| LanceIntentDocument::from_text(index, &intent_text_for_turn(turn)))
        .collect();
    let index = LanceIntentIndex::new(index_dir, documents);
    let matches = index.search(prompt, config.similarity_threshold, config.max_turns)?;
    Ok(context_from_matches(&turns, &matches))
}

fn context_from_matches<T>(
    turns: &[SessionTurn],
    matches: &[crate::intent::IntentMatch<T>],
) -> SessionContext
where
    T: Copy + Into<usize>,
{
    if matches.is_empty() {
        return SessionContext {
            messages: Vec::new(),
            matched_turns: Vec::new(),
        };
    }

    let selected_indices: BTreeSet<_> = matches.iter().map(|matched| matched.item.into()).collect();
    let matched_turns = matches
        .iter()
        .map(|matched| {
            let index: usize = matched.item.into();
            MatchedSessionTurn {
                turn_id: index as u64 + 1,
                score: matched.score,
            }
        })
        .collect();
    let messages = selected_indices
        .into_iter()
        .flat_map(|index| turn_messages(&turns[index]))
        .collect();

    SessionContext {
        messages,
        matched_turns,
    }
}

fn session_from_jsonl(content: &str, fallback_id: &str) -> Result<Session> {
    let now = now_secs();
    let mut session = Session {
        id: fallback_id.to_string(),
        created_at: now,
        updated_at: now,
        summary: None,
        messages: Vec::new(),
    };

    for line in content.lines().filter(|line| !line.trim().is_empty()) {
        let record: SessionRecord = serde_json::from_str(line)?;
        match record {
            SessionRecord::Meta(meta) => {
                session.id = meta.id;
                session.created_at = meta.created_at;
                session.updated_at = meta.updated_at;
                session.summary = meta.summary;
            }
            SessionRecord::Message(record) => session.messages.push(record.message),
        }
    }

    Ok(session)
}

fn session_turns(messages: &[ConversationMessage]) -> Vec<SessionTurn> {
    let mut turns = Vec::new();
    let mut current: Option<SessionTurn> = None;

    for message in messages {
        match message.role.as_str() {
            "user" => {
                if let Some(turn) = current.take() {
                    turns.push(turn);
                }
                current = Some(SessionTurn {
                    user: message.clone(),
                    assistant: None,
                });
            }
            "assistant" => {
                if let Some(turn) = &mut current {
                    if turn.assistant.is_none() {
                        turn.assistant = Some(message.clone());
                    }
                }
            }
            _ => {}
        }
    }

    if let Some(turn) = current {
        turns.push(turn);
    }
    turns
}

fn turn_messages(turn: &SessionTurn) -> Vec<ConversationMessage> {
    let mut messages = vec![turn.user.clone()];
    if let Some(assistant) = &turn.assistant {
        messages.push(assistant.clone());
    }
    messages
}

fn intent_text_for_turn(turn: &SessionTurn) -> String {
    intent_text_from_parts([
        turn.user.content.as_deref().unwrap_or_default(),
        turn.assistant
            .as_ref()
            .and_then(|message| message.content.as_deref())
            .unwrap_or_default(),
    ])
}

fn intent_vector_for_messages<'a>(
    messages: impl IntoIterator<Item = &'a ConversationMessage>,
) -> IntentVector {
    let parts = messages
        .into_iter()
        .filter_map(|message| message.content.as_deref());
    HashedIntentVectorizer.embed(&intent_text_from_parts(parts))
}

fn summarize_messages(messages: &[ConversationMessage], max_chars: usize) -> String {
    let mut out = String::from("Compressed session summary:\n");
    for message in messages {
        let content = message.content.as_deref().unwrap_or("");
        if content.trim().is_empty() {
            continue;
        }
        let line = format!("- {}: {}\n", message.role, content.replace('\n', " "));
        if out.len() + line.len() > max_chars {
            out.push_str("- ... summary truncated\n");
            break;
        }
        out.push_str(&line);
    }
    out
}

fn atomic_write(path: &Path, content: String) -> Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, content)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

fn sanitize_index_name(id: &str) -> String {
    id.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn manager() -> (TempDir, SessionManager) {
        let dir = TempDir::new().unwrap();
        let manager = SessionManager::new(dir.path()).unwrap();
        (dir, manager)
    }

    #[test]
    fn save_writes_one_jsonl_file_per_session() {
        let (_dir, manager) = manager();
        let mut session = manager.create("chat-1");
        session.push(ConversationMessage::user("list files"));
        session.push(ConversationMessage::assistant_text("Cargo.toml"));

        manager.save(&mut session).unwrap();

        assert!(manager.session_file_path("chat-1").exists());
        assert!(!manager.session_json_path("chat-1").exists());
        let content = std::fs::read_to_string(manager.session_file_path("chat-1")).unwrap();
        assert_eq!(content.lines().count(), 3);
        assert!(content
            .lines()
            .all(|line| serde_json::from_str::<serde_json::Value>(line).is_ok()));
    }

    #[test]
    fn load_reads_jsonl_session() {
        let (_dir, manager) = manager();
        let mut session = manager.create("chat-2");
        session.summary = Some("summary".to_string());
        session.push(ConversationMessage::user("hello"));

        manager.save(&mut session).unwrap();
        let loaded = manager.load("chat-2").unwrap();

        assert_eq!(loaded.id, "chat-2");
        assert_eq!(loaded.summary.as_deref(), Some("summary"));
        assert_eq!(loaded.messages.len(), 1);
        assert_eq!(loaded.messages[0].content.as_deref(), Some("hello"));
    }

    #[test]
    fn append_turn_keeps_single_session_file() {
        let (_dir, manager) = manager();
        manager
            .append_turn(
                "chat-3",
                SessionTurn {
                    user: ConversationMessage::user("first"),
                    assistant: Some(ConversationMessage::assistant_text("answer")),
                },
            )
            .unwrap();
        manager
            .append_turn(
                "chat-3",
                SessionTurn {
                    user: ConversationMessage::user("second"),
                    assistant: None,
                },
            )
            .unwrap();

        let loaded = manager.load("chat-3").unwrap();
        assert_eq!(loaded.messages.len(), 3);
        let files = std::fs::read_dir(manager.sessions_dir.clone())
            .unwrap()
            .count();
        assert_eq!(files, 1);
    }

    #[test]
    fn related_context_omits_unrelated_prompt() {
        let (_dir, manager) = manager();
        let mut session = manager.create("chat-4");
        session.push(ConversationMessage::user("介绍 zcode 目录结构"));
        session.push(ConversationMessage::assistant_text(
            "zcode 是 Rust workspace",
        ));
        manager.save(&mut session).unwrap();

        let context = manager
            .select_related_context("chat-4", "午餐推荐")
            .unwrap();

        assert!(context.messages.is_empty());
        assert!(context.matched_turns.is_empty());
    }

    #[test]
    fn related_context_rejects_single_weak_overlap() {
        let (_dir, manager) = manager();
        let mut session = manager.create("chat-weak-overlap");
        session.push(ConversationMessage::user("甲方案"));
        session.push(ConversationMessage::assistant_text("甲方案可行"));
        manager.save(&mut session).unwrap();

        let context = manager
            .select_related_context("chat-weak-overlap", "乙方案")
            .unwrap();

        assert!(context.messages.is_empty());
    }

    #[test]
    fn related_context_picks_semantic_turn() {
        let (_dir, manager) = manager();
        let mut session = manager.create("chat-5");
        session.push(ConversationMessage::user("介绍 zcode 工程结构"));
        session.push(ConversationMessage::assistant_text(
            "zcode 包含 session 和 orchestration crate",
        ));
        session.push(ConversationMessage::user("午餐推荐"));
        session.push(ConversationMessage::assistant_text("附近餐厅列表"));
        manager.save(&mut session).unwrap();

        let context = manager
            .select_related_context("chat-5", "继续讲 zcode 工程")
            .unwrap();

        assert_eq!(context.messages.len(), 2);
        assert_eq!(
            context.messages[0].content.as_deref(),
            Some("介绍 zcode 工程结构")
        );
        assert_eq!(context.matched_turns.len(), 1);
    }

    #[test]
    fn related_context_matches_shared_stable_anchors() {
        let (_dir, manager) = manager();
        let mut session = manager.create("chat-session");
        session.push(ConversationMessage::user("解释 session jsonl 存储机制"));
        session.push(ConversationMessage::assistant_text(
            "session 会写到 .zcode/sessions 下面",
        ));
        manager.save(&mut session).unwrap();

        let context = manager
            .select_related_context("chat-session", "继续讲 session history design")
            .unwrap();

        assert_eq!(context.messages.len(), 2);
        assert_eq!(
            context.messages[0].content.as_deref(),
            Some("解释 session jsonl 存储机制")
        );
    }

    #[test]
    fn load_legacy_json_session() {
        let (_dir, manager) = manager();
        let mut legacy = Session::new("legacy");
        legacy.push(ConversationMessage::user("legacy prompt"));
        let content = serde_json::to_string_pretty(&legacy).unwrap();
        std::fs::write(manager.session_json_path("legacy"), content).unwrap();

        let loaded = manager.load("legacy").unwrap();

        assert_eq!(loaded.messages.len(), 1);
        assert_eq!(loaded.messages[0].content.as_deref(), Some("legacy prompt"));
    }
}
