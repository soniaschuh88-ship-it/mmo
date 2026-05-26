//! VoxelProgram — an ordered sequence of VoxelInstructions with a BLAKE3 root hash.
//!
//! A program represents all voxel mutations for a single chunk-tick. It is:
//! - Deterministically ordered (insertion order; callers must sort by epoch)
//! - Hash-chained: `program_hash = BLAKE3(concat(instruction_hashes))`
//! - Append-only: instructions are pushed, the program is never mutated

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::instruction::{InstructionError, VoxelInstruction};
use crate::payload::InstructionPayload;

#[derive(Debug, Error)]
pub enum ProgramError {
    #[error("instruction error at index {index}: {source}")]
    InstructionError {
        index:  usize,
        #[source]
        source: InstructionError,
    },
    #[error("program hash mismatch: stored={stored} computed={computed}")]
    HashMismatch { stored: String, computed: String },
}

/// An ordered sequence of `VoxelInstruction`s representing one chunk-tick of mutations.
///
/// # Program Hash
///
/// ```text
/// program_hash = BLAKE3(instr[0].hash || instr[1].hash || … || instr[N-1].hash)
/// ```
///
/// An empty program has `program_hash = BLAKE3(b"")`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VoxelProgram {
    pub instructions: Vec<VoxelInstruction>,
    #[serde(with = "hex_bytes")]
    pub program_hash:  [u8; 32],
}

impl VoxelProgram {
    /// Create an empty program.
    pub fn new() -> Self {
        let program_hash = Self::compute_root(&[]);
        Self { instructions: Vec::new(), program_hash }
    }

    /// Append an instruction (built from payload + epoch) and update the root hash.
    pub fn push(&mut self, epoch: u64, payload: InstructionPayload) -> Result<(), InstructionError> {
        let instr = VoxelInstruction::new(epoch, payload)?;
        self.instructions.push(instr);
        self.program_hash = Self::compute_root(&self.instructions);
        Ok(())
    }

    /// Number of instructions in this program.
    pub fn len(&self) -> usize {
        self.instructions.len()
    }

    /// True if no instructions have been added.
    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }

    /// Verify all instructions and the program root hash.
    pub fn verify(&self) -> Result<(), ProgramError> {
        for (i, instr) in self.instructions.iter().enumerate() {
            instr.verify().map_err(|e| ProgramError::InstructionError { index: i, source: e })?;
        }
        let computed = Self::compute_root(&self.instructions);
        if computed != self.program_hash {
            return Err(ProgramError::HashMismatch {
                stored:   hex::encode(self.program_hash),
                computed: hex::encode(computed),
            });
        }
        Ok(())
    }

    /// Compute `BLAKE3(concat(instruction_hashes))`.
    fn compute_root(instructions: &[VoxelInstruction]) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        for instr in instructions {
            hasher.update(&instr.hash);
        }
        *hasher.finalize().as_bytes()
    }
}

// ─── serde helper ─────────────────────────────────────────────────────────────

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
    fn empty_program_valid() {
        let p = VoxelProgram::new();
        assert!(p.is_empty());
        assert!(p.verify().is_ok());
    }

    #[test]
    fn push_and_verify() {
        let mut p = VoxelProgram::new();
        p.push(1, InstructionPayload::SetVoxel(SetVoxelPayload {
            position: VoxelCoord::new(0, 0, 0),
            material: 1,
        })).unwrap();
        p.push(1, InstructionPayload::SimExplosion(SimExplosionPayload {
            center: VoxelCoord::new(10, 0, 10),
            radius: 5,
            force: 200,
            result_material: 0,
        })).unwrap();
        assert_eq!(p.len(), 2);
        assert!(p.verify().is_ok());
    }

    #[test]
    fn program_hash_changes_on_push() {
        let mut p = VoxelProgram::new();
        let h0 = p.program_hash;
        p.push(1, InstructionPayload::SetVoxel(SetVoxelPayload {
            position: VoxelCoord::default(),
            material: 2,
        })).unwrap();
        assert_ne!(p.program_hash, h0);
    }

    #[test]
    fn tampered_instruction_detected() {
        let mut p = VoxelProgram::new();
        p.push(1, InstructionPayload::SetVoxel(SetVoxelPayload {
            position: VoxelCoord::new(1, 2, 3),
            material: 3,
        })).unwrap();
        // Corrupt the first instruction hash
        p.instructions[0].hash[0] ^= 0xFF;
        assert!(p.verify().is_err());
    }

    #[test]
    fn deterministic_root_hash() {
        let build = || {
            let mut p = VoxelProgram::new();
            p.push(5, InstructionPayload::SetVoxel(SetVoxelPayload {
                position: VoxelCoord::new(1, 1, 1),
                material: 4,
            })).unwrap();
            p.program_hash
        };
        assert_eq!(build(), build());
    }
}
