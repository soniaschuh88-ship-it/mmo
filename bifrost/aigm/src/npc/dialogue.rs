//! Layer 2 — NPC dialogue request / response types.
//!
//! This module defines the data that flows through the LLM call path:
//!
//! ```text
//! NpcDialogueTrigger           — what caused the NPC to need an LLM response
//!   └─► NpcLlmRequest          — full prompt payload sent to the LLM
//!         └─► NpcLlmResponse   — structured answer: dialogue, mood, action, memory
//! ```
//!
//! Neither type makes the actual network call — that is the responsibility of
//! the `nova-aigm` service layer.  This crate remains `async`-free.
//!
//! ## Layer 3 cache integration
//!
//! Before dispatching a [`NpcLlmRequest`], callers should:
//!
//! 1. Call [`NpcLlmRequest::prompt_key`] to derive the BLAKE3 cache key.
//! 2. Check [`super::memory::PromptCache::get`] with that key.
//! 3. If a hit is found, deserialise the cached JSON into [`NpcLlmResponse`]
//!    and skip the LLM call entirely.
//! 4. On a miss, dispatch the request, then call
//!    [`super::memory::PromptCache::insert`] with the serialised response.

use serde::{Deserialize, Serialize};

use super::context::{AiContext, Mood};
use super::memory::PromptCache;

// ─── Trigger ─────────────────────────────────────────────────────────────────

/// What caused an NPC to need an LLM response this tick.
///
/// Triggers are ranked by default priority:
/// `PlayerSpeak > PlayerApproach > WorldEvent > Scheduled`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NpcDialogueTrigger {
    /// A player walked within greeting distance of the NPC.
    PlayerApproach {
        player_id: String,
        /// Distance in voxels at the time of the trigger.
        distance: f32,
    },
    /// A player sent a chat message addressed to (or near) this NPC.
    PlayerSpeak {
        player_id: String,
        /// The raw message text.
        text: String,
    },
    /// The AI GM emitted a world event affecting this NPC's zone.
    WorldEvent {
        /// Short identifier for the event (e.g. `"bandit_raid_started"`).
        event_name: String,
        /// Human-readable description injected into the NPC's world context.
        description: String,
    },
    /// Periodic ambient dialogue — NPC speaks unprompted.
    Scheduled,
}

impl NpcDialogueTrigger {
    /// Default priority for sorting the dialogue queue (higher = sooner).
    pub fn priority(&self) -> DialoguePriority {
        match self {
            NpcDialogueTrigger::PlayerSpeak { .. }    => DialoguePriority::High,
            NpcDialogueTrigger::PlayerApproach { .. } => DialoguePriority::Normal,
            NpcDialogueTrigger::WorldEvent { .. }     => DialoguePriority::Low,
            NpcDialogueTrigger::Scheduled             => DialoguePriority::Background,
        }
    }
}

// ─── Player context ──────────────────────────────────────────────────────────

/// Snapshot of player information injected into the NPC's LLM prompt.
///
/// Kept intentionally minimal — only data the NPC could plausibly "know".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerDialogueContext {
    pub player_id: String,
    pub player_name: String,
    pub level: u32,
    pub class: String,
    /// IDs of the player's active quests (for quest-giver NPCs).
    pub active_quest_ids: Vec<String>,
    /// Player's standing with the NPC's faction (-1000 to +1000).
    pub reputation: i32,
}

// ─── LLM request ─────────────────────────────────────────────────────────────

/// Full payload sent to the LLM for a single NPC dialogue turn.
///
/// Construct with [`NpcLlmRequest::new`], then derive the cache key with
/// [`NpcLlmRequest::prompt_key`] before dispatching.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NpcLlmRequest {
    pub npc_id: String,
    /// What triggered this request.
    pub trigger: NpcDialogueTrigger,
    /// Player-facing context (present for `PlayerSpeak` / `PlayerApproach`).
    pub player_context: Option<PlayerDialogueContext>,
    /// Recent world events, formatted as natural language.
    /// Example: "Bandits attacked the north gate. Three guards were killed."
    pub world_context: String,
    /// Full NPC AI context (personality, memory, goal, mood).
    pub ai_context: AiContext,
}

impl NpcLlmRequest {
    /// Create a new request.
    pub fn new(
        npc_id: impl Into<String>,
        trigger: NpcDialogueTrigger,
        player_context: Option<PlayerDialogueContext>,
        world_context: impl Into<String>,
        ai_context: AiContext,
    ) -> Self {
        Self {
            npc_id: npc_id.into(),
            trigger,
            player_context,
            world_context: world_context.into(),
            ai_context,
        }
    }

