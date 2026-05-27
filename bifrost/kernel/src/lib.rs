//! # bifrost-kernel — Forge Kernel
//!
//! Unified kernel that enforces the five BIFROST architecture rules **and**
//! provides the NOVA voxel world engine.  Everything is born here.
//!
//! ## BIFROST Architecture Rules
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
//!
//! ## Voxel World Engine
//!
//! | Module | What it provides |
//! |---|---|
//! | [`core`] | [`VoxelChunk`], [`Voxel`], materials, mesh, navmesh, palette |
//! | [`generator`] | biome system, noise primitives, terrain generator |
//! | [`runtime`] | [`WorldRuntime`] chunk cache, [`ChunkStreamer`] queue |
//! | [`bridge`] | [`RuntimeAdapter`] — WAC JSON → [`VoxelChunk`] pipeline |
//!
//! ## Quick start
//!
//! ```rust,ignore
//! use bifrost_kernel::bridge::RuntimeAdapter;
//!
//! let mut rt = RuntimeAdapter::new();
//! let chunk = rt.apply(r#"{
//!   "type": "biome_chunk",
//!   "pos": {"x": 0, "y": 0, "z": 0},
//!   "name": "crimson_forest",
//!   "seed": 1337,
//!   "rules": {"terrain": "dense", "density": 0.82}
//! }"#).unwrap();
//! ```

// ── BIFROST Architecture Rules (R1-R5) ───────────────────────────────────────
pub mod clock;
pub mod ids;
pub mod ledger;
pub mod pipeline;
pub mod transition;

pub use clock::SequencedInstant;
pub use ids::{FactionId, ZoneId};
pub use ledger::Ledger;
pub use pipeline::{EventPipeline, PipelineError, RawEvent};
pub use transition::{ApplyTransition, StateTransitionFn};

// ── Voxel World Engine ────────────────────────────────────────────────────────
pub mod bridge;
pub mod core;
pub mod generator;
pub mod runtime;

pub use bridge::{RuntimeAdapter, WacError, WacResult};
pub use core::{ChunkPos, VoxelChunk, CHUNK_SIZE};
pub use runtime::WorldRuntime;
