//! # nova-anim
//!
//! NOVA Engine animation system.
//!
//! | Module | What it provides |
//! |---|---|
//! | [`skeleton`] | [`VoxelSkeleton`] — named bone groups for voxel character models |
//! | [`clip`] | [`AnimClip`], [`Track`], [`Keyframe`] — keyframe animation data |
//! | [`state_machine`] | [`AnimStateMachine`] — FSM: idle·walk·attack·hurt·die |
//!
//! ## WAC bridge (feature `wac`)
//!
//! Enable the `wac` feature to unlock [`wac_bridge::build_fsm`], which
//! converts a [`bifrost_wac::types::AnimationGraphIR`] produced by the WAC
//! compiler into a live [`AnimStateMachine`].
//!
//! ```toml
//! nova-anim = { workspace = true, features = ["wac"] }
//! ```
//!
//! ## Integration with bifrost-run
//!
//! [`AnimStateMachine`] is driven by the same [`RunState`] transitions that
//! govern the bifrost-run world epoch.  When a [`WorldRun`] begins, all
//! entities receive a fresh FSM seeded with the run's `world_seed`.
//!
//! ## Quick start
//!
//! ```rust,ignore
//! use nova_anim::{VoxelSkeleton, standard_character_fsm};
//!
//! let mut fsm = standard_character_fsm(VoxelSkeleton::humanoid());
//!
//! // Each frame:
//! fsm.set_moving(true);
//! fsm.update(delta_time);
//! ```

pub mod clip;
pub mod skeleton;
pub mod state_machine;

/// WAC → FSM bridge — converts [`bifrost_wac::types::AnimationGraphIR`] to a
/// live [`AnimStateMachine`].  Only compiled when the `wac` feature is enabled.
///
/// R1: bifrost-wac has **no** nova dependency; this module is the one-way
/// bridge that transforms WAC IR into nova-anim runtime objects.
#[cfg(feature = "wac")]
pub mod wac_bridge;

pub use clip::{AnimClip, Keyframe, Track};
pub use skeleton::{BoneGroup, BonePose, VoxelSkeleton};
pub use state_machine::{
    AnimCondition, AnimParams, AnimState, AnimStateMachine,
    Transition, standard_character_fsm,
};
