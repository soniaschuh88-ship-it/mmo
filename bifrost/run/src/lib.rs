//! # bifrost-run — World Run System
//!
//! Implements the **discrete competitive epoch** model from `docs/CORE.md`.
//!
//! ## Core concept
//!
//! The world is not a persistent endless MMO. Instead it runs in **discrete
//! epochs (Runs)**. Each run:
//!
//! - Has a fixed duration OR win condition.
//! - Pits human player factions against the Synthesis AI civilisation.
//! - Ends when an [`EndCondition`] is reached.
//! - Triggers a new world generation via the WAC pipeline.
//! - Feeds results into the [`MetaProgression`] system.
//!
//! ## Game loop result
//!
//! ```text
//! Run N ends  →  WorldRunDirector  →  WAC Seed Generator
//!                                           │
//!                                    New Biome + Loot synthesis
//!                                           │
//!                                    Run N+1 begins
//! ```
//!
//! ## Meta persistence
//!
//! Two layers of progression:
//!
//! | Layer | Duration | Examples |
//! |---|---|---|
//! | Run progression | Resets per run | skills, gear, territory |
//! | Meta progression | Persistent | unlocks, archetypes, starting perks |
//!
//! ## Key types
//!
//! - [`WorldRun`] — a single competitive epoch
//! - [`RunState`] — lifecycle state machine
//! - [`EndCondition`] — win condition variants
//! - [`RunResult`] — winner / loser outcome with effects
//! - [`MetaProgression`] — persistent cross-run player state
//! - [`WorldRunDirector`] — orchestrates run start / end / world mutation

pub mod director;
pub mod end_condition;
pub mod meta;
pub mod run;

pub use director::WorldRunDirector;
pub use end_condition::{EndCondition, EndConditionEval};
pub use meta::{MetaProgression, MetaUnlock, RunReward, SkillDecay};
pub use run::{WorldRun, RunState, RunResult, FactionId, RunId};
