//! Terrain generator — converts biome rules + chunk position into VoxelChunk.
//!
//! Pipeline per chunk:
//! 1. Generate 32×32 heightmap from fBm noise + terrain style
//! 2. For each column (x, z): fill voxels from 0 to height
//! 3. Assign materials per layer from the biome's VoxelRuleSet
//! 4. Apply emission map
//! 5. Rehash the chunk
//!
//! All operations are deterministic from (chunk_pos, seed, biome).

use crate::core::{ChunkMeta, ChunkPos, VoxelChunk, CHUNK_SIZE, Voxel};
use crate::core::materials;
use crate::generator::biome::{Biome, EmissionMode, TerrainStyle};
use crate::generator::noise;

// ── HeightMap ─────────────────────────────────────────────────────────────────

/// A 32×32 height array — one height per (x, z) column.
/// Height is in [0, CHUNK_SIZE] (inclusive).
pub struct HeightMap {
    pub data: [[u8; CHUNK_SIZE]; CHUNK_SIZE],
}

impl HeightMap {
    fn new() -> Self {
        Self { data: [[0u8; CHUNK_SIZE]; CHUNK_SIZE] }
    }

    pub fn get(&self, x: usize, z: usize) -> u8 {
        self.data[z][x]
    }

    pub fn max_height(&self) -> u8 {
        self.data.iter().flatten().copied().max().unwrap_or(0)
    }
}

// ── Terrain generator ─────────────────────────────────────────────────────────

/// Generate a `VoxelChunk` from a biome specification and chunk position.
pub fn generate_chunk(pos: ChunkPos, biome: &Biome) -> VoxelChunk {
    let seed    = biome.voxel_rules.density as u64 * 1000 + 42; // derive from density + fixed offset
    let hmap    = build_heightmap(pos, biome, seed);
    let mut chunk = VoxelChunk::empty(pos);

    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            let terrain_h = hmap.get(x, z) as usize;

            // World y position relative to this chunk's origin
            // The chunk origin is at y=0 for surface chunks.
            for y in 0..terrain_h.min(CHUNK_SIZE) {
                let depth_frac = y as f32 / terrain_h.max(1) as f32;
                let mat = pick_material(biome, depth_frac, x, y, z, seed);
                let emission = pick_emission(biome, mat, depth_frac);
                let voxel = if emission > 0 {
                    Voxel::emissive(mat, emission)
                } else if mat == materials::WATER || mat == materials::LAVA {
                    Voxel::liquid(mat)
                } else {
                    Voxel::solid(mat)
                };
                chunk.set(x, y, z, voxel);
            }
        }
    }

    let surface_y = hmap.max_height();
    chunk.meta = ChunkMeta {
        biome:        biome.name.clone(),
        seed,
        nav_passable: surface_y > 0 && surface_y < CHUNK_SIZE as u8,
        surface_y,
        fill_count:   0, // recomputed in rehash
    };
    chunk.rehash();
    chunk
}

// ── Heightmap generation ──────────────────────────────────────────────────────

fn build_heightmap(pos: ChunkPos, biome: &Biome, seed: u64) -> HeightMap {
    let mut hmap = HeightMap::new();
    let rules    = &biome.voxel_rules;
    let s = CHUNK_SIZE as f64;
    // World-space base coordinate
    let (wx, _, wz) = pos.world_origin();

    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            let wx_f = (wx as f64 + x as f64) * rules.frequency;
            let wz_f = (wz as f64 + z as f64) * rules.frequency;

            let raw = match rules.terrain_style {
                TerrainStyle::Flat => {
                    0.40 + noise::fbm_2d(wx_f, wz_f, 2, seed, 1.0, 2.0, 0.5) * 0.10
                }
                TerrainStyle::Hills => {
                    noise::fbm_2d(wx_f, wz_f, rules.octaves, seed, 1.0, 2.0, 0.5)
                }
                TerrainStyle::Dense => {
                    let base = noise::fbm_2d(wx_f, wz_f, rules.octaves, seed, 1.0, 2.0, 0.5);
                    let detail = noise::worley_2d(wx_f * 0.5, wz_f * 0.5, seed + 100) * 0.20;
                    (base + detail).clamp(0.0, 1.0)
                }
                TerrainStyle::Cliff => {
                    let r = noise::ridge_noise_2d(wx_f, wz_f, rules.octaves, seed, 1.0);
                    let base = noise::fbm_2d(wx_f, wz_f, 3, seed + 500, 1.0, 2.0, 0.5) * 0.3;
                    (r * 0.7 + base).clamp(0.0, 1.0)
                }
                TerrainStyle::Cave => {
                    let surface = noise::fbm_2d(wx_f, wz_f, 3, seed, 1.0, 2.0, 0.5) * 0.5;
                    (rules.density as f64 * 0.6 + surface * 0.4).clamp(0.0, 1.0)
                }
                TerrainStyle::Volcanic => {
                    let r = noise::ridge_noise_2d(wx_f, wz_f, rules.octaves, seed, 1.0);
                    let warp = noise::warped_fbm_2d(wx_f * 0.5, wz_f * 0.5, 4, seed + 200, 1.0) * 0.25;
                    (r * 0.75 + warp).clamp(0.0, 1.0)
                }
                TerrainStyle::Underwater => {
                    let base = noise::fbm_2d(wx_f, wz_f, 3, seed, 1.0, 2.0, 0.5) * 0.35;
                    rules.density as f64 * 0.3 + base
                }
                TerrainStyle::Alien => {
                    noise::warped_fbm_2d(wx_f, wz_f, rules.octaves, seed, 1.0)
                }
            };

            // Apply density scaling and clamp to CHUNK_SIZE
            let scaled = (raw * rules.density as f64 * s).clamp(1.0, s - 1.0);
            hmap.data[z][x] = scaled as u8;
        }
    }
    hmap
}

