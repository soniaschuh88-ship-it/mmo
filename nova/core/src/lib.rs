//! # nova-core
//!
//! NOVA Engine core systems:
//!
//! | Module | What it provides |
//! |---|---|
//! | [`ecs`] | Sparse-set [`World`], [`EntityId`], [`Component`] |
//! | [`transform`] | [`Vec3`], [`Quat`], [`Mat4`], [`Transform3D`] |
//! | [`scene`] | [`SceneGraph`] — parent/child hierarchy, world-matrix |
//! | [`time`] | [`Time`] delta, fixed-step budget · [`Timer`] |
//!
//! ## Quick start
//!
//! ```rust,ignore
//! use nova_core::{World, Transform3D, Vec3, Name};
//!
//! let mut world = World::new();
//! let player = world.spawn();
//! world.insert(player, Transform3D::at(Vec3::new(10.0, 0.0, 10.0)));
//! world.insert(player, Name::new("Player"));
//!
//! for (id, t) in world.query::<Transform3D>() {
//!     println!("{id}  pos={:?}", t.position);
//! }
//! ```
//!
//! ## Design notes
//!
//! * **No macros required** — components are any `'static + Send + Sync` type.
//! * **Deterministic ordering** — [`BTreeMap`] storage guarantees consistent
//!   iteration order across platforms (important for lockstep networking).
//! * **GPU-ready maths** — `f32` throughout; [`Mat4::as_f32_array`] gives a
//!   column-major `[f32; 16]` ready for a `wgpu` uniform buffer.

pub mod ecs;
pub mod scene;
pub mod time;
pub mod transform;

pub use ecs::{Component, Disabled, EntityId, Name, Tags, World};
pub use scene::SceneGraph;
pub use time::{Time, Timer};
pub use transform::{Mat4, Quat, Transform3D, Vec3};
