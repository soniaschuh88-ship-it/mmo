//! InstructionPayload — per-opcode parameters.
//!
//! Each variant carries exactly the parameters needed for that opcode.
//! The `canonical_bytes()` method produces a deterministic byte sequence
//! used as input to BLAKE3 instruction hashing.

use serde::{Deserialize, Serialize};

use crate::coord::VoxelCoord;
use crate::opcode::VoxelOpcode;

// ─── Per-opcode parameter structs ─────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetVoxelPayload {
    pub position: VoxelCoord,
    /// Material ID (0 = air, 1 = stone, 2 = dirt, …)
    pub material: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FillBoxPayload {
    pub min: VoxelCoord,
    pub max: VoxelCoord,
    pub material: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SphereCutPayload {
    pub center: VoxelCoord,
    pub radius: u32,
    pub material: u8,
}

/// Direction encoded as three signed bytes, each -1, 0, or 1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarchMaterialPayload {
    pub origin: VoxelCoord,
    pub direction: [i8; 3],
    pub steps: u32,
    pub material: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DamageFieldPayload {
    pub center: VoxelCoord,
    pub radius: u32,
    /// Damage applied to voxel durability (0–65535).
    pub damage: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimWaterPayload {
    pub origin: VoxelCoord,
    /// Volume in voxel-units³ of water to inject.
    pub volume: u32,
    /// Pressure level (affects flow speed).
    pub pressure: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimFirePayload {
    pub origin: VoxelCoord,
    /// Initial intensity (0–65535).
    pub intensity: u16,
    /// Fuel budget in voxel-units (fire consumes fuel per tick).
    pub fuel: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimDebrisPayload {
    pub origin: VoxelCoord,
    /// Number of debris particles.
    pub count: u32,
    /// Impulse magnitude (determines initial velocity).
    pub impulse: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimExplosionPayload {
    pub center: VoxelCoord,
    /// Blast radius in voxels.
    pub radius: u32,
    /// Physical force applied to surrounding entities.
    pub force: u32,
    /// Material left after excavation (0 = air).
    pub result_material: u8,
}

// ─── Unified payload enum ──────────────────────────────────────────────────────

/// The payload for a `VoxelInstruction`, parameterized by opcode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum InstructionPayload {
    SetVoxel(SetVoxelPayload),
    FillBox(FillBoxPayload),
    SphereCut(SphereCutPayload),
    MarchMaterial(MarchMaterialPayload),
    DamageField(DamageFieldPayload),
    SimWater(SimWaterPayload),
    SimFire(SimFirePayload),
    SimDebris(SimDebrisPayload),
    SimExplosion(SimExplosionPayload),
}

impl InstructionPayload {
    /// The opcode corresponding to this payload variant.
    pub fn opcode(&self) -> VoxelOpcode {
        match self {
            Self::SetVoxel(_)      => VoxelOpcode::SetVoxel,
            Self::FillBox(_)       => VoxelOpcode::FillBox,
            Self::SphereCut(_)     => VoxelOpcode::SphereCut,
            Self::MarchMaterial(_) => VoxelOpcode::MarchMaterial,
            Self::DamageField(_)   => VoxelOpcode::DamageField,
            Self::SimWater(_)      => VoxelOpcode::SimWater,
            Self::SimFire(_)       => VoxelOpcode::SimFire,
            Self::SimDebris(_)     => VoxelOpcode::SimDebris,
            Self::SimExplosion(_)  => VoxelOpcode::SimExplosion,
        }
    }

    /// Canonical byte representation for BLAKE3 hashing.
    ///
    /// Field order is fixed by this implementation — this is a stability
    /// guarantee. Changing this function breaks all existing instruction hashes.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(32);
        match self {
            Self::SetVoxel(p) => {
                buf.extend_from_slice(&p.position.to_bytes());
                buf.push(p.material);
            }
            Self::FillBox(p) => {
                buf.extend_from_slice(&p.min.to_bytes());
                buf.extend_from_slice(&p.max.to_bytes());
                buf.push(p.material);
            }
            Self::SphereCut(p) => {
                buf.extend_from_slice(&p.center.to_bytes());
                buf.extend_from_slice(&p.radius.to_le_bytes());
                buf.push(p.material);
            }
            Self::MarchMaterial(p) => {
                buf.extend_from_slice(&p.origin.to_bytes());
                buf.push(p.direction[0] as u8);
                buf.push(p.direction[1] as u8);
                buf.push(p.direction[2] as u8);
                buf.extend_from_slice(&p.steps.to_le_bytes());
                buf.push(p.material);
            }
            Self::DamageField(p) => {
                buf.extend_from_slice(&p.center.to_bytes());
                buf.extend_from_slice(&p.radius.to_le_bytes());
                buf.extend_from_slice(&p.damage.to_le_bytes());
            }
            Self::SimWater(p) => {
                buf.extend_from_slice(&p.origin.to_bytes());
                buf.extend_from_slice(&p.volume.to_le_bytes());
                buf.extend_from_slice(&p.pressure.to_le_bytes());
            }
            Self::SimFire(p) => {
                buf.extend_from_slice(&p.origin.to_bytes());
                buf.extend_from_slice(&p.intensity.to_le_bytes());
                buf.extend_from_slice(&p.fuel.to_le_bytes());
            }
            Self::SimDebris(p) => {
                buf.extend_from_slice(&p.origin.to_bytes());
                buf.extend_from_slice(&p.count.to_le_bytes());
                buf.extend_from_slice(&p.impulse.to_le_bytes());
            }
            Self::SimExplosion(p) => {
                buf.extend_from_slice(&p.center.to_bytes());
                buf.extend_from_slice(&p.radius.to_le_bytes());
                buf.extend_from_slice(&p.force.to_le_bytes());
                buf.push(p.result_material);
            }
        }
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opcode_matches_variant() {
        let p = InstructionPayload::SimExplosion(SimExplosionPayload {
            center: VoxelCoord::new(0, 0, 0),
            radius: 12,
            force: 1000,
            result_material: 0,
        });
        assert_eq!(p.opcode(), VoxelOpcode::SimExplosion);
    }

    #[test]
    fn canonical_bytes_deterministic() {
        let p = InstructionPayload::SetVoxel(SetVoxelPayload {
            position: VoxelCoord::new(5, 9, 1),
            material: 3,
        });
        let b1 = p.canonical_bytes();
        let b2 = p.canonical_bytes();
        assert_eq!(b1, b2);
        // 12 bytes coord + 1 byte material
        assert_eq!(b1.len(), 13);
    }

    #[test]
    fn explosion_canonical_bytes_length() {
        let p = InstructionPayload::SimExplosion(SimExplosionPayload {
            center: VoxelCoord::default(),
            radius: 12,
            force: 500,
            result_material: 0,
        });
        // 12 (coord) + 4 (radius) + 4 (force) + 1 (material) = 21
        assert_eq!(p.canonical_bytes().len(), 21);
    }
}
