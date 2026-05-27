//! VoxelChunk — a 32×32×32 region of world voxel data.
//!
//! # Determinism guarantee
//!
//! The `state_hash` is `BLAKE3(concat(voxel.to_bytes() for voxel in voxels in XYZ order))`.
//! Identical inputs to the kernel produce identical `state_hash` on every machine.
//!
//! # Memory layout
//!
//! Voxels are stored in `x + y*S + z*S*S` order (x inner loop).
//! This matches typical chunk-traversal access patterns.

use serde::{Deserialize, Serialize};

use crate::core::voxel::Voxel;

/// Side length of a chunk in voxels (32³ = 32 768 voxels per chunk).
pub const CHUNK_SIZE: usize = 32;
/// Total voxels per chunk.
pub const CHUNK_VOLUME: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

/// Chunk-grid position (one unit = one CHUNK_SIZE × CHUNK_SIZE × CHUNK_SIZE region).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub struct ChunkPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl ChunkPos {
    pub fn new(x: i32, y: i32, z: i32) -> Self { Self { x, y, z } }

    /// Convert chunk-grid position to world-voxel origin.
    pub fn world_origin(self) -> (i64, i64, i64) {
        (
            self.x as i64 * CHUNK_SIZE as i64,
            self.y as i64 * CHUNK_SIZE as i64,
            self.z as i64 * CHUNK_SIZE as i64,
        )
    }
}

impl std::fmt::Display for ChunkPos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "chunk({},{},{})", self.x, self.y, self.z)
    }
}

/// Chunk-level metadata — attached to every generated chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMeta {
    /// Biome name that drove generation (e.g. "crimson_forest").
    pub biome:        String,
    /// WAC seed used for generation.
    pub seed:         u64,
    /// True if any voxel in this chunk has a navigation-passable path.
    pub nav_passable: bool,
    /// Approximate surface height within this chunk (for LOD purposes).
    pub surface_y:    u8,
    /// Number of non-air voxels.
    pub fill_count:   u32,
}

impl Default for ChunkMeta {
    fn default() -> Self {
        Self {
            biome:        String::from("void"),
            seed:         0,
            nav_passable: false,
            surface_y:    0,
            fill_count:   0,
        }
    }
}

/// A 32×32×32 deterministic voxel chunk.
///
/// Boxed to avoid stack overflow — 32768 voxels × 4 bytes = 128 KiB.
#[derive(Clone)]
pub struct VoxelChunk {
    pub position:   ChunkPos,
    /// BLAKE3 of all voxel data in XYZ order. Recomputed on every mutation.
    pub state_hash: [u8; 32],
    /// Flat voxel array: index = x + y*S + z*S*S.
    voxels:         Box<[Voxel; CHUNK_VOLUME]>,
    pub meta:       ChunkMeta,
}

impl VoxelChunk {
    /// Allocate a fully-air chunk at the given position.
    pub fn empty(position: ChunkPos) -> Self {
        Self {
            position,
            state_hash: Self::hash_all_air(),
            voxels:     Box::new([Voxel::AIR; CHUNK_VOLUME]),
            meta:       ChunkMeta::default(),
        }
    }

    // ── Voxel access ──────────────────────────────────────────────────────────

    /// Linear index for local position `(x, y, z)`.
    #[inline]
    pub fn idx(x: usize, y: usize, z: usize) -> usize {
        x + y * CHUNK_SIZE + z * CHUNK_SIZE * CHUNK_SIZE
    }

    /// Get voxel at local chunk position.
    ///
    /// # Panics
    /// If any coordinate is >= CHUNK_SIZE.
    #[inline]
    pub fn get(&self, x: usize, y: usize, z: usize) -> Voxel {
        self.voxels[Self::idx(x, y, z)]
    }

    /// Set voxel at local chunk position.
    ///
    /// Does NOT recompute `state_hash` — call `rehash()` when done mutating.
    #[inline]
    pub fn set(&mut self, x: usize, y: usize, z: usize, v: Voxel) {
        self.voxels[Self::idx(x, y, z)] = v;
    }

    /// Try to get a voxel at local position, returning `None` if out of bounds.
    pub fn get_checked(&self, x: i32, y: i32, z: i32) -> Option<Voxel> {
        let s = CHUNK_SIZE as i32;
        if x < 0 || y < 0 || z < 0 || x >= s || y >= s || z >= s {
            return None;
        }
        Some(self.get(x as usize, y as usize, z as usize))
    }

    // ── Hash ──────────────────────────────────────────────────────────────────

