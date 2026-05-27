//! # bifrost-kernel — Foundational Architecture Primitives
//!
//! This crate owns every type that enforces the five BIFROST architecture
//! rules (see `RULES.md` in the repository root).
//!
//! ## Rule enforcement map
//!
//! | Rule | Type(s) provided |
//! |---|---|
//! | R1 — one concept, one crate | [`FactionId`], [`ZoneId`] — single canonical definition |
//! | R2 — single mutation path | [`StateTransitionFn`], [`ApplyTransition`] |
//! | R3 — EventPipeline required | [`EventPipeline`], [`PipelineError`] |
//! | R4 — replay-safe | [`Ledger`] append-only log |
//! | R5 — no SystemTime | [`SequencedInstant`] — tick-based logical clock |
//!
//! No other crate may define `FactionId`, `ZoneId`, or re-implement these
//! primitives.  All crates that need them import from here.

pub mod clock;
pub mod ids;
pub mod ledger;
pub mod pipeline;
pub mod transition;

pub use clock::SequencedInstant;
pub use ids::{FactionId, ZoneId};
pub use ledger::Ledger;
pub use pipeline::{EventPipeline, PipelineError};
pub use transition::{ApplyTransition, StateTransitionFn};