    /// Derive the BLAKE3 Layer-3 cache key for this request.
    ///
    /// The key covers the model, system prompt, and the full context window
    /// (goal + mood + facts + memory).  It does **not** cover the trigger or
    /// player identity — only the NPC's internal state determines whether a
    /// cached response is reusable.
    pub fn prompt_key(&self) -> [u8; 32] {
        PromptCache::compute_key(
            &self.ai_context.model,
            &self.ai_context.system_prompt,
            &self.ai_context.build_context_window(),
        )
    }

    /// Build the full system + user prompt string to send to the LLM.
    ///
    /// Format:
    /// ```text
    /// [IDENTITY]
    /// You are Aldric ...
    ///
    /// [CURRENT STATE]
    /// Goal: protect the city
    /// Mood: tense
    ///
    /// [KNOWN FACTS]
    /// - Bandits attacked last night
    ///
    /// [RECENT INTERACTIONS]
    /// [player:alice]: Do you have supplies?
    ///
    /// [WORLD CONTEXT]
    /// Tension is rising in the north.
    ///
    /// [PLAYER]
    /// Alice (level 5 warrior), reputation: 120
    ///
    /// [TRIGGER]
    /// Player says: "Where can I find the blacksmith?"
    ///
    /// Respond in JSON: {"dialogue":"...","mood":"...","action":null,"memory":null,"goal_update":null}
    /// ```
    pub fn build_prompt(&self) -> String {
        let mut out = self.ai_context.build_context_window();

        if !self.world_context.is_empty() {
            out.push_str("\n\n[WORLD CONTEXT]\n");
            out.push_str(&self.world_context);
        }

        if let Some(pc) = &self.player_context {
            out.push_str("\n\n[PLAYER]\n");
            out.push_str(&format!(
                "{} (level {} {}), reputation: {}",
                pc.player_name, pc.level, pc.class, pc.reputation
            ));
        }

        out.push_str("\n\n[TRIGGER]\n");
        match &self.trigger {
            NpcDialogueTrigger::PlayerSpeak { text, .. } => {
                out.push_str(&format!("Player says: \"{text}\""));
            }
            NpcDialogueTrigger::PlayerApproach { distance, .. } => {
                out.push_str(&format!("A player approaches ({distance:.0} voxels away)."));
            }
            NpcDialogueTrigger::WorldEvent { description, .. } => {
                out.push_str(&format!("World event: {description}"));
            }
            NpcDialogueTrigger::Scheduled => {
                out.push_str("Speak an ambient line appropriate to your current state.");
            }
        }

        out.push_str(
            "\n\nRespond in JSON: \
             {\"dialogue\":\"...\",\"mood\":\"...\",\"action\":null,\
             \"memory\":null,\"goal_update\":null}",
        );
        out
    }
}

// ─── NPC action ───────────────────────────────────────────────────────────────

/// A concrete action the NPC takes as a result of the LLM response.
///
/// Emitted alongside the dialogue line and handled by the AI GM system
/// as follow-up `WorldEvent`s (e.g. `aigm.quest.create`, `entity.despawn`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NpcAction {
    /// Offer a quest to the player.  The AI GM creates the quest.
    OfferQuest {
        /// Reference to a quest template or description.
        quest_template_id: String,
    },
    /// Open the NPC's shop UI for the interacting player.
    OpenShop,
    /// NPC turns hostile and attacks.
    Attack {
        target_player_id: String,
    },
    /// NPC flees the area.
    Flee,
    /// Give the player an item.
    GiveItem {
        item_id: String,
        quantity: u32,
    },
    /// Adjust the player's reputation with the NPC's faction.
    GrantReputation {
        faction_id: String,
        /// Positive = gain, negative = loss.
        delta: i32,
    },
    /// Move the NPC to a different zone.
    Teleport {
        zone_id: String,
        position: [f32; 3],
    },
    /// Extension point for game-specific actions.
    Custom {
        action_type: String,
        data: serde_json::Value,
    },
}

// ─── LLM response ─────────────────────────────────────────────────────────────

/// Structured response from the LLM for a single NPC dialogue turn.
///
/// Deserialised from the LLM's JSON output and then:
/// - `dialogue` → emitted as `aigm.npc.speak` event
/// - `mood`     → updates [`AiContext::mood`]
/// - `action`   → handled by the AI GM as follow-up events
/// - `memory`   → pushed to [`super::memory::ShortTermMemory`]
/// - `goal_update` → updates [`AiContext::current_goal`]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NpcLlmResponse {
    /// The line the NPC speaks aloud.
    pub dialogue: String,
    /// NPC's emotional state after this turn.
    pub mood: Mood,
    /// Optional follow-up action.
    #[serde(default)]
    pub action: Option<NpcAction>,
    /// What the NPC should remember from this interaction.
    /// Pushed to [`super::memory::ShortTermMemory`] if present.
    #[serde(default)]
    pub memory: Option<String>,
    /// Updated goal for the NPC.  Replaces [`AiContext::current_goal`] if set.
    #[serde(default)]
    pub goal_update: Option<String>,
}

