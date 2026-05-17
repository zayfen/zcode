//! Local intent-vector indexing for session context retrieval.
//!
//! The index is deterministic and dependency-free. It behaves like a small
//! vector database over the turns stored in a session file, and can be swapped
//! for a provider-backed embedding adapter later without changing callers.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeSet;

pub const VECTOR_DIMS: usize = 256;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentVector {
    values: Vec<f32>,
}

impl IntentVector {
    pub fn new(values: Vec<f32>) -> Self {
        Self { values }
    }

    pub fn values(&self) -> &[f32] {
        &self.values
    }

    pub fn cosine_similarity(&self, other: &Self) -> f32 {
        cosine_similarity(&self.values, &other.values)
    }
}

#[derive(Debug, Clone, Default)]
pub struct HashedIntentVectorizer;

impl HashedIntentVectorizer {
    pub fn embed(&self, text: &str) -> IntentVector {
        let tokens = tokenize_intent(text);
        let mut values = vec![0.0f32; VECTOR_DIMS];

        for token in tokens {
            let hash = hash_token(&token);
            let index = (hash as usize) % VECTOR_DIMS;
            let sign = if hash & 1 == 0 { 1.0 } else { -1.0 };
            values[index] += sign;
        }

        normalize(&mut values);
        IntentVector::new(values)
    }
}

#[derive(Debug, Clone)]
pub struct IntentDocument<T> {
    pub item: T,
    pub vector: IntentVector,
    pub profile: IntentProfile,
}

impl<T> IntentDocument<T> {
    pub fn from_text(item: T, text: &str) -> Self {
        let vectorizer = HashedIntentVectorizer;
        Self {
            item,
            vector: vectorizer.embed(text),
            profile: IntentProfile::analyze(text),
        }
    }
}

#[derive(Debug, Clone)]
pub struct IntentMatch<T> {
    pub item: T,
    pub score: f32,
    pub vector_score: f32,
    pub profile_score: f32,
}

#[derive(Debug, Clone)]
pub struct IntentVectorIndex<T> {
    documents: Vec<IntentDocument<T>>,
    vectorizer: HashedIntentVectorizer,
}

impl<T: Clone> IntentVectorIndex<T> {
    pub fn new(documents: Vec<IntentDocument<T>>) -> Self {
        Self {
            documents,
            vectorizer: HashedIntentVectorizer,
        }
    }

