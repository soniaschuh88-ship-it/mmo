//! # nexus-voxel-kernel
//!
//! **Headless Voxel Runtime — Deterministic Voxel Execution Engine**
//!
//! Converts AI/LLM world specifications (WAC JSON) into deterministic
//! voxel chunks for the NOVA MMO world.
//!
//! ## Architecture
//!
//! ```text
//! LLM / NPC / World Director
//!         ↓  WAC JSON
//! bridge::RuntimeAdapter::apply()
//!         ↓
//! generator::terrain::generate_chunk()
//!         ↓
//! core::VoxelChunk  →  BLAKE3 state_hash
//!         ↓
//! runtime::WorldRuntime  →  ChunkStreamer  →  BIFROST
//! ```
//!
//! ## Quick start
//!
//! ```rust,ignore
//! use nexus_voxel_kernel::bridge::RuntimeAdapter;
//!
//! let mut rt = RuntimeAdapter::new();
//! let chunk = rt.apply(r#"{
//!   "type": "biome_chunk",
//!   "pos": {"x": 0, "y": 0, "z": 0},
//!   "name": "crimson_forest",
//!   "seed": 1337,
//!   "rules": {
//!     "terrain": "dense",
//!     "material": ["crystal_red", "obsidian"],
//!     "emission": "night_glow",
//!     "density": 0.82
//!   }
//! }"#).unwrap();
//! println!("state_hash: {}", hex::encode(chunk.chunk().unwrap().state_hash));
//! ```

pub mod bridge;
pub mod core;
pub mod generator;
pub mod runtime;

// Re-export the main entry points
pub use bridge::{RuntimeAdapter, WacError, WacResult};
pub use core::{ChunkPos, VoxelChunk, CHUNK_SIZE};
pub use runtime::WorldRuntime;
