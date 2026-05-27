//! Biome system — physical rule instances for world synthesis.
//!
//! A Biome is not a theme — it is a **physics specification** that drives
//! the terrain generator. The AI (WAC adapter) selects or creates biomes;
//! the kernel executes them deterministically.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::core::materials;

// ── TerrainStyle ──────────────────────────────────────────────────────────────

/// How the terrain height profile is generated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerrainStyle {
    /// Nearly flat, minimal variation.
    Flat,
    /// Gentle rolling hills.
    Hills,
    /// Dense mixed terrain, moderate heights.
    Dense,
    /// Dramatic cliffs and deep valleys.
    Cliff,
    /// Low terrain with extensive cave systems.
    Cave,
    /// Extreme peaks, lava flows, obsidian fields.
    Volcanic,
    /// Submerged, flat base with sand/coral structures.
    Underwater,
    /// Warped, alien-looking fBm with ridge lines.
    Alien,
}

// ── EmissionMode ─────────────────────────────────────────────────────────────

/// How light emission is distributed in the biome.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EmissionMode {
    /// No emission.
    None,
    /// Emissive voxels at specific material types.
    NightGlow { intensity: u8 },
    /// Lava flows emit heat light.
    LavaGlow,
    /// Crystal formations glow with specific color.
    CrystalPulse { intensity: u8 },
    /// Bioluminescent organic materials.
    Bio,
}

// ── MaterialLayer ─────────────────────────────────────────────────────────────

/// Assigns a material to a height range within a chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialLayer {
    /// Fraction of chunk height at which this layer starts (0.0 = bottom).
    pub depth_start: f32,
    /// Fraction at which this layer ends (1.0 = top).
    pub depth_end:   f32,
    /// Primary material ID for this layer.
    pub material:    u16,
    /// Optional second material mixed in via noise (0 = none).
    pub secondary:   u16,
    /// Mixing probability for secondary (0.0–1.0).
    pub mix_ratio:   f32,
}

impl MaterialLayer {
    pub fn solid(depth_start: f32, depth_end: f32, material: u16) -> Self {
        Self { depth_start, depth_end, material, secondary: 0, mix_ratio: 0.0 }
    }
    pub fn mixed(depth_start: f32, depth_end: f32, material: u16, secondary: u16, mix: f32) -> Self {
        Self { depth_start, depth_end, material, secondary, mix_ratio: mix }
    }
}

// ── VoxelRuleSet ─────────────────────────────────────────────────────────────

/// The rule set that drives chunk generation for a biome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoxelRuleSet {
    pub terrain_style:  TerrainStyle,
    /// Terrain density [0.0–1.0]: how tall/full the terrain is.
    pub density:        f32,
    /// Surface material (top visible voxel layer).
    pub surface:        u16,
    /// Fill material (subsurface).
    pub subsurface:     u16,
    /// Deepest layer material.
    pub bedrock:        u16,
    /// Horizontal terrain frequency [0.01–0.1].
    pub frequency:      f64,
    /// fBm octaves for terrain (4–8).
    pub octaves:        u32,
    pub emission:       EmissionMode,
    /// Ordered material layers (surface to deep).
    pub layers:         Vec<MaterialLayer>,
    /// Feature placement: extra voxel objects placed on surface.
    pub feature_density: f32,
}

// ── EntitySpawn ───────────────────────────────────────────────────────────────

/// Defines which entity types can spawn in this biome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySpawn {
    pub entity_type: String,
    pub weight:      f32,   // relative spawn weight
    pub min_y:       f32,   // min terrain height fraction
    pub max_y:       f32,   // max terrain height fraction
}

// ── Biome ─────────────────────────────────────────────────────────────────────

/// A physical biome specification — a rule instance for world synthesis.
///
/// > "AI defines physics + world rules → Kernel builds reality."
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Biome {
    pub name:         String,
    /// Temperature [0.0 = frozen, 1.0 = volcanic].
    pub temperature:  f32,
    /// Humidity [0.0 = arid, 1.0 = submerged].
    pub humidity:     f32,
    pub voxel_rules:  VoxelRuleSet,
    pub spawn_rules:  Vec<EntitySpawn>,
}

impl Biome {
    pub fn new(
        name:        impl Into<String>,
        temperature: f32,
        humidity:    f32,
        rules:       VoxelRuleSet,
    ) -> Self {
        Self {
            name: name.into(),
            temperature,
            humidity,
            voxel_rules: rules,
            spawn_rules: Vec::new(),
        }
    }
}

// ── Built-in biomes ───────────────────────────────────────────────────────────