    pub fn search(&self, prompt: &str, threshold: f32, limit: usize) -> Vec<IntentMatch<T>> {
        if prompt.trim().is_empty() || limit == 0 {
            return Vec::new();
        }

        let query = self.vectorizer.embed(prompt);
        let query_profile = IntentProfile::analyze(prompt);
        let mut matches: Vec<_> = self
            .documents
            .iter()
            .filter_map(|document| {
                let vector_score = query.cosine_similarity(&document.vector);
                let relation = query_profile.relation_to(&document.profile);
                if !relation.is_related(vector_score, threshold) {
                    return None;
                }
                let profile_score = relation.score;
                let score = (vector_score * 0.45) + (profile_score * 0.55);
                Some(IntentMatch {
                    item: document.item.clone(),
                    score,
                    vector_score,
                    profile_score,
                })
            })
            .collect();

        matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        matches.truncate(limit);
        matches
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntentProfile {
    tokens: BTreeSet<String>,
    anchors: BTreeSet<String>,
    stable_anchors: BTreeSet<String>,
}

impl IntentProfile {
    pub fn analyze(text: &str) -> Self {
        let tokens = tokenize_intent(text);
        let mut profile = Self::default();

        for token in tokens {
            if is_anchor_token(&token) {
                profile.anchors.insert(token.clone());
            }
            if is_stable_anchor_token(&token) {
                profile.stable_anchors.insert(token.clone());
            }
            profile.tokens.insert(token);
        }

        profile
    }

    pub fn relation_to(&self, other: &Self) -> IntentRelation {
        let token_score = jaccard(&self.tokens, &other.tokens);
        let anchor_score = jaccard(&self.anchors, &other.anchors);
        let stable_anchor_score = jaccard(&self.stable_anchors, &other.stable_anchors);
        let shared_tokens = overlap_count(&self.tokens, &other.tokens);
        let shared_anchors = overlap_count(&self.anchors, &other.anchors);
        let shared_stable_anchors = overlap_count(&self.stable_anchors, &other.stable_anchors);
        let score = (stable_anchor_score * 0.45) + (anchor_score * 0.35) + (token_score * 0.20);

        IntentRelation {
            score: score.min(1.0),
            shared_tokens,
            shared_anchors,
            shared_stable_anchors,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IntentRelation {
    pub score: f32,
    pub shared_tokens: usize,
    pub shared_anchors: usize,
    pub shared_stable_anchors: usize,
}

impl IntentRelation {
    pub fn is_related(&self, vector_score: f32, vector_threshold: f32) -> bool {
        if self.shared_stable_anchors > 0
            && (self.score >= 0.10 || vector_score >= vector_threshold * 0.55)
        {
            return true;
        }

        if self.shared_anchors >= 2
            && (self.score >= 0.12 || vector_score >= vector_threshold * 0.70)
        {
            return true;
        }

        if self.shared_tokens >= 3 && self.score >= 0.16 {
            return true;
        }

        self.shared_tokens >= 2 && vector_score >= vector_threshold && self.score >= 0.08
    }
}

pub fn intent_text_from_parts<'a>(parts: impl IntoIterator<Item = &'a str>) -> String {
    let mut out = String::new();
    for part in parts {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(trimmed);
    }
    out
}

fn is_anchor_token(token: &str) -> bool {
    if is_stop_token(token) {
        return false;
    }
    let len = token.chars().count();
    if token.chars().any(|ch| ch.is_ascii_alphanumeric()) {
        return len >= 3;
    }
    token.chars().all(is_cjk) && len >= 2
}

fn is_stable_anchor_token(token: &str) -> bool {
    !is_stop_token(token)
        && token.chars().count() >= 3
        && token.chars().any(|ch| ch.is_ascii_alphanumeric())
}

fn overlap_count(left: &BTreeSet<String>, right: &BTreeSet<String>) -> usize {
    left.iter().filter(|item| right.contains(*item)).count()
}

fn jaccard(left: &BTreeSet<String>, right: &BTreeSet<String>) -> f32 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let intersection = left.iter().filter(|item| right.contains(*item)).count();
    if intersection == 0 {
        return 0.0;
    }
    let union = left.len() + right.len() - intersection;
    intersection as f32 / union as f32
}

fn tokenize_intent(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    let mut tokens = Vec::new();
    let mut ascii = String::new();
    let mut cjk_run = Vec::new();

    for ch in lower.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            flush_cjk(&mut cjk_run, &mut tokens);
            ascii.push(ch);
            continue;
        }

        flush_ascii(&mut ascii, &mut tokens);
        if is_cjk(ch) {
            cjk_run.push(ch);
        } else {
            flush_cjk(&mut cjk_run, &mut tokens);
        }
    }

    flush_ascii(&mut ascii, &mut tokens);
    flush_cjk(&mut cjk_run, &mut tokens);
    tokens
}

fn flush_ascii(buffer: &mut String, tokens: &mut Vec<String>) {
    if buffer.chars().count() >= 2 && !is_stop_token(buffer) {
        tokens.push(buffer.clone());
    }
    buffer.clear();
}

fn flush_cjk(buffer: &mut Vec<char>, tokens: &mut Vec<String>) {
    match buffer.len() {
        0 => {}
        1 => tokens.push(buffer[0].to_string()),
        _ => {
            for window in buffer.windows(2) {
                let token: String = window.iter().collect();
                if !is_stop_token(&token) {
                    tokens.push(token);
                }
            }
            for window in buffer.windows(3) {
                let token: String = window.iter().collect();
                if !is_stop_token(&token) {
                    tokens.push(token);
                }
            }
            if buffer.len() <= 6 {
                let token: String = buffer.iter().collect();
                if !is_stop_token(&token) {
                    tokens.push(token);
                }
            }
        }
    }
    buffer.clear();
}

fn is_stop_token(token: &str) -> bool {
    matches!(
        token,
        "the"
            | "and"
            | "for"
            | "with"
            | "this"
            | "that"
            | "what"
            | "which"
            | "who"
            | "when"
            | "where"
            | "how"
            | "can"
            | "you"
            | "please"
            | "about"
            | "today"
            | "tomorrow"
            | "yesterday"
            | "这个"
            | "一下"
            | "一个"
            | "我们"
            | "你们"
            | "继续"
            | "接着"
            | "刚才"
            | "上面"
            | "前面"
            | "之前"
            | "今天"
            | "明天"
            | "昨天"
            | "当前"
            | "现在"
    )
}

fn is_cjk(ch: char) -> bool {
    matches!(
        ch as u32,
        0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0xF900..=0xFAFF
    )
}

fn normalize(values: &mut [f32]) {
    let norm = values.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm == 0.0 {
        return;
    }
    for value in values {
        *value /= norm;
    }
}

fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    left.iter()
        .zip(right.iter())
        .map(|(left, right)| left * right)
        .sum::<f32>()
        .max(0.0)
}

fn hash_token(token: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in token.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn related_project_prompts_are_similar() {
        let vectorizer = HashedIntentVectorizer;
        let left = vectorizer.embed("介绍 zcode 工程结构");
        let right = vectorizer.embed("继续讲 zcode 工程");

        assert!(left.cosine_similarity(&right) > 0.10);
    }

    #[test]
    fn unrelated_prompts_have_low_similarity() {
        let vectorizer = HashedIntentVectorizer;
        let left = vectorizer.embed("介绍 zcode 工程结构");
        let right = vectorizer.embed("午餐推荐");

        assert!(left.cosine_similarity(&right) < 0.10);
    }

    #[test]
    fn profile_connects_shared_stable_anchors() {
        let left = IntentProfile::analyze("session jsonl storage");
        let right = IntentProfile::analyze("continue session history design");
        let relation = left.relation_to(&right);

        assert_eq!(relation.shared_stable_anchors, 1);
        assert!(relation.is_related(0.0, 0.18));
    }

    #[test]
    fn profile_rejects_single_weak_cjk_overlap() {
        let left = IntentProfile::analyze("甲方案");
        let right = IntentProfile::analyze("乙方案");
        let relation = left.relation_to(&right);

        assert_eq!(relation.shared_anchors, 1);
        assert!(!relation.is_related(0.9, 0.18));
    }
}
