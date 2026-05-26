//! PhysicsWorld — the deterministic voxel world state.
//!
//! # Determinism guarantees
//!
//! - `BTreeMap` iteration is ordered by `VoxelKey = (i32, i32, i32)` — stable across platforms.
//! - Air voxels are not stored (absent key = air) — reduces memory and hash input.
//! - State hash is `BLAKE3(for key in sorted_order: key_bytes || voxel_bytes)`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::voxel::{VoxelKey, VoxelState};

/// The full mutable world state.
///
/// Only non-air voxels are stored. Air is the default for absent keys.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PhysicsWorld {
    /// All non-air voxels. BTreeMap ensures deterministic iteration order.
    voxels: BTreeMap<VoxelKey, VoxelState>,

    /// Current tick number.
    tick: u64,
}

impl PhysicsWorld {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the state of a voxel. Returns `None` for air (absent keys).
    pub fn get(&self, key: VoxelKey) -> Option<&VoxelState> {
        self.voxels.get(&key)
    }

    /// Get the state of a voxel, or air if absent.
    pub fn get_or_air(&self, key: VoxelKey) -> VoxelState {
        self.voxels
            .get(&key)
            .cloned()
            .unwrap_or_else(VoxelState::air)
    }

    /// Set a voxel. Automatically removes air voxels to keep the map sparse.
    pub fn set(&mut self, key: VoxelKey, state: VoxelState) {
        if state.is_air() {
            self.voxels.remove(&key);
        } else {
            self.voxels.insert(key, state);
        }
    }

    /// Remove a voxel (set to air).
    pub fn remove(&mut self, key: &VoxelKey) {
        self.voxels.remove(key);
    }

    /// Advance the world tick counter.
    pub fn advance_tick(&mut self) -> u64 {
        self.tick += 1;
        self.tick
    }

    /// Current tick.
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Number of non-air voxels.
    pub fn voxel_count(&self) -> usize {
        self.voxels.len()
    }

    /// Iterate all non-air voxels in deterministic (sorted) order.
    pub fn iter(&self) -> impl Iterator<Item = (&VoxelKey, &VoxelState)> {
        self.voxels.iter()
    }

    /// Compute the BLAKE3 state hash.
    ///
    /// # Hash layout
    ///
    /// ```text
    /// BLAKE3(
    ///   for (key, voxel) in sorted_voxels:
    ///     key.x_le4 || key.y_le4 || key.z_le4 || voxel.canonical_bytes(28)
    /// )
    /// ```
    ///
    /// An empty world hashes to `BLAKE3(b"")`.
    pub fn state_hash(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        // BTreeMap iterates in sorted key order — deterministic.
        for ((x, y, z), voxel) in &self.voxels {
            hasher.update(&x.to_le_bytes());
            hasher.update(&y.to_le_bytes());
            hasher.update(&z.to_le_bytes());
            hasher.update(&voxel.canonical_bytes());
        }
        *hasher.finalize().as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::{MAT_DIRT, MAT_STONE};
    use crate::voxel::VoxelState;

    #[test]
    fn empty_world() {
        let w = PhysicsWorld::new();
        assert_eq!(w.voxel_count(), 0);
    }

    #[test]
    fn set_get_remove() {
        let mut w = PhysicsWorld::new();
        w.set((0, 0, 0), VoxelState::solid(MAT_STONE));
        assert!(w.get((0, 0, 0)).is_some());
        w.remove(&(0, 0, 0));
        assert!(w.get((0, 0, 0)).is_none());
    }

    #[test]
    fn air_removed_on_set() {
        let mut w = PhysicsWorld::new();
        w.set((1, 2, 3), VoxelState::solid(MAT_DIRT));
        w.set((1, 2, 3), VoxelState::air());
        assert_eq!(w.voxel_count(), 0);
    }

    #[test]
    fn state_hash_deterministic() {
        let mut w = PhysicsWorld::new();
        w.set((0, 0, 0), VoxelState::solid(MAT_STONE));
        w.set((1, 0, 0), VoxelState::solid(MAT_DIRT));
        let h1 = w.state_hash();
        let h2 = w.state_hash();
        assert_eq!(h1, h2);
    }

    #[test]
    fn state_hash_changes_on_mutation() {
        let mut w = PhysicsWorld::new();
        let h_empty = w.state_hash();
        w.set((0, 0, 0), VoxelState::solid(MAT_STONE));
        let h_with_voxel = w.state_hash();
        assert_ne!(h_empty, h_with_voxel);
    }

    #[test]
    fn tick_advance() {
        let mut w = PhysicsWorld::new();
        assert_eq!(w.tick(), 0);
        w.advance_tick();
        assert_eq!(w.tick(), 1);
    }
}