fn plains() -> Biome {
    Biome::new("plains", 0.5, 0.4, VoxelRuleSet {
        terrain_style: TerrainStyle::Flat,
        density: 0.45, surface: materials::GRASS, subsurface: materials::DIRT, bedrock: materials::STONE,
        frequency: 0.03, octaves: 4,
        emission: EmissionMode::None,
        layers: vec![
            MaterialLayer::solid(0.0, 0.5, materials::STONE),
            MaterialLayer::solid(0.5, 0.85, materials::DIRT),
            MaterialLayer::solid(0.85, 1.0, materials::GRASS),
        ],
        feature_density: 0.05,
    })
}

fn forest() -> Biome {
    Biome::new("forest", 0.55, 0.6, VoxelRuleSet {
        terrain_style: TerrainStyle::Hills,
        density: 0.55, surface: materials::GRASS, subsurface: materials::DIRT, bedrock: materials::STONE,
        frequency: 0.04, octaves: 5,
        emission: EmissionMode::None,
        layers: vec![
            MaterialLayer::solid(0.0, 0.45, materials::STONE),
            MaterialLayer::mixed(0.45, 0.8, materials::DIRT, materials::GRAVEL, 0.15),
            MaterialLayer::solid(0.8, 1.0, materials::GRASS),
        ],
        feature_density: 0.20,
    })
}

fn crimson_forest() -> Biome {
    Biome::new("crimson_forest", 0.75, 0.35, VoxelRuleSet {
        terrain_style: TerrainStyle::Dense,
        density: 0.82,
        surface: materials::NIGHTWOOD, subsurface: materials::OBSIDIAN, bedrock: materials::VOID_MATTER,
        frequency: 0.05, octaves: 6,
        emission: EmissionMode::NightGlow { intensity: 8 },
        layers: vec![
            MaterialLayer::solid(0.0, 0.3, materials::VOID_MATTER),
            MaterialLayer::mixed(0.3, 0.65, materials::OBSIDIAN, materials::DARK_CRYSTAL, 0.25),
            MaterialLayer::mixed(0.65, 0.88, materials::NIGHTWOOD, materials::CRYSTAL_RED, 0.35),
            MaterialLayer::solid(0.88, 1.0, materials::CRYSTAL_RED),
        ],
        feature_density: 0.35,
    })
}

fn ice_plains() -> Biome {
    Biome::new("ice_plains", 0.05, 0.8, VoxelRuleSet {
        terrain_style: TerrainStyle::Flat,
        density: 0.40,
        surface: materials::SNOW, subsurface: materials::ICE, bedrock: materials::STONE,
        frequency: 0.02, octaves: 3,
        emission: EmissionMode::None,
        layers: vec![
            MaterialLayer::solid(0.0, 0.55, materials::STONE),
            MaterialLayer::solid(0.55, 0.80, materials::ICE),
            MaterialLayer::solid(0.80, 1.0, materials::SNOW),
        ],
        feature_density: 0.02,
    })
}

fn volcanic_wastes() -> Biome {
    Biome::new("volcanic_wastes", 0.95, 0.10, VoxelRuleSet {
        terrain_style: TerrainStyle::Volcanic,
        density: 0.70,
        surface: materials::MAGMA_ROCK, subsurface: materials::OBSIDIAN, bedrock: materials::VOID_MATTER,
        frequency: 0.06, octaves: 7,
        emission: EmissionMode::LavaGlow,
        layers: vec![
            MaterialLayer::solid(0.0, 0.25, materials::VOID_MATTER),
            MaterialLayer::mixed(0.25, 0.55, materials::OBSIDIAN, materials::LAVA, 0.20),
            MaterialLayer::mixed(0.55, 0.80, materials::MAGMA_ROCK, materials::OBSIDIAN, 0.30),
            MaterialLayer::solid(0.80, 1.0, materials::MAGMA_ROCK),
        ],
        feature_density: 0.15,
    })
}

fn ocean_floor() -> Biome {
    Biome::new("ocean_floor", 0.40, 1.0, VoxelRuleSet {
        terrain_style: TerrainStyle::Underwater,
        density: 0.30,
        surface: materials::SAND, subsurface: materials::GRAVEL, bedrock: materials::STONE,
        frequency: 0.03, octaves: 4,
        emission: EmissionMode::Bio,
        layers: vec![
            MaterialLayer::solid(0.0, 0.60, materials::STONE),
            MaterialLayer::mixed(0.60, 0.85, materials::GRAVEL, materials::SAND, 0.4),
            MaterialLayer::mixed(0.85, 1.0, materials::SAND, materials::CORAL, 0.30),
        ],
        feature_density: 0.25,
    })
}

