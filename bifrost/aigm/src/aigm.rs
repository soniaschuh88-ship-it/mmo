//! AI Game Master — world-level tick coordinator.
//!
//! [`AiGmState`] owns all zone-level AI GM state and wires together the
//! three NPC layers with the quest and story systems:
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────┐
//! │  AiGmState::tick(incoming_events, npc_input, budget, …)      │
//! │                                                              │
//! │  1. Ingest incoming WorldEvents                              │
//! │     ├── apply to QuestRegistry                               │
//! │     ├── apply to StoryEngine                                 │
//! │     └── append to recent_events window (max 200)             │
//! │                                                              │
//! │  2. Drive Layer 1 NPC state machines                         │
//! │     └── collect Layer 2 dialogue triggers (≤ max_npc_dialogues) │
//! │                                                              │
//! │  3. Fire story beats (≤ max_story_beats)                     │
//! │     └── wrap StoryBeatPayload → WorldEvent                   │
//! │                                                              │
//! │  Returns AiGmTick { events_out, pending_dialogues, budget_used } │
//! └──────────────────────────────────────────────────────────────┘
//! ```
//!
//! This crate is `async`-free.  The caller (nova-aigm service) is
//! responsible for dispatching [`PendingDialogue`] items to the LLM and
//! feeding responses back through the event ledger.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::event::{
    AuthorId, EventPayload, EventType, WorldEvent,
};
use crate::npc::dialogue::{DialogueQueue, PendingDialogue};
use crate::npc::memory::PromptCache;
use crate::npc::registry::{NpcRegistry, NpcTickInput};
use crate::quest::QuestRegistry;
use crate::story::StoryEngine;

// ─── TickBudget ───────────────────────────────────────────────────────────────

/// Hard per-tick event caps.  Enforced by [`AiGmState::tick`] to prevent
/// runaway event storms from flooding the ledger.
///
/// # Defaults
///
/// | Field              | Default |
/// |--------------------|---------|
/// | `max_ai_events`    |  50     |
/// | `max_quest_creates`|   3     |
/// | `max_story_beats`  |   1     |
/// | `max_npc_dialogues`|  10     |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TickBudget {
    /// Maximum AI-authored events emitted to the ledger per tick.
    pub max_ai_events: u32,
    /// Maximum `AigmQuestCreate` events per tick.
    pub max_quest_creates: u32,
    /// Maximum story-beat events per tick.
    pub max_story_beats: u32,
    /// Maximum NPC dialogue (LLM) calls queued per tick.
    pub max_npc_dialogues: u32,
}

impl Default for TickBudget {
    fn default() -> Self {
        Self {
            max_ai_events:      50,
            max_quest_creates:   3,
            max_story_beats:     1,
            max_npc_dialogues:  10,
        }
    }
}

// ─── BudgetUsage ─────────────────────────────────────────────────────────────

/// How much of each [`TickBudget`] slot was consumed in one tick.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct BudgetUsage {
    pub ai_events: u32,
    pub quest_creates: u32,
    pub story_beats: u32,
    pub npc_dialogues: u32,
}

// ─── AiGmTick ────────────────────────────────────────────────────────────────

/// Result of executing one [`AiGmState::tick`].
///
/// The caller (nova-aigm) should:
/// 1. Append `events_out` to the ledger.
/// 2. Dispatch `pending_dialogues` to the LLM asynchronously.
/// 3. Feed LLM responses back as `aigm.npc.speak` + `aigm.npc.goal_set` events
///    in the **next** tick's `incoming_events`.
#[derive(Debug)]
pub struct AiGmTick {
    /// Events the AI GM wants to emit to the ledger this tick.
    pub events_out: Vec<WorldEvent>,
    /// Layer 2 NPC dialogue requests ready for LLM dispatch.
    ///
    /// Already filtered by [`TickBudget::max_npc_dialogues`] and the NPC's
    /// per-NPC cooldown ([`AiContext::can_speak`]).
    pub pending_dialogues: Vec<PendingDialogue>,
    /// Breakdown of budget consumed.
    pub budget_used: BudgetUsage,
}

// ─── AiGmError ───────────────────────────────────────────────────────────────