impl NpcLlmResponse {
    /// Construct a minimal response (dialogue only, everything else neutral/None).
    pub fn dialogue_only(dialogue: impl Into<String>) -> Self {
        Self {
            dialogue: dialogue.into(),
            mood: Mood::Neutral,
            action: None,
            memory: None,
            goal_update: None,
        }
    }
}

// ─── Dialogue queue ───────────────────────────────────────────────────────────

/// Priority level for ordering items in the [`DialogueQueue`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DialoguePriority {
    /// Background / ambient chatter — lowest priority.
    Background = 0,
    /// World event reaction.
    Low = 1,
    /// Player approach greeting.
    Normal = 2,
    /// Direct player speech — highest priority.
    High = 3,
}

/// A single pending LLM call waiting to be dispatched.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingDialogue {
    pub npc_id: String,
    pub request: NpcLlmRequest,
    pub priority: DialoguePriority,
    /// Wall-clock time when this was enqueued (unix milliseconds).
    pub enqueued_at_ms: u64,
}

impl PendingDialogue {
    pub fn new(request: NpcLlmRequest, enqueued_at_ms: u64) -> Self {
        let priority = request.trigger.priority();
        let npc_id = request.npc_id.clone();
        Self { npc_id, request, priority, enqueued_at_ms }
    }
}

/// Maximum pending dialogue calls per zone per tick.
pub const DIALOGUE_QUEUE_CAPACITY: usize = 64;

/// Per-zone queue of pending LLM calls, ordered by priority then enqueue time.
///
/// The AI GM drains up to `max_npc_dialogues` entries per tick (see
/// [`crate::aigm::AiGmBudget`]).
#[derive(Debug, Clone, Default)]
pub struct DialogueQueue {
    items: Vec<PendingDialogue>,
}

impl DialogueQueue {
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of pending items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// True if no items are pending.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Enqueue a pending dialogue call.
    ///
    /// If the queue is at capacity, the lowest-priority item is dropped.
    /// If the new item has lower priority than all existing items and the
    /// queue is full, it is dropped silently.
    pub fn push(&mut self, pending: PendingDialogue) {
        if self.items.len() >= DIALOGUE_QUEUE_CAPACITY {
            // Find the lowest-priority item.
            let min_idx = self.items.iter().enumerate()
                .min_by_key(|(_, p)| (p.priority, p.enqueued_at_ms))
                .map(|(i, _)| i)
                .unwrap_or(0);
            if self.items[min_idx].priority <= pending.priority {
                self.items.remove(min_idx);
            } else {
                // New item has lower priority than everything — drop it.
                return;
            }
        }
        self.items.push(pending);
    }

    /// Drain up to `max_count` items in priority order (highest first),
    /// breaking ties by enqueue time (oldest first).
    ///
    /// Returns the drained items.  The queue shrinks by the returned count.
    pub fn drain(&mut self, max_count: usize) -> Vec<PendingDialogue> {
        // Sort descending priority, ascending enqueue time.
        self.items.sort_by(|a, b| {
            b.priority.cmp(&a.priority)
                .then(a.enqueued_at_ms.cmp(&b.enqueued_at_ms))
        });
        let count = max_count.min(self.items.len());
        self.items.drain(..count).collect()
    }

    /// Remove all pending items for a specific NPC (e.g. on despawn).
    pub fn remove_npc(&mut self, npc_id: &str) {
        self.items.retain(|p| p.npc_id != npc_id);
    }

