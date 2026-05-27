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
//!
//! // Read current bone poses for the renderer:
//! let head_pose = fsm.skeleton.current_pose("head");
//! ```

pub mod clip;
pub mod skeleton;
pub mod state_machine;

pub use clip::{AnimClip, Keyframe, Track};
pub use skeleton::{BoneGroup, BonePose, VoxelSkeleton};
pub use state_machine::{
    AnimCondition, AnimParams, AnimState, AnimStateMachine,
    Transition, standard_character_fsm,
};
