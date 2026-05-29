//! # bifrost-physics — Deterministic WASM Physics Kernel
//!
//! A physics kernel designed to produce **identical output on every platform**:
//! browser WASM, desktop native, mobile WASM, and edge nodes.
//!
//! # Determinism guarantees
//!
//! - `BTreeMap` for all world state — no `HashMap` nondeterminism
//! - No `SystemTime` — tick number is the only time reference
//! - No OS-specific behavior — pure computation
//! - Integer squared distances for radius checks — no `sqrt` divergence
//! - IEEE 754 f64 for velocity — deterministic under WASM's float rules
//!
//! # Key types
//!
//! - [`PhysicsWorld`] — sparse voxel world state (BTreeMap-backed)
//! - [`VoxelState`] — material, durability, velocity, flags per voxel
//! - [`PhysicsExecutor`] — stateless executor applying `VoxelProgram`s
//! - [`PhysicsTickResult`] — tick number + BLAKE3 state hash output

pub mod executor;
pub mod material;
pub mod vec3;
pub mod voxel;
pub mod world;

pub use executor::{PhysicsExecutor, PhysicsTickResult};
pub use material::{MaterialProps, MAT_AIR, MAT_DIRT, MAT_GRASS, MAT_IRON,
                   MAT_SAND, MAT_STONE, MAT_WATER, MAT_WOOD};
pub use vec3::PhysicsVec3;
pub use voxel::{flags, VoxelKey, VoxelState};
pub use world::PhysicsWorld;