    /// Recompute `state_hash` from all voxels. Call after batch mutations.
    pub fn rehash(&mut self) {
        let mut hasher = blake3::Hasher::new();
        for v in self.voxels.iter() {
            hasher.update(&v.to_bytes());
        }
        self.state_hash = *hasher.finalize().as_bytes();

        // Update fill count
        self.meta.fill_count = self.voxels.iter().filter(|v| !v.is_air()).count() as u32;
    }

    fn hash_all_air() -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        for _ in 0..CHUNK_VOLUME {
            hasher.update(&Voxel::AIR.to_bytes());
        }
        *hasher.finalize().as_bytes()
    }

    // ── Bulk operations ───────────────────────────────────────────────────────

    /// Fill the entire chunk with a single voxel type, then rehash.
    pub fn fill(&mut self, v: Voxel) {
        self.voxels.fill(v);
        self.rehash();
    }

    /// Fill a sub-region `[x0..=x1, y0..=y1, z0..=z1]` with `v`.
    /// Coordinates are clamped to `[0, CHUNK_SIZE)`.
    pub fn fill_region(
        &mut self,
        x0: usize, y0: usize, z0: usize,
        x1: usize, y1: usize, z1: usize,
        v: Voxel,
    ) {
        let clamp = |n: usize| n.min(CHUNK_SIZE - 1);
        for z in clamp(z0)..=clamp(z1) {
            for y in clamp(y0)..=clamp(y1) {
                for x in clamp(x0)..=clamp(x1) {
                    self.voxels[Self::idx(x, y, z)] = v;
                }
            }
        }
    }

    /// Iterate over all non-air voxels with their local (x, y, z) positions.
    pub fn non_air_voxels(&self) -> impl Iterator<Item = (usize, usize, usize, Voxel)> + '_ {
        let s = CHUNK_SIZE;
        self.voxels.iter().enumerate().filter_map(move |(idx, &v)| {
            if v.is_air() { return None; }
            let x = idx % s;
            let y = (idx / s) % s;
            let z = idx / (s * s);
            Some((x, y, z, v))
        })
    }

    /// Number of non-air voxels (cached in meta after rehash).
    pub fn fill_count(&self) -> u32 { self.meta.fill_count }
}

impl std::fmt::Debug for VoxelChunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VoxelChunk")
            .field("position",   &self.position)
            .field("state_hash", &hex::encode(&self.state_hash[..4]))
            .field("fill_count", &self.meta.fill_count)
            .field("biome",      &self.meta.biome)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::voxel::Voxel;
    use crate::core::materials;

    #[test]
    fn empty_chunk_all_air() {
        let c = VoxelChunk::empty(ChunkPos::default());
        assert_eq!(c.fill_count(), 0);
        assert!(c.get(0, 0, 0).is_air());
        assert!(c.get(31, 31, 31).is_air());
    }

    #[test]
    fn set_get_roundtrip() {
        let mut c = VoxelChunk::empty(ChunkPos::new(1, 0, -1));
        c.set(5, 10, 15, Voxel::solid(materials::STONE));
        assert_eq!(c.get(5, 10, 15).material, materials::STONE);
        assert!(c.get(5, 10, 14).is_air());
    }

    #[test]
    fn rehash_after_mutation() {
        let mut c = VoxelChunk::empty(ChunkPos::default());
        let h0 = c.state_hash;
        c.set(0, 0, 0, Voxel::solid(materials::STONE));
        c.rehash();
        assert_ne!(c.state_hash, h0);
        assert_eq!(c.fill_count(), 1);
    }

    #[test]
    fn fill_region() {
        let mut c = VoxelChunk::empty(ChunkPos::default());
        c.fill_region(0, 0, 0, 3, 3, 3, Voxel::solid(materials::DIRT));
        c.rehash();
        assert_eq!(c.fill_count(), 4 * 4 * 4); // 4×4×4 = 64
        assert!(c.get(4, 0, 0).is_air());
    }

    #[test]
    fn non_air_iterator() {
        let mut c = VoxelChunk::empty(ChunkPos::default());
        c.set(1, 2, 3, Voxel::solid(materials::GRASS));
        c.set(4, 5, 6, Voxel::solid(materials::WATER));
        let filled: Vec<_> = c.non_air_voxels().collect();
        assert_eq!(filled.len(), 2);
    }

    #[test]
    fn world_origin() {
        let pos = ChunkPos::new(2, -1, 3);
        let (wx, wy, wz) = pos.world_origin();
        let s = CHUNK_SIZE as i64;
        assert_eq!(wx, 2 * s);
        assert_eq!(wy, -1 * s);
        assert_eq!(wz, 3 * s);
    }
}