fn dungeon() -> Biome {
    Biome::new("dungeon", 0.30, 0.20, VoxelRuleSet {
        terrain_style: TerrainStyle::Cave,
        density: 0.65,
        surface: materials::STONE, subsurface: materials::STONE, bedrock: materials::OBSIDIAN,
        frequency: 0.07, octaves: 5,
        emission: EmissionMode::CrystalPulse { intensity: 5 },
        layers: vec![
            MaterialLayer::solid(0.0, 0.40, materials::OBSIDIAN),
            MaterialLayer::mixed(0.40, 0.75, materials::STONE, materials::BONE, 0.15),
            MaterialLayer::mixed(0.75, 1.0, materials::STONE, materials::DARK_CRYSTAL, 0.10),
        ],
        feature_density: 0.08,
    })
}

fn crystal_caves() -> Biome {
    Biome::new("crystal_caves", 0.20, 0.30, VoxelRuleSet {
        terrain_style: TerrainStyle::Cave,
        density: 0.72,
        surface: materials::DARK_CRYSTAL, subsurface: materials::STONE, bedrock: materials::VOID_MATTER,
        frequency: 0.08, octaves: 6,
        emission: EmissionMode::CrystalPulse { intensity: 10 },
        layers: vec![
            MaterialLayer::solid(0.0, 0.30, materials::VOID_MATTER),
            MaterialLayer::mixed(0.30, 0.60, materials::STONE, materials::DARK_CRYSTAL, 0.40),
            MaterialLayer::mixed(0.60, 0.80, materials::CRYSTAL_BLUE, materials::CRYSTAL_RED, 0.50),
            MaterialLayer::solid(0.80, 1.0, materials::GLOWSTONE),
        ],
        feature_density: 0.50,
    })
}

// ── BiomeRegistry ─────────────────────────────────────────────────────────────

/// Registry of all known biomes — built-in + AI-registered.
#[derive(Debug, Default)]
pub struct BiomeRegistry {
    biomes: BTreeMap<String, Biome>,
}

impl BiomeRegistry {
    /// Create a registry with all built-in biomes pre-registered.
    pub fn with_builtins() -> Self {
        let mut r = Self::default();
        for b in [plains(), forest(), crimson_forest(), ice_plains(), volcanic_wastes(),
                  ocean_floor(), dungeon(), crystal_caves()] {
            r.register(b);
        }
        r
    }

    /// Register a biome (built-in or AI-generated).
    pub fn register(&mut self, b: Biome) {
        self.biomes.insert(b.name.clone(), b);
    }

    /// Look up a biome by name.
    pub fn get(&self, name: &str) -> Option<&Biome> {
        self.biomes.get(name)
    }

    /// All registered biome names in sorted order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.biomes.keys().map(|s| s.as_str())
    }

    /// Number of registered biomes.
    pub fn len(&self) -> usize { self.biomes.len() }

    /// Fall back to `"plains"` if the name is unknown.
    pub fn get_or_default<'a>(&'a self, name: &str) -> &'a Biome {
        self.biomes.get(name)
            .or_else(|| self.biomes.get("plains"))
            .expect("plains biome must always be registered")
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_builtins() {
        let r = BiomeRegistry::with_builtins();
        assert!(r.len() >= 8);
        assert!(r.get("plains").is_some());
        assert!(r.get("crimson_forest").is_some());
        assert!(r.get("volcanic_wastes").is_some());
    }

    #[test]
    fn fallback_to_plains() {
        let r = BiomeRegistry::with_builtins();
        let b = r.get_or_default("nonexistent_biome");
        assert_eq!(b.name, "plains");
    }

    #[test]
    fn ai_biome_registration() {
        let mut r = BiomeRegistry::with_builtins();
        let before = r.len();
        let ai_biome = Biome::new("alien_swamp", 0.6, 0.9, VoxelRuleSet {
            terrain_style: TerrainStyle::Alien,
            density: 0.6,
            surface: materials::MUSHROOM, subsurface: materials::MOSS, bedrock: materials::STONE,
            frequency: 0.05, octaves: 5,
            emission: EmissionMode::Bio,
            layers: vec![MaterialLayer::solid(0.0, 1.0, materials::MUSHROOM)],
            feature_density: 0.4,
        });
        r.register(ai_biome);
        assert_eq!(r.len(), before + 1);
        assert!(r.get("alien_swamp").is_some());
    }

    #[test]
    fn crimson_forest_is_dense() {
        let r = BiomeRegistry::with_builtins();
        let b = r.get("crimson_forest").unwrap();
        assert!(b.voxel_rules.density > 0.7);
        assert_eq!(b.voxel_rules.terrain_style, TerrainStyle::Dense);
    }
}
