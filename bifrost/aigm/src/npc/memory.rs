//! Layer 3 — NPC memory primitives.
//!
//! ## Short-term memory
//!
//! A bounded sliding-window log of recent interactions. Stored on the NPC's
//! [`AiContext`][super::context::AiContext] and injected into every LLM prompt.
//!
//! ## Prompt cache
//!
//! A BLAKE3-keyed ring buffer mapping exact prompt fingerprints to serialised
//! LLM responses.  When the same context recurs, the cached response is
//! replayed without calling the LLM.
//!
//! Cache key derivation:
//! ```text
//! key = BLAKE3(model_id || system_prompt || context_window)
//! ```
//!
//! The cache is **per-zone** and bounded to [`PROMPT_CACHE_CAPACITY`] entries.

use serde::{Deserialize, Serialize};

// ─── Memory entry ─────────────────────────────────────────────────────────────

/// A single entry in an NPC's short-term interaction log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Who spoke (e.g. `"player:uuid"`, `"npc:aldric"`, `"system"`).
    pub speaker: String,
    /// What was said or what happened.
    pub text: String,
    /// Wall-clock timestamp (unix milliseconds).
    pub ts_ms: u64,
}

impl MemoryEntry {
    pub fn new(speaker: impl Into<String>, text: impl Into<String>, ts_ms: u64) -> Self {
        Self { speaker: speaker.into(), text: text.into(), ts_ms }
    }

    /// Compact one-liner for LLM prompt injection: `"[speaker]: text"`.
    pub fn format_line(&self) -> String {
        format!("[{}]: {}", self.speaker, self.text)
    }
}

// ─── Short-term memory ────────────────────────────────────────────────────────

/// A bounded sliding-window log of recent NPC interactions.
///
/// Older entries are evicted when the capacity is reached.  The capacity is
/// fixed at construction.
///
/// # Example
///
/// ```rust
/// use bifrost_aigm::npc::memory::{ShortTermMemory, MemoryEntry};
///
/// let mut mem = ShortTermMemory::new(4);
/// mem.push(MemoryEntry::new("player:alice", "Hello!", 0));
/// mem.push(MemoryEntry::new("npc:aldric",   "Greetings.", 1000));
/// println!("{}", mem.format());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShortTermMemory {
    /// Chronological log of entries (oldest first).
    entries: Vec<MemoryEntry>,
    /// Maximum number of entries retained.
    capacity: usize,
}

