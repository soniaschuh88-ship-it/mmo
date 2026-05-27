//! WorldRuntime — in-memory world state: chunk cache + voxel queries.

use std::collections::BTreeMap;

use crate::core::{ChunkPos, MaterialPalette, VoxelChunk, Voxel, CHUNK_SIZE};
use crate::generator::biome::BiomeRegistry;
use crate::generator::terrain::generate_chunk;

/// The active world — a cache of generated chunks and the tools to make more.
pub struct WorldRuntime {
    /// Loaded chunks keyed by position.
    chunks:   BTreeMap<ChunkPos, VoxelChunk>,
    /// Biome registry shared across all chunks.
    pub biomes:   BiomeRegistry,
    /// Material palette shared across all chunks.
    pub palette:  MaterialPalette,
}

impl WorldRuntime {
    /// Create an empty world with all built-in biomes and materials registered.
    pub fn new() -> Self {
        Self {
            chunks:  BTreeMap::new(),
            biomes:  BiomeRegistry::with_builtins(),
            palette: MaterialPalette::builtin(),
        }
    }

    // ── Chunk access ──────────────────────────────────────────────────────────

    /// Get a chunk if it is loaded.
    pub fn get_chunk(&self, pos: ChunkPos) -> Option<&VoxelChunk> {
        self.chunks.get(&pos)
    }

    /// Get or generate a chunk for the given biome.
    ///
    /// If the chunk is already loaded, returns the cached version.
    /// Otherwise generates it deterministically and caches it.
    pub fn get_or_generate(&mut self, pos: ChunkPos, biome_name: &str) -> &VoxelChunk {
        if !self.chunks.contains_key(&pos) {
            let biome = self.biomes.get_or_default(biome_name).clone();
            let chunk = generate_chunk(pos, &biome);
            self.chunks.insert(pos, chunk);
        }
        self.chunks.get(&pos).unwrap()
    }

    /// Insert a pre-built chunk (e.g. from the WAC adapter).
    pub fn insert_chunk(&mut self, chunk: VoxelChunk) {
        self.chunks.insert(chunk.position, chunk);
    }

    /// Remove a chunk from the cache (eviction).
    pub fn evict(&mut self, pos: &ChunkPos) -> Option<VoxelChunk> {
        self.chunks.remove(pos)
    }

    /// Evict all chunks farther than `radius` chunks from `center`.
    pub fn evict_distant(&mut self, center: ChunkPos, radius: i32) {
        let r = radius as i64;
        self.chunks.retain(|pos, _| {
            let dx = (pos.x as i64 - center.x as i64).abs();
            let dy = (pos.y as i64 - center.y as i64).abs();
            let dz = (pos.z as i64 - center.z as i64).abs();
            dx <= r && dy <= r && dz <= r
        });
    }

    // ── World-space voxel access ───────────────────────────────────────────────

    /// Convert world voxel coordinates to (chunk_pos, local_x, local_y, local_z).
    fn world_to_local(wx: i64, wy: i64, wz: i64) -> (ChunkPos, usize, usize, usize) {
        let s = CHUNK_SIZE as i64;
        let cx = wx.div_euclid(s) as i32;
        let cy = wy.div_euclid(s) as i32;
        let cz = wz.div_euclid(s) as i32;
        let lx = wx.rem_euclid(s) as usize;
        let ly = wy.rem_euclid(s) as usize;
        let lz = wz.rem_euclid(s) as usize;
        (ChunkPos::new(cx, cy, cz), lx, ly, lz)
    }

    /// Get the voxel at world coordinates.
    /// Returns `Voxel::AIR` if the chunk is not loaded.
    pub fn get_voxel(&self, wx: i64, wy: i64, wz: i64) -> Voxel {
        let (pos, lx, ly, lz) = Self::world_to_local(wx, wy, wz);
        self.chunks
            .get(&pos)
            .map(|c| c.get(lx, ly, lz))
            .unwrap_or(Voxel::AIR)
    }

    /// Set a voxel at world coordinates.
    /// No-op if the chunk is not loaded.
    pub fn set_voxel(&mut self, wx: i64, wy: i64, wz: i64, v: Voxel) {
        let (pos, lx, ly, lz) = Self::world_to_local(wx, wy, wz);
        if let Some(chunk) = self.chunks.get_mut(&pos) {
            chunk.set(lx, ly, lz, v);
            // Mark chunk as needing rehash (caller should call rehash_chunk)
        }
    }

    /// Rehash a specific chunk after mutations.
    pub fn rehash_chunk(&mut self, pos: &ChunkPos) {
        if let Some(c) = self.chunks.get_mut(pos) { c.rehash(); }
    }

    // ── Statistics ────────────────────────────────────────────────────────────

    pub fn chunk_count(&self) -> usize { self.chunks.len() }

    pub fn total_voxels(&self) -> u64 {
        self.chunks.values().map(|c| c.fill_count() as u64).sum()
    }

    /// All loaded chunk positions in sorted order.
    pub fn loaded_positions(&self) -> impl Iterator<Item = &ChunkPos> {
        self.chunks.keys()
    }
}

impl Default for WorldRuntime {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_and_retrieve() {
        let mut w = WorldRuntime::new();
        let pos = ChunkPos::new(0, 0, 0);
        w.get_or_generate(pos, "plains");
        assert_eq!(w.chunk_count(), 1);
        assert!(w.get_chunk(pos).is_some());
    }

    #[test]
    fn cached_chunk_same_hash() {
        let mut w = WorldRuntime::new();
        let pos = ChunkPos::new(2, 0, -1);
        let h1 = w.get_or_generate(pos, "forest").state_hash;
        let h2 = w.get_or_generate(pos, "forest").state_hash; // from cache
        assert_eq!(h1, h2);
    }

    #[test]
    fn world_to_local_conversion() {
        let s = CHUNK_SIZE as i64;
        let (pos, lx, ly, lz) = WorldRuntime::world_to_local(s + 3, s * 2 + 7, 5);
        assert_eq!(pos, ChunkPos::new(1, 2, 0));
        assert_eq!(lx, 3);
        assert_eq!(ly, 7);
        assert_eq!(lz, 5);
    }

    #[test]
    fn negative_world_coords() {
        let s = CHUNK_SIZE as i64;
        let (pos, lx, _, _) = WorldRuntime::world_to_local(-1, 0, 0);
        assert_eq!(pos.x, -1, "x should be chunk -1");
        assert_eq!(lx as i64, s - 1, "local x should be CHUNK_SIZE-1");
    }

    #[test]
    fn get_set_voxel_worldspace() {
        let mut w = WorldRuntime::new();
        w.get_or_generate(ChunkPos::default(), "plains");
        let v = crate::core::Voxel::solid(crate::core::materials::STONE);
        w.set_voxel(5, 10, 15, v);
        assert_eq!(w.get_voxel(5, 10, 15).material, crate::core::materials::STONE);
    }

    #[test]
    fn evict_distant_chunks() {
        let mut w = WorldRuntime::new();
        for x in -3..=3 {
            for z in -3..=3 {
                let p = ChunkPos::new(x, 0, z);
                w.get_or_generate(p, "plains");
            }
        }
        assert_eq!(w.chunk_count(), 7*7);
        w.evict_distant(ChunkPos::default(), 1);
        // Should keep (-1..=1) × (-1..=1) = 9 chunks
        assert_eq!(w.chunk_count(), 3*3);
    }
}
