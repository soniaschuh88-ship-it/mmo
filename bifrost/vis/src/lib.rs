//! # bifrost-vis — Voxel Instruction Set
//!
//! The VIS is the deterministic opcode layer of the Bifrost architecture.
//! Instead of transmitting individual voxel mutations, the network transmits
//! **batch instructions** that every peer executes locally:
//!
//! ```text
//! SIM_EXPLOSION(center=(10,0,10), radius=12, force=1000)
//! ```
//!
//! This reduces network cost from O(radius³) individual events to O(1)
//! per instruction, while preserving full determinism via BLAKE3 hashing.
//!
//! ## Key types
//!
//! - [`VoxelOpcode`] — the 9-opcode instruction table
//! - [`InstructionPayload`] — typed parameters per opcode
//! - [`VoxelInstruction`] — opcode + epoch + payload + BLAKE3 hash
//! - [`VoxelProgram`] — ordered sequence with BLAKE3 root hash

pub mod coord;
pub mod instruction;
pub mod opcode;
pub mod payload;
pub mod program;

pub use coord::VoxelCoord;
pub use instruction::{InstructionError, VoxelInstruction};
pub use opcode::VoxelOpcode;
pub use payload::{
    DamageFieldPayload, FillBoxPayload, InstructionPayload, MarchMaterialPayload,
    SetVoxelPayload, SimDebrisPayload, SimExplosionPayload, SimFirePayload,
    SimWaterPayload, SphereCutPayload,
};
pub use program::{ProgramError, VoxelProgram};