impl ShortTermMemory {
    /// Create a new memory store with the given capacity.
    ///
    /// `capacity` is clamped to a minimum of 1.
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self { entries: Vec::with_capacity(capacity), capacity }
    }

    /// Number of entries currently stored.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when no entries are stored.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The configured maximum number of entries.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Append a new entry, evicting the oldest if at capacity.
    pub fn push(&mut self, entry: MemoryEntry) {
        if self.entries.len() >= self.capacity {
            self.entries.remove(0);
        }
        self.entries.push(entry);
    }

    /// Iterate over entries from oldest to newest.
    pub fn iter(&self) -> impl Iterator<Item = &MemoryEntry> {
        self.entries.iter()
    }

    /// Format as a multi-line string for injection into an LLM prompt.
    ///
    /// Returns an empty string if there are no entries.
    ///
    /// ```text
    /// [player:alice]: I need supplies.
    /// [npc:aldric]: What do you want, stranger?
    /// ```
    pub fn format(&self) -> String {
        self.entries.iter()
            .map(MemoryEntry::format_line)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Remove all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

// ─── Prompt cache ─────────────────────────────────────────────────────────────

/// Maximum number of entries stored in a single [`PromptCache`].
pub const PROMPT_CACHE_CAPACITY: usize = 512;

/// A BLAKE3-keyed FIFO ring-buffer caching serialised LLM responses.
///
/// ## Purpose
///
/// Identical NPC contexts produce identical responses.  Rather than calling
/// the LLM every time a player triggers the same NPC greeting, the cache
/// intercepts the call and returns the stored response instantly.
///
/// ## Key derivation
///
/// ```text
/// key = BLAKE3(model_id || system_prompt || context_window)
/// ```
///
/// Use [`PromptCache::compute_key`] to derive the key from an
/// [`AiContext`][super::context::AiContext] before checking the cache.
///
/// ## Value format
///
/// Values are stored as raw JSON strings.  The dialogue layer serialises
/// [`super::dialogue::NpcLlmResponse`] before insertion and deserialises
/// after retrieval.  This keeps `memory` free of cross-module dependencies.
///
/// ## Eviction policy
///
/// FIFO ring buffer — the oldest entry is dropped when capacity is reached.
/// Identical keys inserted twice overwrite the existing entry in-place without
/// changing its position in the eviction queue.
#[derive(Debug, Clone, Default)]
pub struct PromptCache {
    /// Ordered insertion log (oldest first) — parallel to `values`.
    keys: Vec<[u8; 32]>,
    /// Cached JSON response strings — parallel to `keys`.
    values: Vec<String>,
}

impl PromptCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Compute the BLAKE3 cache key for a given prompt context.
    ///
    /// Inputs:
    /// - `model_id`       — LLM model identifier (e.g. `"ollama/llama3-8b"`)
    /// - `system_prompt`  — NPC personality / role description
    /// - `context_window` — full context string from [`super::context::AiContext::build_context_window`]
    pub fn compute_key(model_id: &str, system_prompt: &str, context_window: &str) -> [u8; 32] {
        let mut h = blake3::Hasher::new();
        h.update(model_id.as_bytes());
        h.update(b"\x00"); // domain separator
        h.update(system_prompt.as_bytes());
        h.update(b"\x00");
        h.update(context_window.as_bytes());
        *h.finalize().as_bytes()
    }

    /// Look up a cached response by key.
    ///
    /// Returns the serialised JSON string if found, `None` on a cache miss.
    pub fn get(&self, key: &[u8; 32]) -> Option<&str> {
        self.keys.iter().position(|k| k == key)
            .map(|i| self.values[i].as_str())
    }

    /// Insert or update a key–value pair.
    ///
    /// If `key` already exists, the value is updated in-place (no eviction).
    /// If the cache is full, the oldest entry is evicted before inserting.
    pub fn insert(&mut self, key: [u8; 32], value: String) {
        if let Some(i) = self.keys.iter().position(|k| k == &key) {
            self.values[i] = value;
            return;
        }
        if self.keys.len() >= PROMPT_CACHE_CAPACITY {
            self.keys.remove(0);
            self.values.remove(0);
        }
        self.keys.push(key);
        self.values.push(value);
    }

    /// Number of cached entries.
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// True if the cache contains no entries.
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Remove all entries.
    pub fn clear(&mut self) {
        self.keys.clear();
        self.values.clear();
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── ShortTermMemory ───────────────────────────────────────────────────────

    #[test]
    fn push_within_capacity() {
        let mut mem = ShortTermMemory::new(4);
        for i in 0..4u64 {
            mem.push(MemoryEntry::new("player:1", format!("msg {i}"), i * 1000));
        }
        assert_eq!(mem.len(), 4);
    }

    #[test]
    fn evicts_oldest_at_capacity() {
        let mut mem = ShortTermMemory::new(3);
        mem.push(MemoryEntry::new("p", "first",  1000));
        mem.push(MemoryEntry::new("p", "second", 2000));
        mem.push(MemoryEntry::new("p", "third",  3000));
        mem.push(MemoryEntry::new("p", "fourth", 4000));
        assert_eq!(mem.len(), 3);
        assert!(!mem.format().contains("first"), "oldest entry should be evicted");
        assert!(mem.format().contains("second"));
        assert!(mem.format().contains("fourth"));
    }

    #[test]
    fn format_empty_returns_empty_string() {
        let mem = ShortTermMemory::new(8);
        assert_eq!(mem.format(), "");
    }

    #[test]
    fn format_multiline() {
        let mut mem = ShortTermMemory::new(8);
        mem.push(MemoryEntry::new("player:alice", "hello",    1000));
        mem.push(MemoryEntry::new("npc:aldric",   "greetings", 2000));
        let fmt = mem.format();
        assert!(fmt.contains("[player:alice]: hello"));
        assert!(fmt.contains("[npc:aldric]: greetings"));
    }

    #[test]
    fn clear_empties() {
        let mut mem = ShortTermMemory::new(4);
        mem.push(MemoryEntry::new("p", "hi", 0));
        mem.clear();
        assert!(mem.is_empty());
        assert_eq!(mem.len(), 0);
    }

    #[test]
    fn capacity_clamped_to_one() {
        let mem = ShortTermMemory::new(0);
        assert_eq!(mem.capacity(), 1);
    }

    // ── PromptCache ───────────────────────────────────────────────────────────

    #[test]
    fn prompt_cache_miss_returns_none() {
        let cache = PromptCache::new();
        let key = PromptCache::compute_key("llama3", "You are Aldric.", "ctx");
        assert!(cache.get(&key).is_none());
    }

    #[test]
    fn cache_hit_after_insert() {
        let mut cache = PromptCache::new();
        let key = PromptCache::compute_key("llama3", "You are Aldric.", "ctx");
        cache.insert(key, r#"{"dialogue":"Hello!"}"#.to_string());
        assert_eq!(cache.get(&key), Some(r#"{"dialogue":"Hello!"}"#));
    }

    #[test]
    fn different_models_produce_different_keys() {
        let k1 = PromptCache::compute_key("llama3",    "prompt", "ctx");
        let k2 = PromptCache::compute_key("mistral-7b", "prompt", "ctx");
        assert_ne!(k1, k2);
    }

    #[test]
    fn different_prompts_produce_different_keys() {
        let k1 = PromptCache::compute_key("m", "prompt-a", "ctx");
        let k2 = PromptCache::compute_key("m", "prompt-b", "ctx");
        assert_ne!(k1, k2);
    }

    #[test]
    fn update_in_place_does_not_grow() {
        let mut cache = PromptCache::new();
        let key = PromptCache::compute_key("m", "s", "c");
        cache.insert(key, "v1".to_string());
        cache.insert(key, "v2".to_string());
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&key), Some("v2"));
    }

    #[test]
    fn prompt_cache_evicts_oldest_at_capacity() {
        let mut cache = PromptCache::new();
        // Fill to capacity
        for i in 0..PROMPT_CACHE_CAPACITY {
            let key = PromptCache::compute_key("m", "s", &i.to_string());
            cache.insert(key, format!("v{i}"));
        }
        assert_eq!(cache.len(), PROMPT_CACHE_CAPACITY);

        // Insert one more — oldest (i=0) should be evicted
        let first_key = PromptCache::compute_key("m", "s", "0");
        let overflow_key = PromptCache::compute_key("m", "s", "overflow");
        cache.insert(overflow_key, "voverflow".to_string());
        assert_eq!(cache.len(), PROMPT_CACHE_CAPACITY);
        assert!(cache.get(&first_key).is_none(), "oldest entry should be evicted");
        assert_eq!(cache.get(&overflow_key), Some("voverflow"));
    }

    #[test]
    fn prompt_cache_clear_empties() {
        let mut cache = PromptCache::new();
        let key = PromptCache::compute_key("m", "s", "c");
        cache.insert(key, "v".to_string());
        cache.clear();
        assert!(cache.is_empty());
        assert!(cache.get(&key).is_none());
    }
}
