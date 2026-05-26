//! # bifrost-aigm — AI Game Master
//!
//! Event-driven story, quest, and NPC layer for the NOVA voxel engine.
//!
//! ## Architecture
//!
//! The AI GM is a **pure event transformer**:
//!
//! ```text
//! WorldEvents (in) → AiGmTick → WorldEvents (out)
//! ```
//!
//! It never mutates the voxel world directly. Every world change it wants
//! to make is expressed as a `WorldEvent` emitted into the ledger.
//!
//! ## Three-layer NPC model
//!
//! ```text
//! Layer 1 — Reactive (every tick, O(1))
//!   └─ State machine: patrol → idle → chase → flee → speak
//!
//! Layer 2 — LLM trigger (event-driven, async, rate-limited)
//!   └─ Fires only on: player_speak | world_event | quest_trigger
//!
//! Layer 3 — Cached responses (BLAKE3-keyed prompt cache)
//!   └─ Identical context → identical response, no LLM call needed
//! ```
//!
//! ## Event budget
//!
//! Each [`AiGmTick`] enforces hard caps to prevent runaway event storms:
//!
//! ```text
//! maxAIEvents        = 50  per zone per tick
//! maxQuestCreates    =  3  per tick
//! maxStoryBeats      =  1  per tick
//! maxNpcDialogues    = 10  per tick
//! ```
//!
//! ## Key types
//!
//! - [`WorldEvent`] — the universal event ledger entry
//! - [`QuestRegistry`] — tracks active / completed quests per zone
//! - [`StoryEngine`] — drives narrative arcs and world mood
//! - [`NpcRegistry`] — NPC state, 3-layer behaviour, dialogue queue
//! - [`AiGmState`] — world-level GM context
//! - [`AiGmTick`] — single tick execution with event budget enforcement

pub mod event;
pub mod quest;
pub mod story;
pub mod npc;
pub mod aigm;

pub use event::{WorldEvent, EventType, AuthorId, EventPayload};
pub use quest::{Quest, QuestObjective, QuestReward, QuestRegistry};
pub use story::{StoryArc, StoryBeat, WorldMood, StoryEngine};
pub use npc::{NpcState, AiContext, NpcBehavior, NpcDialogueTrigger, NpcRegistry, NpcTickInput};
pub use aigm::{AiGmState, AiGmTick, AiGmError, TickBudget};
