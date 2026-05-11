//! Session message storage, history loading, deletion, and compression.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use zcode_core::agent::ConversationMessage;
use zcode_core::{Result, ZcodeError};

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
    compression: CompressionConfig,
}

impl SessionManager {
    pub fn new(project_root: impl AsRef<Path>) -> Result<Self> {
        let sessions_dir = project_root.as_ref().join(".zcode").join("sessions");
        std::fs::create_dir_all(&sessions_dir)?;
        Ok(Self {
            sessions_dir,
            compression: CompressionConfig::default(),
        })
    }

    pub fn with_compression(mut self, compression: CompressionConfig) -> Self {
        self.compression = compression;
        self
    }

    pub fn create(&self, id: impl Into<String>) -> Session {
        Session::new(id)
    }

    pub fn save(&self, session: &mut Session) -> Result<()> {
        session.updated_at = now_secs();
        let path = self.session_path(&session.id);
        let json = serde_json::to_string_pretty(session)?;
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    pub fn load(&self, id: &str) -> Result<Session> {
        let path = self.session_path(id);
        if !path.exists() {
            return Err(ZcodeError::FileNotFound {
                path: path.display().to_string(),
            });
        }
        let json = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&json)?)
    }

    pub fn list(&self) -> Result<Vec<Session>> {
        let mut sessions = Vec::new();
        for entry in std::fs::read_dir(&self.sessions_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            if let Ok(json) = std::fs::read_to_string(&path) {
                if let Ok(session) = serde_json::from_str::<Session>(&json) {
                    sessions.push(session);
                }
            }
        }
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(sessions)
    }

    pub fn delete(&self, id: &str) -> Result<bool> {
        let path = self.session_path(id);
        if !path.exists() {
            return Ok(false);
        }
        std::fs::remove_file(path)?;
        Ok(true)
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

    fn session_path(&self, id: &str) -> PathBuf {
        self.sessions_dir.join(format!("{}.json", id))
    }
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

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

