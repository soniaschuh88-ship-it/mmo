//! VoxelOpcode — the full Voxel Instruction Set opcode table.

use serde::{Deserialize, Serialize};

/// Every operation the distributed physics fabric can execute.
///
/// Opcodes are designed to be **batch instructions**, not individual voxel
/// mutations. A single `SimExplosion` replaces O(radius³) individual `SetVoxel`
/// events — every peer executes the same deterministic local expansion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum VoxelOpcode {
    /// Set a single voxel to a material.
    SetVoxel      = 0x01,
    /// Fill an axis-aligned bounding box with a material.
    FillBox       = 0x02,
    /// Cut a sphere out of the world, replacing with a material.
    SphereCut     = 0x03,
    /// March along a direction, painting material at each step.
    MarchMaterial = 0x04,
    /// Apply a damage field (reduces voxel durability) within a radius.
    DamageField   = 0x05,
    /// Simulate a water flow tick from an origin.
    SimWater      = 0x06,
    /// Simulate fire propagation from an origin.
    SimFire       = 0x07,
    /// Scatter debris particles from an origin.
    SimDebris     = 0x08,
    /// Explosion: sphere-cut + damage-field + debris scatter.
    SimExplosion  = 0x09,
}

impl VoxelOpcode {
    /// Raw byte discriminant used in instruction hashing.
    #[inline]
    pub fn as_byte(self) -> u8 {
        self as u8
    }

    /// Human-readable name for display and debugging.
    pub fn name(self) -> &'static str {
        match self {
            Self::SetVoxel      => "SET_VOXEL",
            Self::FillBox       => "FILL_BOX",
            Self::SphereCut     => "SPHERE_CUT",
            Self::MarchMaterial => "MARCH_MATERIAL",
            Self::DamageField   => "DAMAGE_FIELD",
            Self::SimWater      => "SIM_WATER",
            Self::SimFire       => "SIM_FIRE",
            Self::SimDebris     => "SIM_DEBRIS",
            Self::SimExplosion  => "SIM_EXPLOSION",
        }
    }
}

impl std::fmt::Display for VoxelOpcode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discriminant_stability() {
        // These values are part of the wire format — never change them.
        assert_eq!(VoxelOpcode::SetVoxel     as u8, 0x01);
        assert_eq!(VoxelOpcode::FillBox      as u8, 0x02);
        assert_eq!(VoxelOpcode::SphereCut    as u8, 0x03);
        assert_eq!(VoxelOpcode::SimExplosion as u8, 0x09);
    }

    #[test]
    fn display() {
        assert_eq!(VoxelOpcode::SimExplosion.to_string(), "SIM_EXPLOSION");
        assert_eq!(VoxelOpcode::SetVoxel.to_string(), "SET_VOXEL");
    }
}
