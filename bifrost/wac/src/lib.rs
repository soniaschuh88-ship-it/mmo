//! # bifrost-wac — World Asset Compiler
//!
//! Implements the WAC pipeline: LLM / designer intent → validated,
//! deterministic 2-D asset IR → BIFROST runtime.
//!
//! All world assets are **2-D tile-based** — matching the `game.html`
//! canvas renderer.  The old 3-D `VoxelChunkIR` has been replaced by
//! [`TileMapIR`].
//!
//! ## Hard rules (from WAC.md)
//!
//! | LLM / designer MAY | LLM / designer MAY NOT |
//! |---|---|
//! | describe rules | set tiles directly |
//! | define constraints | spawn loot directly |
//! | specify semantics | define animation frames |
//! | provide a seed | mutate world state |
//!
//! ## Pipeline
//!
//! ```text
//! AssetBlueprint (spec + constraints + seed)
//!      │
//!      ▼  [optional: NVIDIA NIM generates spec from faction intent]
//!      │
//!      ▼
//! validate() → WacError on violation
//!      │
//!      ▼
//! compile() → AssetIR  (typed, version-stamped, 2-D)
//!      │
//!      ▼
//! AssetCache (BLAKE3 key = semantic hash of spec + constraints)
//!      │
//!      ▼
//! BIFROST runtime translators
//! ```
//!
//! ## Feature flags
//!
//! | Flag | Description |
//! |---|---|
//! | `nvidia-nim` | Enables [`nvidia`] module — NVIDIA NIM API client for real LLM generation |

pub mod cache;
pub mod compile;
pub mod director;
pub mod pressure;
pub mod types;
pub mod validate;

#[cfg(feature = "nvidia-nim")]
pub mod nvidia;

pub use cache::AssetCache;
pub use compile::compile;
pub use director::WorldDirector;
pub use pressure::PressureGraph;
pub use types::*;
pub use validate::validate;
