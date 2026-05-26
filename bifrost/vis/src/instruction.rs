//! VoxelInstruction — a single deterministic voxel operation.
//!
//! An instruction is the atomic unit of the VIS protocol. It carries:
//! - An opcode identifying the operation
//! - The epoch in which it was issued (for replay ordering)
//! - A typed payload with operation parameters
//! - A BLAKE3 hash of (opcode || epoch || payload) for witness verification

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::opcode::VoxelOpcode;
use crate::payload::InstructionPayload;

/// A verified, hashable voxel instruction.
///
/// # Hash Stability
///
/// The `hash` field is computed as:
/// ```text
/// BLAKE3(opcode_byte || epoch_le_8 || payload_canonical_bytes)
/// ```
/// This is a **stability guarantee** — existing hashes must remain valid across
/// software upgrades.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoxelInstruction {
    pub opcode:  VoxelOpcode,
    /// Epoch number from the chunk authority system.
    pub epoch:   u64,
    pub payload: InstructionPayload,
    /// BLAKE3 hash of the canonical instruction bytes.
    #[serde(with = "hex_bytes")]
    pub hash:    [u8; 32],
}

#[derive(Debug, Error)]
pub enum InstructionError {
    #[error("opcode mismatch: payload is {payload_op} but opcode field is {opcode_field}")]
    OpcodeMismatch {
        payload_op:   VoxelOpcode,
        opcode_field: VoxelOpcode,
    },
    #[error("hash verification failed: stored={stored} computed={computed}")]
    HashMismatch {
        stored:   String,
        computed: String,
    },
}

impl VoxelInstruction {
    /// Build a new instruction, computing the BLAKE3 hash automatically.
    ///
    /// Returns `Err` if the opcode field does not match the payload variant.
    pub fn new(epoch: u64, payload: InstructionPayload) -> Result<Self, InstructionError> {
        let opcode = payload.opcode();
        let hash = Self::compute_hash(opcode, epoch, &payload);
        Ok(Self { opcode, epoch, payload, hash })
    }

    /// Verify that the stored hash matches the instruction contents.
    pub fn verify(&self) -> Result<(), InstructionError> {
        // Check opcode consistency
        let payload_op = self.payload.opcode();
        if payload_op != self.opcode {
            return Err(InstructionError::OpcodeMismatch {
                payload_op,
                opcode_field: self.opcode,
            });
        }
        // Check hash
        let computed = Self::compute_hash(self.opcode, self.epoch, &self.payload);
        if computed != self.hash {
            return Err(InstructionError::HashMismatch {
                stored:   hex::encode(self.hash),
                computed: hex::encode(computed),
            });
        }
        Ok(())
    }

    /// Compute the canonical BLAKE3 hash for the given instruction parameters.
    ///
    /// Hash input layout (all fields fixed-width or length-prefixed):
    /// ```text
    /// opcode_byte (1)
    /// || epoch_le  (8)
    /// || payload_canonical_bytes (variable)
    /// ```
    pub fn compute_hash(
        opcode:  VoxelOpcode,
        epoch:   u64,
        payload: &InstructionPayload,
    ) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&[opcode.as_byte()]);
        hasher.update(&epoch.to_le_bytes());
        hasher.update(&payload.canonical_bytes());
        *hasher.finalize().as_bytes()
    }
}

// ─── serde helper for [u8; 32] as lowercase hex ───────────────────────────────

mod hex_bytes {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &[u8; 32], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&hex::encode(bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 32], D::Error> {
        let s = String::deserialize(d)?;
        let v = hex::decode(&s).map_err(serde::de::Error::custom)?;
        v.try_into()
            .map_err(|_| serde::de::Error::custom("expected 32-byte hex string"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coord::VoxelCoord;
    use crate::payload::{SimExplosionPayload, SetVoxelPayload};

    #[test]
    fn new_and_verify() {
        let payload = InstructionPayload::SetVoxel(SetVoxelPayload {
            position: VoxelCoord::new(10, 20, 30),
            material: 1,
        });
        let instr = VoxelInstruction::new(42, payload).unwrap();
        assert_eq!(instr.opcode, VoxelOpcode::SetVoxel);
        assert!(instr.verify().is_ok());
    }

    #[test]
    fn hash_determinism() {
        let payload = InstructionPayload::SimExplosion(SimExplosionPayload {
            center: VoxelCoord::new(0, 64, 0),
            radius: 12,
            force:  1000,
            result_material: 0,
        });
        let h1 = VoxelInstruction::compute_hash(VoxelOpcode::SimExplosion, 7, &payload);
        let h2 = VoxelInstruction::compute_hash(VoxelOpcode::SimExplosion, 7, &payload);
        assert_eq!(h1, h2);
    }

    #[test]
    fn different_epoch_different_hash() {
        let payload = InstructionPayload::SetVoxel(SetVoxelPayload {
            position: VoxelCoord::default(),
            material: 2,
        });
        let h1 = VoxelInstruction::compute_hash(VoxelOpcode::SetVoxel, 1, &payload);
        let h2 = VoxelInstruction::compute_hash(VoxelOpcode::SetVoxel, 2, &payload);
        assert_ne!(h1, h2);
    }

    #[test]
    fn tampered_hash_detected() {
        let payload = InstructionPayload::SetVoxel(SetVoxelPayload {
            position: VoxelCoord::new(1, 2, 3),
            material: 5,
        });
        let mut instr = VoxelInstruction::new(1, payload).unwrap();
        instr.hash[0] ^= 0xFF; // corrupt the hash
        assert!(instr.verify().is_err());
    }
}
