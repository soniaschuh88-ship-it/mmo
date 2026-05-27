//! # bifrost-wac — World Asset Compiler
//!
//! Implements the WAC pipeline: LLM / designer intent → validated,
//! deterministic asset IR → BIFROST runtime.
//!
//! ## Hard rules (from WAC.md)
//!
//! | LLM / designer MAY | LLM / designer MAY NOT |
//! |---|---|
//! | describe rules | set voxels directly |
//! | define constraints | spawn loot directly |
//! | specify semantics | define animation frames |
//! | provide a seed | mutate world state |
//!
//! ## Pipeline
//!
//! ```text
//! AssetBlueprint (spec + constraints + seed)
//!      │
//!      ▼
//! validate() → WacError on violation
//!      │
//!      ▼
//! compile() → AssetIR  (typed, version-stamped)
//!      │
//!      ▼
//! AssetCache (BLAKE3 key = semantic hash of spec + constraints)
//!      │
//!      ▼
//! BIFROST runtime translators
//! ```

pub mod cache;
pub mod compile;
pub mod director;
pub mod pressure;
pub mod types;
pub mod validate;

pub use cache::AssetCache;
pub use compile::compile;
pub use director::WorldDirector;
pub use pressure::PressureGraph;
pub use types::*;
pub use validate::validate;