    /// Clear all pending items.
    pub fn clear(&mut self) {
        self.items.clear();
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;


    fn make_ai_context(npc_id: &str) -> AiContext {
        AiContext::new(npc_id, "llama3-8b", "You are a merchant.", "sell goods")
    }

    fn make_request(npc_id: &str, trigger: NpcDialogueTrigger) -> NpcLlmRequest {
        NpcLlmRequest::new(npc_id, trigger, None, "All is calm.", make_ai_context(npc_id))
    }

    // ── Trigger priority ──────────────────────────────────────────────────────

    #[test]
    fn player_speak_highest_priority() {
        let t = NpcDialogueTrigger::PlayerSpeak {
            player_id: "p1".into(),
            text: "hello".into(),
        };
        assert_eq!(t.priority(), DialoguePriority::High);
    }

    #[test]
    fn scheduled_lowest_priority() {
        assert_eq!(NpcDialogueTrigger::Scheduled.priority(), DialoguePriority::Background);
    }

    // ── Prompt key ────────────────────────────────────────────────────────────

    #[test]
    fn same_context_same_key() {
        let r1 = make_request("aldric", NpcDialogueTrigger::Scheduled);
        let r2 = make_request("aldric", NpcDialogueTrigger::Scheduled);
        assert_eq!(r1.prompt_key(), r2.prompt_key());
    }

    #[test]
    fn different_goal_different_key() {
        let mut ctx1 = make_ai_context("aldric");
        let mut ctx2 = make_ai_context("aldric");
        ctx1.current_goal = "patrol the gates".into();
        ctx2.current_goal = "guard the vault".into();
        let r1 = NpcLlmRequest::new("aldric", NpcDialogueTrigger::Scheduled, None, "", ctx1);
        let r2 = NpcLlmRequest::new("aldric", NpcDialogueTrigger::Scheduled, None, "", ctx2);
        assert_ne!(r1.prompt_key(), r2.prompt_key());
    }

    // ── Build prompt ──────────────────────────────────────────────────────────

    #[test]
    fn prompt_contains_trigger_text() {
        let req = make_request(
            "merchant",
            NpcDialogueTrigger::PlayerSpeak {
                player_id: "p1".into(),
                text: "How much for the sword?".into(),
            },
        );
        let prompt = req.build_prompt();
        assert!(prompt.contains("How much for the sword?"));
        assert!(prompt.contains("[TRIGGER]"));
    }

    #[test]
    fn prompt_contains_world_context() {
        let req = NpcLlmRequest::new(
            "aldric",
            NpcDialogueTrigger::Scheduled,
            None,
            "Bandits attacked the north gate.",
            make_ai_context("aldric"),
        );
        let prompt = req.build_prompt();
        assert!(prompt.contains("Bandits attacked the north gate."));
    }

    #[test]
    fn prompt_contains_json_instruction() {
        let req = make_request("m", NpcDialogueTrigger::Scheduled);
        assert!(req.build_prompt().contains("Respond in JSON"));
    }

    // ── NpcLlmResponse ────────────────────────────────────────────────────────

    #[test]
    fn dialogue_only_response_has_neutral_mood() {
        let resp = NpcLlmResponse::dialogue_only("Hello traveller.");
        assert_eq!(resp.mood, Mood::Neutral);
        assert!(resp.action.is_none());
        assert!(resp.memory.is_none());
        assert!(resp.goal_update.is_none());
    }

    #[test]
    fn response_round_trips_json() {
        let resp = NpcLlmResponse {
            dialogue: "Buy something!".into(),
            mood: Mood::Happy,
            action: Some(NpcAction::OpenShop),
            memory: Some("Player bought a sword".into()),
            goal_update: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let back: NpcLlmResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp, back);
    }

    // ── DialogueQueue ─────────────────────────────────────────────────────────

    #[test]
    fn drain_returns_highest_priority_first() {
        let mut q = DialogueQueue::new();
        q.push(PendingDialogue::new(
            make_request("a", NpcDialogueTrigger::Scheduled), 1000,
        ));
        q.push(PendingDialogue::new(
            make_request("b", NpcDialogueTrigger::PlayerSpeak {
                player_id: "p1".into(), text: "hi".into(),
            }), 2000,
        ));
        let drained = q.drain(1);
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0].npc_id, "b"); // high priority first
    }

    #[test]
    fn drain_respects_max_count() {
        let mut q = DialogueQueue::new();
        for i in 0..5 {
            q.push(PendingDialogue::new(
                make_request(&i.to_string(), NpcDialogueTrigger::Scheduled), i,
            ));
        }
        let drained = q.drain(3);
        assert_eq!(drained.len(), 3);
        assert_eq!(q.len(), 2);
    }

    #[test]
    fn remove_npc_clears_its_items() {
        let mut q = DialogueQueue::new();
        q.push(PendingDialogue::new(make_request("aldric", NpcDialogueTrigger::Scheduled), 0));
        q.push(PendingDialogue::new(make_request("barkeep", NpcDialogueTrigger::Scheduled), 0));
        q.remove_npc("aldric");
        assert_eq!(q.len(), 1);
        assert_eq!(q.items[0].npc_id, "barkeep");
    }

    #[test]
    fn memory_entry_push_after_response() {
        // Simulate what the AI GM does after receiving a response.
        let mut ctx = make_ai_context("aldric");
        let resp = NpcLlmResponse {
            dialogue: "Beware the swamps.".into(),
            mood: Mood::Anxious,
            action: None,
            memory: Some("Warned player about swamps".into()),
            goal_update: Some("warn travellers of swamps".into()),
        };

        if let Some(mem) = &resp.memory {
            ctx.short_term_memory.push(
                crate::npc::memory::MemoryEntry::new("npc:aldric", mem.clone(), 5000),
            );
        }
        if let Some(goal) = &resp.goal_update {
            ctx.current_goal = goal.clone();
        }
        ctx.mood = resp.mood.clone();

        assert_eq!(ctx.mood, Mood::Anxious);
        assert_eq!(ctx.current_goal, "warn travellers of swamps");
        assert!(!ctx.short_term_memory.is_empty());
    }
}
