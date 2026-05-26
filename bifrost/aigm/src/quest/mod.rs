//! # Quest system
//!
//! Tracks the full lifecycle of dynamically generated quests:
//!
//! ```text
//! AI GM emits AigmQuestCreate
//!   → Quest enters QuestRegistry (state: Active)
//!   → Players make progress via AigmQuestUpdate events
//!   → Quest resolves via AigmQuestComplete / AigmQuestFail
//!   → Completed quests move to history (pruned after retention window)
//! ```
//!
//! Quests are **pure data** — all state transitions happen through events.
//! The registry is an in-memory projection rebuilt from the ledger on startup.

pub mod registry;
pub mod types;

pub use registry::QuestRegistry;
pub use types::{
    Quest, QuestObjective, QuestObjectiveKind, QuestReward,
    QuestState, ObjectiveProgress,
};
