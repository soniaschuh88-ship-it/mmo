//! AiContext — the LLM personality and memory component for a single NPC.
//!
//! This is the data passed to the LLM on every Layer 2 call.
//! It is persisted in the ECS and updated after each interaction.

use serde::{Deserialize, Serialize};

use super::memory::ShortTermMemory;

/// The emotional state of an NPC.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Mood {
    #[default]
    Neutral,
    Happy,
    Sad,
    Anxious,
    Angry,
    Fearful,
    Excited,
    Suspicious,
    Grieving,
    Hopeful,
}

impl std::fmt::Display for Mood {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_else(|| format!("{self:?}"));
        write!(f, "{s}")
    }
}

/// Full AI context for one NPC.
///
/// Serialised and included in every LLM system prompt for this NPC.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiContext {
    pub npc_id: String,

    /// LLM model identifier, e.g. `"ollama/llama3-8b"`, `"openrouter/mistral-7b"`.
    pub model: String,

    /// Base personality / role description — the system prompt prefix.
    /// Example: "You are Aldric, a grizzled town guard who distrusts strangers."
    pub system_prompt: String,

    /// Recent interactions (sliding window, bounded by [`ShortTermMemory::capacity`]).
    pub short_term_memory: ShortTermMemory,

    /// Opaque references into a vector DB for long-term memory retrieval.
    /// The LLM layer resolves these before building the final prompt.
    pub vector_ids: Vec<String>,

    /// What the NPC is trying to accomplish right now.
    pub current_goal: String,

    /// Current emotional state.
    pub mood: Mood,

    /// Facts the NPC knows (injected into context window each call).
    ///
    /// Bounded to the last 20 entries to control prompt size.
    pub known_facts: Vec<String>,

    /// Unix-ms timestamp of the last LLM call.
    pub last_spoken_at_ms: u64,

    /// Minimum ms between LLM calls for this NPC.
    ///
    /// Default: 5 000 ms (5 seconds). High-traffic NPCs can lower this;
    /// background NPCs should keep it at 30 000+.
    pub cooldown_ms: u64,
}

/// Maximum number of known facts kept in context.
const MAX_KNOWN_FACTS: usize = 20;

impl AiContext {
    pub fn new(
        npc_id: impl Into<String>,
        model: impl Into<String>,
        system_prompt: impl Into<String>,
        current_goal: impl Into<String>,
    ) -> Self {
        Self {
            npc_id:             npc_id.into(),
            model:              model.into(),
            system_prompt:      system_prompt.into(),
            short_term_memory:  ShortTermMemory::new(16),
            vector_ids:         Vec::new(),
            current_goal:       current_goal.into(),
            mood:               Mood::default(),
            known_facts:        Vec::new(),
            last_spoken_at_ms:  0,
            cooldown_ms:        5_000,
        }
    }

    /// True if this NPC may make an LLM call right now.
    pub fn can_speak(&self, now_ms: u64) -> bool {
        now_ms.saturating_sub(self.last_spoken_at_ms) >= self.cooldown_ms
    }

    /// Record that an LLM call was just made.
    pub fn mark_spoken(&mut self, now_ms: u64) {
        self.last_spoken_at_ms = now_ms;
    }

    /// Add a fact to the known-facts list, pruning the oldest if at capacity.
    pub fn add_fact(&mut self, fact: impl Into<String>) {
        if self.known_facts.len() >= MAX_KNOWN_FACTS {
            self.known_facts.remove(0);
        }
        self.known_facts.push(fact.into());
    }

    /// Remove a fact (e.g. once it's no longer relevant).
    pub fn remove_fact(&mut self, fact: &str) {
        self.known_facts.retain(|f| f != fact);
    }

    /// Build the context window string injected into the LLM prompt.
    ///
    /// Format:
    /// ```text
    /// [IDENTITY]
    /// {system_prompt}
    ///
    /// [CURRENT STATE]
    /// Goal: {current_goal}
    /// Mood: {mood}
    ///
    /// [KNOWN FACTS]
    /// - {fact1}
    /// - {fact2}
    ///
    /// [RECENT INTERACTIONS]
    /// {short_term_memory formatted}
    /// ```
    pub fn build_context_window(&self) -> String {
        let mut out = String::new();

        out.push_str("[IDENTITY]\n");
        out.push_str(&self.system_prompt);
        out.push_str("\n\n[CURRENT STATE]\n");
        out.push_str(&format!("Goal: {}\nMood: {}\n", self.current_goal, self.mood));

        if !self.known_facts.is_empty() {
            out.push_str("\n[KNOWN FACTS]\n");
            for fact in &self.known_facts {
                out.push_str(&format!("- {fact}\n"));
            }
        }

        let memories = self.short_term_memory.format();
        if !memories.is_empty() {
            out.push_str("\n[RECENT INTERACTIONS]\n");
            out.push_str(&memories);
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::npc::memory::MemoryEntry;

    #[test]
    fn cooldown_enforced() {
        let mut ctx = AiContext::new("aldric", "llama3", "You are Aldric.", "protect the city");
        ctx.cooldown_ms = 5_000;
        ctx.mark_spoken(10_000);
        assert!(!ctx.can_speak(14_999));
        assert!( ctx.can_speak(15_000));
    }

    #[test]
    fn known_facts_pruned_at_capacity() {
        let mut ctx = AiContext::new("n", "m", "s", "g");
        for i in 0..25 {
            ctx.add_fact(format!("fact {i}"));
        }
        assert!(ctx.known_facts.len() <= MAX_KNOWN_FACTS);
        // Oldest facts should have been pruned
        assert!(!ctx.known_facts.iter().any(|f| f == "fact 0"));
    }

    #[test]
    fn context_window_contains_goal_and_mood() {
        let ctx = AiContext::new("aldric", "m", "You are Aldric.", "protect the city");
        let window = ctx.build_context_window();
        assert!(window.contains("protect the city"));
        assert!(window.contains("neutral")); // default mood
    }

    #[test]
    fn remove_fact() {
        let mut ctx = AiContext::new("n", "m", "s", "g");
        ctx.add_fact("bandits spotted north");
        ctx.remove_fact("bandits spotted north");
        assert!(ctx.known_facts.is_empty());
    }
}
