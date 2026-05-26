//! # bifrost-chunk — Chunk Authority Epochs
//!
//! Spatial authority partitioning for the Bifrost Layer.
//!
//! The world is divided into 64×64×64 voxel chunks. Each chunk has a single
//! **authority peer** responsible for aggregating `VoxelInstruction`s and
//! producing the reference tick state hash. Two **witness peers** independently
//! verify that hash.
//!
//! Authority rotates every `epoch_duration_ticks` using deterministic
//! round-robin over the peer pool — no single peer controls a chunk indefinitely.
//!
//! ## Key types
//!
//! - [`ChunkCoord`] / [`ChunkId`] — spatial chunk identity
//! - [`PeerId`] — 32-byte peer public key identity
//! - [`ChunkAuthority`] — current authority + witnesses for an epoch
//! - [`EpochBoundary`] — signed checkpoint at epoch rotation
//! - [`ChunkRegistry`] — manages all chunk assignments + rotation

pub mod authority;
pub mod coord;
pub mod epoch;
pub mod peer;
pub mod registry;

pub use authority::ChunkAuthority;
pub use coord::{ChunkCoord, ChunkId};
pub use epoch::EpochBoundary;
pub use peer::PeerId;
pub use registry::{ChunkRegistry, RegistryError};
