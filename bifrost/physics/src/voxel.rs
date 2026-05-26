//! VoxelState and VoxelKey — the fundamental world unit.

use serde::{Deserialize, Serialize};

use crate::material::MAT_AIR;
use crate::vec3::Vec3;

/// World-space key for a voxel position.
///
/// Uses tuple ordering: (x, y, z) in BTreeMap → deterministic iteration.
pub type VoxelKey = (i32, i32, i32);

/// Bitflags for voxel state.
pub mod flags {
    pub const ON_FIRE:   u8 = 0b0000_0001;
    pub const FLOODED:   u8 = 0b0000_0010;
    pub const DAMAGED:   u8 = 0b0000_0100;
    pub const UNSTABLE:  u8 = 0b0000_1000;
}

/// The full state of a single voxel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoxelState {
    /// Material ID (0 = air).
    pub material:   u8,
    /// Remaining durability (0 = destroyed, becomes air on next tick).
    pub durability: u16,
    /// Current velocity (for debris/projectile voxels).
    pub velocity:   Vec3,
    /// Bitflags: `ON_FIRE`, `FLOODED`, etc.
    pub flags:      u8,
}

impl VoxelState {
    /// A solid voxel with full durability and no velocity.
    pub fn solid(material: u8) -> Self {
        let base_durability = crate::material::MaterialProps::for_material(material).durability;
        Self {
            material,
            durability: base_durability,
            velocity:   Vec3::ZERO,
            flags:      0,
        }
    }

    /// Air (empty space).
    pub fn air() -> Self {
        Self { material: MAT_AIR, durability: 0, velocity: Vec3::ZERO, flags: 0 }
    }

    /// True if this is air / empty.
    pub fn is_air(&self) -> bool {
        self.material == MAT_AIR
    }

    /// Apply `damage` to durability. Returns true if the voxel was destroyed.
    pub fn apply_damage(&mut self, damage: u16) -> bool {
        self.durability = self.durability.saturating_sub(damage);
        if self.durability == 0 && self.material != MAT_AIR {
            *self = Self::air();
            true
        } else {
            false
        }
    }

    /// Canonical byte sequence for BLAKE3 hashing.
    ///
    /// Layout: material(1) || durability_le(2) || velocity_bytes(24) || flags(1) = 28 bytes
    pub fn canonical_bytes(&self) -> [u8; 28] {
        let mut buf = [0u8; 28];
        buf[0]       = self.material;
        buf[1..3].copy_from_slice(&self.durability.to_le_bytes());
        buf[3..27].copy_from_slice(&self.velocity.canonical_bytes());
        buf[27]      = self.flags;
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::MAT_STONE;

    #[test]
    fn solid_not_air() {
        let v = VoxelState::solid(MAT_STONE);
        assert!(!v.is_air());
        assert!(v.durability > 0);
    }

    #[test]
    fn damage_destroys() {
        let mut v = VoxelState::solid(MAT_STONE);
        v.durability = 5;
        let destroyed = v.apply_damage(10);
        assert!(destroyed);
        assert!(v.is_air());
    }

    #[test]
    fn damage_partial() {
        let mut v = VoxelState::solid(MAT_STONE);
        v.durability = 100;
        let destroyed = v.apply_damage(30);
        assert!(!destroyed);
        assert_eq!(v.durability, 70);
    }

    #[test]
    fn canonical_bytes_length() {
        let v = VoxelState::solid(MAT_STONE);
        assert_eq!(v.canonical_bytes().len(), 28);
    }
}