/// Error type for AI GM operations.
///
/// Tick execution itself does **not** return errors — budget overruns are
/// handled by silently dropping excess items.  This error is returned by
/// utility methods that build or validate AI GM state.
#[derive(Debug, Error)]
pub enum AiGmError {
    /// An operation tried to use more budget than allowed.
    #[error("budget exhausted: {budget} reached limit of {limit}")]
    BudgetExhausted { budget: &'static str, limit: u32 },

    /// A JSON serialisation / deserialisation failure.
    #[error("serialisation error: {0}")]
    Serialisation(#[from] serde_json::Error),

    /// An NPC referenced by id was not found in the registry.
    #[error("NPC not found: {npc_id}")]
    NpcNotFound { npc_id: String },

    /// A quest referenced by id was not found in the registry.
    #[error("quest not found: {quest_id}")]
    QuestNotFound { quest_id: String },
}

// ─── AiGmState ───────────────────────────────────────────────────────────────

/// Maximum number of recent events kept in [`AiGmState::recent_events`].
const MAX_RECENT_EVENTS: usize = 200;

/// World-level AI Game Master state for one zone.
///
/// Owns the three-layer NPC system, the quest registry, the story engine, the
/// dialogue queue, and the prompt cache.
///
/// # Example
///
/// ```rust,ignore
/// let mut gm = AiGmState::new("world-alpha", "ollama/llama3-8b");
/// gm.npc_registry.insert(NpcState::new(/* ... */));
///
/// let result = gm.tick(
///     &incoming_events,
///     &npc_input,
///     TickBudget::default(),
///     now_ms,
///     current_tick,
/// );
/// // Send result.events_out to the ledger.
/// // Dispatch result.pending_dialogues to the LLM.
/// ```
#[derive(Debug)]
pub struct AiGmState {
    /// Stable world identifier.
    pub world_id: String,

    /// NPC state, Layer 1 state machine, Layer 2/3 AI context.
    pub npc_registry: NpcRegistry,

    /// Active and historical quest state.
    pub quest_registry: QuestRegistry,

    /// Story arc and beat engine.
    pub story_engine: StoryEngine,

    /// Pending LLM calls awaiting dispatch.
    pub dialogue_queue: DialogueQueue,

    /// Layer 3 BLAKE3-keyed prompt response cache.
    pub prompt_cache: PromptCache,

    /// Sliding window of recent `WorldEvent`s (context for AI decisions).
    ///
    /// Limited to [`MAX_RECENT_EVENTS`] entries; oldest are evicted first.
    pub recent_events: Vec<WorldEvent>,

    /// Number of connected players.  Influences quest / beat generation rate.
    pub active_player_count: u32,

    /// Default LLM model identifier for NPC dialogue calls.
    pub model: String,

    /// Minimum ticks between AI GM quest-create events.
    pub quest_cooldown_ticks: u64,

    /// Tick at which the last `AigmQuestCreate` event was emitted.
    pub last_quest_at_tick: u64,

    /// Monotonically increasing sequence number.
    ///
    /// Incremented by [`AiGmState::next_seq`] when building outbound events.
    /// Should be initialised from the ledger head on startup.
    pub head_seq: u64,

    /// BLAKE3 integrity chain head.
    ///
    /// Updated each time an incoming event is ingested and each time an
    /// outbound event is built.
    pub head_hash: [u8; 32],
}

impl AiGmState {
    /// Create a new, empty AI GM state for `world_id`.
    pub fn new(world_id: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            world_id: world_id.into(),
            npc_registry: NpcRegistry::new(),
            quest_registry: QuestRegistry::new(),
            story_engine: StoryEngine::new(),
            dialogue_queue: DialogueQueue::new(),
            prompt_cache: PromptCache::new(),
            recent_events: Vec::new(),
            active_player_count: 0,
            model: model.into(),
            quest_cooldown_ticks: 1_000,
            last_quest_at_tick: 0,
            head_seq: 0,
            head_hash: [0u8; 32],
        }
    }

    // ── Ledger integration ────────────────────────────────────────────────────

    /// Consume the next sequence number.
    ///
    /// Call this when building a new outbound [`WorldEvent`].
    pub fn next_seq(&mut self) -> u64 {
        self.head_seq += 1;
        self.head_seq
    }

    /// Ingest a batch of incoming events into all internal registries.
    ///
    /// Events are processed in ascending `seq` order (as received from the
    /// ledger).  This method updates:
    /// - `recent_events` window
    /// - `quest_registry` via [`QuestRegistry::apply_event`]
    /// - `story_engine` via [`StoryEngine::apply_event`]
    /// - `head_seq` and `head_hash` (chain integrity)
    pub fn ingest(&mut self, events: &[WorldEvent]) {
        for event in events {
            // Maintain sliding window.
            if self.recent_events.len() >= MAX_RECENT_EVENTS {
                self.recent_events.remove(0);
            }
            self.recent_events.push(event.clone());

            // Project into sub-systems.
            let _ = self.quest_registry.apply_event(event);
            self.story_engine.apply_event(event);

            // Advance chain head.
            if event.seq >= self.head_seq {
                self.head_seq = event.seq;
                self.head_hash = event.world_hash;
            }
        }
    }

    // ── Tick ──────────────────────────────────────────────────────────────────

    /// Execute one AI GM tick.
    ///
    /// Steps:
    /// 1. Ingest `incoming_events` (quest / story projection, chain advance).
    /// 2. Drive all NPCs through Layer 1; collect Layer 2 dialogue triggers.
    /// 3. Fire ready story beats (≤ `budget.max_story_beats`).
    ///
    /// Budget overruns are handled by dropping excess items — this method
    /// never panics or returns an error for budget violations.
    ///
    /// # Arguments
    ///
    /// * `incoming_events` — Events from the ledger since the last tick.
    /// * `npc_input`       — Per-NPC player proximity and dialogue data.
    /// * `budget`          — Hard caps for this tick.
    /// * `now_ms`          — Wall-clock time (unix ms), used for NPC cooldowns.
    /// * `current_tick`    — Authoritative simulation tick number.
    pub fn tick(
        &mut self,
        incoming_events: &[WorldEvent],
        npc_input: &NpcTickInput<'_>,
        budget: TickBudget,
        now_ms: u64,
        current_tick: u64,
    ) -> AiGmTick {
        let mut usage = BudgetUsage::default();
        let mut events_out: Vec<WorldEvent> = Vec::new();

        // Step 1: ingest.
        self.ingest(incoming_events);

        // Step 2: Layer 1 NPC state machines → Layer 2 dialogue triggers.
        let all_pending = self.npc_registry.tick(npc_input, now_ms);
        let mut pending_dialogues: Vec<PendingDialogue> = Vec::new();

        for p in all_pending {
            if usage.npc_dialogues >= budget.max_npc_dialogues {
                break;
            }
            usage.npc_dialogues += 1;
            pending_dialogues.push(p);
        }

        // Step 3: Story beats.
        let beat_payloads = self.story_engine.tick(current_tick, self.active_player_count);

        for payload in beat_payloads {
            if usage.story_beats >= budget.max_story_beats {
                break;
            }
            if usage.ai_events >= budget.max_ai_events {
                break;
            }
            let seq = self.next_seq();
            let event = WorldEvent::new(
                seq,
                EventType::AigmStoryBeat,
                EventPayload::AigmStoryBeat(payload),
                AuthorId::AiGm,
                &self.head_hash,
                &self.world_id,
                now_ms,
            );
            // Advance our own chain head with the outbound event.
            self.head_hash = event.world_hash;
            events_out.push(event);
            usage.story_beats += 1;
            usage.ai_events += 1;
        }

        AiGmTick { events_out, pending_dialogues, budget_used: usage }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::npc::registry::{NpcTickInput};
    use crate::npc::{NpcState, AiContext};
    use crate::npc::behavior::{BehaviorConfig, NpcFaction};

    fn friendly(id: &str) -> NpcState {
        NpcState::new(
            id,
            AiContext::new(id, "llama3", "You are friendly.", "help"),
            BehaviorConfig { faction: NpcFaction::Friendly, ..Default::default() },
            100,
            [0.0; 3],
            "zone-a",
        )
    }

    #[test]
    fn new_state_is_empty() {
        let gm = AiGmState::new("world-1", "llama3");
        assert!(gm.npc_registry.is_empty());
        assert_eq!(gm.head_seq, 0);
    }

    #[test]
    fn ingest_advances_head() {
        let mut gm = AiGmState::new("w", "m");
        let genesis = [0u8; 32];
        let event = WorldEvent::new(
            1,
            EventType::ZoneLoad,
            EventPayload::ZoneLoad(crate::event::ZonePayload { zone_id: "zone-a".into() }),
            AuthorId::System,
            &genesis,
            "zone-a",
            1_000,
        );
        gm.ingest(&[event]);
        assert_eq!(gm.head_seq, 1);
        assert_eq!(gm.recent_events.len(), 1);
    }

    #[test]
    fn tick_with_no_npcs_produces_no_dialogues() {
        let mut gm = AiGmState::new("w", "m");
        let input = NpcTickInput::default();
        let result = gm.tick(&[], &input, TickBudget::default(), 0, 1);
        assert!(result.pending_dialogues.is_empty());
    }

    #[test]
    fn tick_triggers_npc_dialogue_on_player_speak() {
        let mut gm = AiGmState::new("w", "m");
        gm.npc_registry.insert(friendly("aldric"));

        let mut input = NpcTickInput::default();
        input.speaking_to.insert("aldric", ("player-1", "What news?"));

        let result = gm.tick(&[], &input, TickBudget::default(), 0, 1);
        assert_eq!(result.pending_dialogues.len(), 1);
        assert_eq!(result.pending_dialogues[0].npc_id, "aldric");
    }

    #[test]
    fn budget_caps_npc_dialogues() {
        let mut gm = AiGmState::new("w", "m");
        for i in 0..5 {
            gm.npc_registry.insert(friendly(&format!("npc-{i}")));
        }

        let mut input = NpcTickInput::default();
        for i in 0..5 {
            input.speaking_to.insert(
                Box::leak(format!("npc-{i}").into_boxed_str()),
                ("player-1", "hello"),
            );
        }

        let budget = TickBudget { max_npc_dialogues: 2, ..Default::default() };
        let result = gm.tick(&[], &input, budget, 0, 1);
        assert!(result.budget_used.npc_dialogues <= 2);
        assert!(result.pending_dialogues.len() <= 2);
    }

    #[test]
    fn next_seq_increments() {
        let mut gm = AiGmState::new("w", "m");
        assert_eq!(gm.next_seq(), 1);
        assert_eq!(gm.next_seq(), 2);
        assert_eq!(gm.next_seq(), 3);
    }

    #[test]
    fn tick_budget_default_values() {
        let b = TickBudget::default();
        assert_eq!(b.max_ai_events,     50);
        assert_eq!(b.max_quest_creates,  3);
        assert_eq!(b.max_story_beats,    1);
        assert_eq!(b.max_npc_dialogues, 10);
    }
}