// ── Material selection ────────────────────────────────────────────────────────

/// Pick a material based on depth fraction and biome layer rules.
fn pick_material(
    biome:      &Biome,
    depth_frac: f32,
    x:          usize,
    _y:         usize,
    z:          usize,
    seed:       u64,
) -> u16 {
    for layer in &biome.voxel_rules.layers {
        if depth_frac >= layer.depth_start && depth_frac < layer.depth_end {
            // Mix secondary material stochastically
            if layer.secondary != 0 && layer.mix_ratio > 0.0 {
                let n = noise::hash2(x as f64, z as f64, seed + 9999) as f32;
                if n < layer.mix_ratio {
                    return layer.secondary;
                }
            }
            return layer.material;
        }
    }
    // Fallback: bedrock at bottom, surface at top
    if depth_frac < 0.15 {
        biome.voxel_rules.bedrock
    } else {
        biome.voxel_rules.surface
    }
}

/// Determine emission level for a voxel given its material and biome.
fn pick_emission(biome: &Biome, material: u16, depth_frac: f32) -> u8 {
    match &biome.voxel_rules.emission {
        EmissionMode::None => 0,
        EmissionMode::NightGlow { intensity } => {
            if material == materials::CRYSTAL_RED
                || material == materials::CRYSTAL_BLUE
                || material == materials::DARK_CRYSTAL
            {
                *intensity
            } else { 0 }
        }
        EmissionMode::LavaGlow => {
            if material == materials::LAVA { 12 } else { 0 }
        }
        EmissionMode::CrystalPulse { intensity } => {
            if material == materials::CRYSTAL_BLUE
                || material == materials::CRYSTAL_RED
                || material == materials::GLOWSTONE
                || material == materials::DARK_CRYSTAL
            {
                *intensity
            } else { 0 }
        }
        EmissionMode::Bio => {
            if (material == materials::CORAL || material == materials::MOSS) && depth_frac > 0.7 {
                3
            } else { 0 }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::biome::BiomeRegistry;

    fn get_biome(name: &str) -> Biome {
        BiomeRegistry::with_builtins().get(name).unwrap().clone()
    }

    #[test]
    fn generate_plains_chunk() {
        let b = get_biome("plains");
        let c = generate_chunk(ChunkPos::default(), &b);
        assert!(!c.state_hash.iter().all(|&b| b == 0));
        assert!(c.fill_count() > 0, "plains chunk should have voxels");
        assert_eq!(c.meta.biome, "plains");
    }

    #[test]
    fn generate_crimson_forest_chunk() {
        let b = get_biome("crimson_forest");
        let c = generate_chunk(ChunkPos::new(5, 0, -3), &b);
        assert!(c.fill_count() > 0);
        // Crimson forest should have density > plains
        let plains_c = generate_chunk(ChunkPos::new(5, 0, -3), &get_biome("plains"));
        assert!(c.fill_count() > plains_c.fill_count(),
            "crimson forest should be denser than plains: {} vs {}", c.fill_count(), plains_c.fill_count());
    }

    #[test]
    fn generate_volcanic_chunk() {
        let b = get_biome("volcanic_wastes");
        let c = generate_chunk(ChunkPos::default(), &b);
        // Volcanic should contain lava or magma_rock voxels
        let has_hot = c.non_air_voxels().any(|(_, _, _, v)| {
            v.material == materials::LAVA || v.material == materials::MAGMA_ROCK
        });
        assert!(has_hot, "volcanic chunk should contain lava or magma_rock");
    }

    #[test]
    fn determinism_same_chunk_same_hash() {
        let b = get_biome("crimson_forest");
        let c1 = generate_chunk(ChunkPos::new(3, 0, 7), &b);
        let c2 = generate_chunk(ChunkPos::new(3, 0, 7), &b);
        assert_eq!(c1.state_hash, c2.state_hash,
            "same (pos, biome) must produce identical state_hash");
    }

    #[test]
    fn different_positions_different_hash() {
        let b = get_biome("forest");
        let c1 = generate_chunk(ChunkPos::new(0, 0, 0), &b);
        let c2 = generate_chunk(ChunkPos::new(1, 0, 0), &b);
        assert_ne!(c1.state_hash, c2.state_hash,
            "adjacent chunks should differ");
    }

    #[test]
    fn emissive_voxels_in_crystal_caves() {
        let b = get_biome("crystal_caves");
        let c = generate_chunk(ChunkPos::default(), &b);
        let has_emission = c.non_air_voxels().any(|(_, _, _, v)| v.is_emissive());
        assert!(has_emission, "crystal caves should have emissive voxels");
    }
}
