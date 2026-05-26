//! # Story system
//!
//! Drives the macro-level narrative of the world.
//!
//! ## Concepts
//!
//! - [`WorldMood`]  — the emotional tone of the world right now
//!                    (calm → tense → war → catastrophe → recovery).
//! - [`StoryBeat`]  — a single narrative event within an arc
//!                    (the bandit chief appears / the city falls / a hero rises).
//! - [`StoryArc`]   — an ordered sequence of beats forming a complete story.
//! - [`StoryEngine`] — projects ledger events onto arc/beat state and decides
//!                    when the next beat should fire.
//!
//! ## Beat triggering rules
//!
//! A beat fires when ALL of its trigger conditions are satisfied:
//!
//! ```text
//! TriggerCondition::MinTick          — world tick >= N
//! TriggerCondition::QuestCompleted   — quest_id is in completed set
//! TriggerCondition::NpcDead          — npc_id is in dead set
//! TriggerCondition::PlayerCount      — active players >= N
//! TriggerCondition::MoodIs           — current world mood == M
//! TriggerCondition::PreviousBeat     — beat_id is already completed
//! ```
//!
//! The engine checks all pending beats every tick via [`StoryEngine::tick`].

pub mod arc;
pub mod beat;
pub mod engine;
pub mod mood;

pub use arc::StoryArc;
pub use beat::{StoryBeat, BeatState, TriggerCondition};
pub use engine::StoryEngine;
pub use mood::WorldMood;
