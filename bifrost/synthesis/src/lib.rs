//! # bifrost-synthesis — Synthesis AI Civilization
//!
//! Implements the **Synthesis** faction: a distributed strategic AI civilisation
//! that competes against human players using the **same rules** as all other factions.
//!
//! > "Not NPCs. This is a distributed strategic civilisation."
//! >
//! > — `docs/FACTION.md`
//!
//! ## Symmetry guarantee
//!
//! Synthesis agents emit world-manipulation intents in the same format as
//! human players.  Every intent flows through:
//!
//! ```text
//! SynthesisTick → FactionIntent → WAC Blueprint → validate() → compile() → World
//! ```
//!
//! No special access to the world state, no cheating.
//!
//! ## Scale model
//!
//! ```text
//! 1 AgentNode     = Squad / Clan level
//! 1 SubAi         = Region Controller
//! 1 SynthesisCore = Global strategist (backed by NVIDIA NIM)
//! ```
//!
//! ## Tick loop
//!
//! ```text
//! Tick:
//!   1. Sense World  (BIFROST snapshot)
//!   2. Update faction strategy
//!   3. Emit intents (same format as players)
//!   4. Validate via WAC IVL
//!   5. Execute via WAC + world engine
//! ```
//!
//! ## Key types
//!
//! - [`AiFaction`] — top-level faction entity
//! - [`AiMetaFaction`] — cross-run persistent memory + strategy evolution
//! - [`AgentNode`] — individual squad-level agent
//! - [`FactionIntent`] — world manipulation intent emitted each tick
//! - [`SynthesisTick`] — single tick execution producing intents
//! - [`FactionMemory`] — in-run memory graph
//! - [`RunMemoryGraph`] — cross-run strategic memory

pub mod agent;
pub mod faction;
pub mod intent;
pub mod memory;
pub mod strategy;
pub mod tick;

pub use agent::{AgentNode, AgentRole, AgentState};
// FactionId and ZoneId are canonical in bifrost_kernel — import from there.
pub use faction::{AiFaction, AiMetaFaction};
pub use intent::{FactionIntent, IntentType, IntentPriority};
pub use memory::{FactionMemory, FactionMemoryEntry, RunMemoryGraph};
pub use strategy::{WorldModel, StrategyEngine, StrategyGoal};
pub use tick::{SynthesisTick, TickOutput};
