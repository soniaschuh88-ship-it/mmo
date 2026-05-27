//! VoxelSkeleton — named bone groups for voxel character models.
//!
//! Rather than per-vertex weights, a [`VoxelSkeleton`] defines named **groups**
//! of voxel pixels that transform as a rigid body.  Each group maps to a
//! rectangular region of the voxel grid:
//!
//! ```text
//! 8×12 voxel character (front view):
//!
//!   y 11 ┌──────────┐  ← head      (y 9-11, x 2-5)
//!        │          │
//!    9   └──────────┘
//!    8   ┌──────────┐  ← upper_body (y 5-8, x 1-6)
//!        │  torso   │    arm_l (x 0-1), arm_r (x 6-7)
//!    5   └──────────┘
//!    4   ┌──────────┐  ← leg_l (x 2-3), leg_r (x 4-5)
//!        │  legs    │
//!    0   └──────────┘
//! ```
//!
//! This layout is shared with `game.html`'s sprite renderer and
//! `nova-render`'s voxel character pass.

use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use nova_core::transform::{Quat, Vec3};

// ─── BoneGroup ────────────────────────────────────────────────────────────────

/// One named group of voxels that transform together.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoneGroup {
    pub name:    String,
    /// Y voxel rows controlled by this bone (inclusive).
    pub y_range: (u8, u8),
    /// X voxel cols controlled by this bone (inclusive).
    pub x_range: (u8, u8),
    /// Local rotation pivot in voxel-space (col, row, depth).
    pub pivot:   Vec3,
    /// Parent bone name — transforms are concatenated up the chain.
    pub parent:  Option<String>,
}

impl BoneGroup {
    pub fn new(name: impl Into<String>, y: (u8,u8), x: (u8,u8), pivot: Vec3) -> Self {
        Self { name: name.into(), y_range: y, x_range: x, pivot, parent: None }
    }

    pub fn with_parent(mut self, p: impl Into<String>) -> Self {
        self.parent = Some(p.into()); self
    }
}

// ─── BonePose ─────────────────────────────────────────────────────────────────

/// Current pose offset for one bone group (local-space, relative to bind pose).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BonePose {
    pub translation: Vec3,
    pub rotation:    Quat,
    pub scale:       Vec3,
}

impl BonePose {
    pub fn identity() -> Self {
        Self { translation: Vec3::ZERO, rotation: Quat::IDENTITY, scale: Vec3::ONE }
    }

    /// Linear blend with another pose.
    pub fn lerp(&self, other: &BonePose, t: f32) -> BonePose {
        BonePose {
            translation: self.translation.lerp(other.translation, t),
            rotation:    self.rotation.slerp(other.rotation, t),
            scale:       self.scale.lerp(other.scale, t),
        }
    }
}

// ─── VoxelSkeleton ────────────────────────────────────────────────────────────

/// Complete skeleton for a voxel character.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoxelSkeleton {
    pub bones: Vec<BoneGroup>,
    /// Live pose per bone name.  Absent → identity.
    pub pose:  BTreeMap<String, BonePose>,
}

impl VoxelSkeleton {
    pub fn new(bones: Vec<BoneGroup>) -> Self {
        let pose = bones.iter()
            .map(|b| (b.name.clone(), BonePose::identity()))
            .collect();
        Self { bones, pose }
    }

    /// Standard 8×12 humanoid skeleton shared with `game.html` and `nova-render`.
    pub fn humanoid() -> Self {
        Self::new(vec![
            BoneGroup::new("root",       (0,11), (0,7), Vec3::new(4.0, 0.0, 2.0)),
            BoneGroup::new("head",       (9,11), (2,5), Vec3::new(4.0, 9.0, 2.0))
                .with_parent("root"),
            BoneGroup::new("upper_body", (5, 8), (1,6), Vec3::new(4.0, 6.0, 2.0))
                .with_parent("root"),
            BoneGroup::new("arm_l",      (4, 8), (0,1), Vec3::new(1.0, 7.0, 2.0))
                .with_parent("upper_body"),
            BoneGroup::new("arm_r",      (4, 8), (6,7), Vec3::new(7.0, 7.0, 2.0))
                .with_parent("upper_body"),
            BoneGroup::new("leg_l",      (0, 4), (2,3), Vec3::new(3.0, 4.0, 2.0))
                .with_parent("root"),
            BoneGroup::new("leg_r",      (0, 4), (4,5), Vec3::new(5.0, 4.0, 2.0))
                .with_parent("root"),
        ])
    }

    pub fn set_pose(&mut self, bone: &str, pose: BonePose) {
        self.pose.insert(bone.to_string(), pose);
    }

    pub fn current_pose(&self, bone: &str) -> BonePose {
        self.pose.get(bone).cloned().unwrap_or_else(BonePose::identity)
    }

    /// Reset every bone to its identity / bind pose.
    pub fn reset_to_bind(&mut self) {
        for b in &self.bones {
            self.pose.insert(b.name.clone(), BonePose::identity());
        }
    }

    pub fn bone(&self, name: &str) -> Option<&BoneGroup> {
        self.bones.iter().find(|b| b.name == name)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn humanoid_has_all_standard_bones() {
        let sk = VoxelSkeleton::humanoid();
        for name in &["root","head","upper_body","arm_l","arm_r","leg_l","leg_r"] {
            assert!(sk.bone(name).is_some(), "missing bone: {name}");
        }
    }

    #[test]
    fn set_and_read_pose() {
        let mut sk = VoxelSkeleton::humanoid();
        let p = BonePose { translation: Vec3::new(0.0, 0.3, 0.0), ..BonePose::identity() };
        sk.set_pose("head", p);
        assert!((sk.current_pose("head").translation.y - 0.3).abs() < 1e-6);
    }

    #[test]
    fn missing_bone_returns_identity() {
        let sk = VoxelSkeleton::humanoid();
        assert_eq!(sk.current_pose("nonexistent").translation, Vec3::ZERO);
    }

    #[test]
    fn reset_to_bind_clears_pose() {
        let mut sk = VoxelSkeleton::humanoid();
        sk.set_pose("arm_l", BonePose { translation: Vec3::new(2.0,0.0,0.0), ..BonePose::identity() });
        sk.reset_to_bind();
        assert_eq!(sk.current_pose("arm_l").translation, Vec3::ZERO);
    }

    #[test]
    fn bone_pose_lerp() {
        let a = BonePose::identity();
        let b = BonePose { translation: Vec3::new(0.0, 1.0, 0.0), ..BonePose::identity() };
        let mid = a.lerp(&b, 0.5);
        assert!((mid.translation.y - 0.5).abs() < 1e-5);
    }
}
