//! WAC Adapter — World Assembly Code → VoxelChunk bridge.
//!
//! WAC is the JSON intermediate representation emitted by the AI/LLM layer.
//! The adapter parses WAC specs, resolves material names, constructs `Biome`
//! objects, and delegates to the voxel kernel's terrain generator.
//!
//! # Pipeline
//!
//! ```text
//! LLM / NPC / World Director
//!         ↓  JSON string
//! parse_wac()  →  WacSpec
//!         ↓
//! RuntimeAdapter::apply()
//!         ↓  builds Biome + registers custom materials
//! generate_chunk()   →  VoxelChunk
//!         ↓
//! WorldRuntime::insert_chunk()
//!         ↓
//! ChunkStreamer → BIFROST
//! ```
//!
//! # WAC Format (examples)
//!
//! **Biome definition + chunk generation:**
//! ```json
//! {
//!   "type": "biome_chunk",
//!   "pos": {"x": 5, "y": 0, "z": -3},
//!   "name": "crimson_forest",
//!   "seed": 1337,
//!   "rules": {
//!     "terrain": "dense",
//!     "material": ["crystal_red", "obsidian"],
//!     "emission": "night_glow",
//!     "density": 0.82
//!   }
//! }
//! ```
//!
//! **Reference a built-in biome:**
//! ```json
//! { "type": "chunk", "pos": {"x": 0, "y": 0, "z": 0}, "biome": "volcanic_wastes" }
//! ```
//!
//! **Register a custom material and use it:**
//! ```json
//! {
//!   "type": "material",
//!   "name": "alien_ore",
//!   "color": [0, 200, 100, 255],
//!   "emission": 6,
//!   "solid": true
//! }
//! ```

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::core::{ChunkPos, MaterialFlags, MaterialPalette, VoxelChunk};
use crate::core::materials;
use crate::generator::biome::{
    Biome, EmissionMode, MaterialLayer, TerrainStyle, VoxelRuleSet,
};
use crate::generator::terrain::generate_chunk;
use crate::runtime::{ChunkStreamer, WorldRuntime};

// ── WAC Error ─────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum WacError {
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unknown WAC type: {0}")]
    UnknownType(String),
    #[error("missing required field: {0}")]
    MissingField(&'static str),
    #[error("unknown terrain style: {0}")]
    UnknownTerrain(String),
}

// ── WAC Spec types (deserialized from JSON) ────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkPosSpec {
    pub x: i32,
    #[serde(default)]
    pub y: i32,
    pub z: i32,
}
impl From<ChunkPosSpec> for ChunkPos {
    fn from(s: ChunkPosSpec) -> Self { ChunkPos::new(s.x, s.y, s.z) }
}

/// Rules block inside a WAC biome spec.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BiomeRulesSpec {
    /// Terrain style: "flat" | "hills" | "dense" | "cliff" | "cave" |
    ///                "volcanic" | "underwater" | "alien"
    #[serde(default)]
    pub terrain:     Option<String>,
    /// Material names to use (resolved via palette).
    #[serde(default)]
    pub material:    Option<Vec<String>>,
    /// Emission mode: "none" | "night_glow" | "lava_glow" | "crystal_pulse" | "bio"
    #[serde(default)]
    pub emission:    Option<String>,
    /// Terrain density [0.0–1.0].
    #[serde(default)]
    pub density:     Option<f32>,
    /// Horizontal frequency [0.01–0.15].
    #[serde(default)]
    pub frequency:   Option<f64>,
    /// Temperature [0.0 frozen – 1.0 volcanic].
    #[serde(default)]
    pub temperature: Option<f32>,
    /// Humidity [0.0 arid – 1.0 submerged].
    #[serde(default)]
    pub humidity:    Option<f32>,
}

/// A WAC document — tagged union by `"type"` field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WacSpec {
    /// Generate a chunk at pos using a named built-in biome.
    Chunk {
        pos:   ChunkPosSpec,
        biome: String,
    },
    /// Define a new biome and generate a chunk at pos.
    BiomeChunk {
        pos:   ChunkPosSpec,
        name:  String,
        #[serde(default)]
        seed:  Option<u64>,
        #[serde(default)]
        rules: Option<BiomeRulesSpec>,
    },
    /// Define a new biome and register it (no chunk generated).
    RegisterBiome {
        name:  String,
        #[serde(default)]
        rules: Option<BiomeRulesSpec>,
    },
    /// Register a custom material in the palette.
    Material {
        name:     String,
        color:    [u8; 4],
        #[serde(default)]
        emission: Option<u8>,
        #[serde(default)]
        solid:    Option<bool>,
        #[serde(default)]
        liquid:   Option<bool>,
    },
    /// Generate a 3D region (2*radius+1)³ of chunks using a biome.
    Region {
        center: ChunkPosSpec,
        radius: i32,
        biome:  String,
    },
}

// ── WacResult ─────────────────────────────────────────────────────────────────

/// Outcome of processing a WAC document.
#[derive(Debug)]
pub enum WacResult {
    ChunkGenerated(Box<VoxelChunk>),
    RegionGenerated(Vec<Box<VoxelChunk>>),
    BiomeRegistered(String),
    MaterialRegistered { name: String, id: u16 },
}

impl WacResult {
    pub fn is_chunk(&self) -> bool { matches!(self, Self::ChunkGenerated(_)) }
    pub fn chunk(self) -> Option<Box<VoxelChunk>> {
        match self { Self::ChunkGenerated(c) => Some(c), _ => None }
    }
}

// ── Biome IR ──────────────────────────────────────────────────────────────────

// R1 — One concept, one crate.
// BiomeIR is defined once in bifrost-wac (the "World Type Authority").
// bifrost-kernel imports it from there rather than redefining it.
pub use bifrost_wac::types::BiomeIR;

// ── RuntimeAdapter ────────────────────────────────────────────────────────────

/// The main WAC entry point — wraps WorldRuntime and exposes the WAC interface.
///
/// ```rust,ignore
/// let mut rt = RuntimeAdapter::new();
/// let result = rt.apply(r#"{
///   "type": "biome_chunk",
///   "pos": {"x": 0, "y": 0, "z": 0},
///   "name": "crimson_forest",
///   "seed": 1337,
///   "rules": {"terrain": "dense", "density": 0.82}
/// }"#).unwrap();
/// ```
pub struct RuntimeAdapter {
    pub world:    WorldRuntime,
    pub streamer: ChunkStreamer,
}

impl RuntimeAdapter {
    pub fn new() -> Self {
        Self {
            world:    WorldRuntime::new(),
            streamer: ChunkStreamer::new(),
        }
    }

    /// Parse and apply a WAC JSON string.
    pub fn apply(&mut self, wac_json: &str) -> Result<WacResult, WacError> {
        let spec: WacSpec = serde_json::from_str(wac_json)?;
        self.apply_spec(spec)
    }

    /// Apply a parsed WacSpec.
    pub fn apply_spec(&mut self, spec: WacSpec) -> Result<WacResult, WacError> {
        match spec {
            WacSpec::Chunk { pos, biome } => {
                let pos   = ChunkPos::from(pos);
                let chunk = {
                    let b = self.world.biomes.get_or_default(&biome).clone();
                    generate_chunk(pos, &b)
                };
                self.world.insert_chunk(chunk.clone());
                self.streamer.mark_generated(pos);
                Ok(WacResult::ChunkGenerated(Box::new(chunk)))
            }

            WacSpec::BiomeChunk { pos, name, seed, rules } => {
                let biome = build_biome_from_spec(&name, seed, rules.as_ref(), &self.world.palette)?;
                // Optionally register the biome
                self.world.biomes.register(biome.clone());
                let pos   = ChunkPos::from(pos);
                let chunk = generate_chunk(pos, &biome);
                self.world.insert_chunk(chunk.clone());
                self.streamer.mark_generated(pos);
                Ok(WacResult::ChunkGenerated(Box::new(chunk)))
            }

            WacSpec::RegisterBiome { name, rules } => {
                let biome = build_biome_from_spec(&name, None, rules.as_ref(), &self.world.palette)?;
                self.world.biomes.register(biome);
                Ok(WacResult::BiomeRegistered(name))
            }

            WacSpec::Material { name, color, emission, solid, liquid } => {
                let flags = MaterialFlags {
                    solid:       solid.unwrap_or(true),
                    liquid:      liquid.unwrap_or(false),
                    transparent: false,
                    emissive:    emission.map(|e| e > 0).unwrap_or(false),
                    flammable:   false,
                };
                let id = self.world.palette.register(name.clone(), color, flags, emission.unwrap_or(0));
                Ok(WacResult::MaterialRegistered { name, id })
            }

            WacSpec::Region { center, radius, biome } => {
                let center = ChunkPos::from(center);
                let biome_obj = self.world.biomes.get_or_default(&biome).clone();
                let mut chunks = Vec::new();
                for dy in -radius..=radius {
                    for dz in -radius..=radius {
                        for dx in -radius..=radius {
                            let p = ChunkPos::new(center.x+dx, center.y+dy, center.z+dz);
                            let c = generate_chunk(p, &biome_obj);
                            self.world.insert_chunk(c.clone());
                            self.streamer.mark_generated(p);
                            chunks.push(Box::new(c));
                        }
                    }
                }
                Ok(WacResult::RegionGenerated(chunks))
            }
        }
    }

    /// Convenience: apply a `BiomeIR` directly (e.g. from the WAC pipeline or LLM output).
    ///
    /// `BiomeIR` now comes from `bifrost-wac` (R1 — single definition).
    pub fn apply_biome_ir(&mut self, ir: BiomeIR, pos: ChunkPos) -> VoxelChunk {
        let rules = build_rules_from_ir(&ir, &self.world.palette);
        let biome = Biome::new(ir.id.clone(), ir.temperature, ir.humidity, rules);
        self.world.biomes.register(biome.clone());
        let chunk = generate_chunk(pos, &biome);
        self.world.insert_chunk(chunk.clone());
        self.streamer.mark_generated(pos);
        chunk
    }
}

impl Default for RuntimeAdapter {
    fn default() -> Self { Self::new() }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn parse_terrain_style(s: &str) -> Result<TerrainStyle, WacError> {
    match s {
        "flat"        => Ok(TerrainStyle::Flat),
        "hills"       => Ok(TerrainStyle::Hills),
        "dense"       => Ok(TerrainStyle::Dense),
        "cliff"       => Ok(TerrainStyle::Cliff),
        "cave"        => Ok(TerrainStyle::Cave),
        "volcanic"    => Ok(TerrainStyle::Volcanic),
        "underwater"  => Ok(TerrainStyle::Underwater),
        "alien"       => Ok(TerrainStyle::Alien),
        other         => Err(WacError::UnknownTerrain(other.to_string())),
    }
}

fn parse_emission(s: &str) -> EmissionMode {
    match s {
        "night_glow"    => EmissionMode::NightGlow { intensity: 8 },
        "lava_glow"     => EmissionMode::LavaGlow,
        "crystal_pulse" => EmissionMode::CrystalPulse { intensity: 8 },
        "bio"           => EmissionMode::Bio,
        _               => EmissionMode::None,
    }
}

fn build_biome_from_spec(
    name:    &str,
    _seed:   Option<u64>,
    rules:   Option<&BiomeRulesSpec>,
    palette: &MaterialPalette,
) -> Result<Biome, WacError> {
    let r = rules.cloned().unwrap_or_default();

    let terrain_style = r.terrain.as_deref()
        .map(parse_terrain_style)
        .transpose()?
        .unwrap_or(TerrainStyle::Hills);

    let emission = r.emission.as_deref()
        .map(parse_emission)
        .unwrap_or(EmissionMode::None);

    let density   = r.density.unwrap_or(0.55).clamp(0.05, 1.0);
    let frequency = r.frequency.unwrap_or(0.04).clamp(0.005, 0.2);

    // Resolve primary materials from names
    let mats: Vec<u16> = r.material.as_ref()
        .map(|names| names.iter().map(|n| palette.resolve_name(n)).collect())
        .unwrap_or_else(|| vec![materials::STONE]);

    let surface    = *mats.first().unwrap_or(&materials::GRASS);
    let subsurface = *mats.get(1).unwrap_or(&materials::DIRT);
    let bedrock    = *mats.get(2).unwrap_or(&materials::STONE);

    // Build layered material rules from the provided materials
    let layers = build_layers_from_materials(&mats, palette);

    let vr = VoxelRuleSet {
        terrain_style,
        density,
        surface,
        subsurface,
        bedrock,
        frequency,
        octaves: 5,
        emission,
        layers,
        feature_density: density * 0.3,
    };

    Ok(Biome::new(name, r.temperature.unwrap_or(0.5), r.humidity.unwrap_or(0.5), vr))
}

/// Distribute provided materials across height layers.
fn build_layers_from_materials(mats: &[u16], _palette: &MaterialPalette) -> Vec<MaterialLayer> {
    match mats.len() {
        0 => vec![MaterialLayer::solid(0.0, 1.0, materials::STONE)],
        1 => vec![
            MaterialLayer::solid(0.0, 0.4, materials::STONE),
            MaterialLayer::solid(0.4, 0.8, mats[0]),
            MaterialLayer::solid(0.8, 1.0, mats[0]),
        ],
        2 => vec![
            MaterialLayer::solid(0.0, 0.3, materials::STONE),
            MaterialLayer::solid(0.3, 0.65, mats[1]),
            MaterialLayer::mixed(0.65, 1.0, mats[0], mats[1], 0.30),
        ],
        _ => {
            // Distribute evenly
            let n = mats.len();
            let step = 1.0 / n as f32;
            mats.iter().enumerate().map(|(i, &mat)| {
                let next_mat = if i + 1 < n { mats[i+1] } else { mat };
                MaterialLayer::mixed(
                    i as f32 * step,
                    (i + 1) as f32 * step,
                    mat, next_mat, 0.20,
                )
            }).collect()
        }
    }
}

/// Derive nexus-kernel voxel rules from the canonical bifrost-wac `BiomeIR`.
///
/// Maps the richer WAC spec onto the kernel's internal `VoxelRuleSet`:
/// - Material strings → resolved palette IDs
/// - Biome `id` → `TerrainStyle` heuristic
/// - `elevation` → noise frequency (higher elevation = broader terrain features)
/// - `tree_density` → fill density
/// - `light_emission` → `EmissionMode`
fn build_rules_from_ir(ir: &BiomeIR, palette: &MaterialPalette) -> VoxelRuleSet {
    // Resolve string material names to palette IDs.
    let mats: Vec<u16> = [
        ir.dominant_material.as_str(),
        ir.secondary_material.as_str(),
        ir.accent_material.as_str(),
    ]
    .iter()
    .map(|name| palette.resolve_name(name))
    .collect();

    let surface    = *mats.first().unwrap_or(&materials::GRASS);
    let subsurface = *mats.get(1).unwrap_or(&materials::DIRT);
    let bedrock    = *mats.get(2).unwrap_or(&materials::STONE);
    let layers     = build_layers_from_materials(&mats, palette);

    VoxelRuleSet {
        terrain_style:  terrain_style_from_biome_id(&ir.id),
        density:        ir.tree_density,
        surface,
        subsurface,
        bedrock,
        // Higher elevation → broader, lower-frequency terrain features.
        frequency:      (0.02 + (1.0 - ir.elevation as f64) * 0.04).clamp(0.005, 0.12),
        octaves:        5,
        emission:       emission_from_wac(&ir.light_emission),
        layers,
        feature_density: ir.tree_density * 0.3,
    }
}

/// Heuristic: map a canonical biome ID to the matching `TerrainStyle`.
fn terrain_style_from_biome_id(id: &str) -> TerrainStyle {
    match id {
        "dungeon"                           => TerrainStyle::Cave,
        "volcanic"                          => TerrainStyle::Volcanic,
        "deep_water" | "water"              => TerrainStyle::Underwater,
        "mountain" | "rock"                 => TerrainStyle::Cliff,
        "crimson_forest" | "dark_forest"    => TerrainStyle::Dense,
        _                                   => TerrainStyle::Hills,
    }
}

/// Convert a WAC `LightEmission` option into a kernel `EmissionMode`.
fn emission_from_wac(le: &Option<bifrost_wac::types::LightEmission>) -> EmissionMode {
    use bifrost_wac::types::EmissionPattern;
    match le {
        None => EmissionMode::None,
        Some(l) => match l.pattern {
            EmissionPattern::NocturnalGlow =>
                EmissionMode::NightGlow { intensity: (l.intensity * 15.0) as u8 },
            EmissionPattern::SineFlicker | EmissionPattern::Constant =>
                EmissionMode::CrystalPulse { intensity: (l.intensity * 15.0) as u8 },
            EmissionPattern::PulseOnPlayer =>
                EmissionMode::Bio,
        },
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn adapter() -> RuntimeAdapter { RuntimeAdapter::new() }

    const CRIMSON_FOREST: &str = r#"{
        "type": "biome_chunk",
        "pos": {"x": 0, "y": 0, "z": 0},
        "name": "crimson_forest",
        "seed": 1337,
        "rules": {
            "terrain": "dense",
            "material": ["crystal_red", "obsidian"],
            "emission": "night_glow",
            "density": 0.82
        }
    }"#;

    const BUILTIN_CHUNK: &str = r#"{
        "type": "chunk",
        "pos": {"x": 3, "y": 0, "z": -2},
        "biome": "volcanic_wastes"
    }"#;

    const CUSTOM_MATERIAL: &str = r#"{
        "type": "material",
        "name": "alien_ore",
        "color": [0, 200, 100, 255],
        "emission": 7,
        "solid": true
    }"#;

    const REGISTER_BIOME: &str = r#"{
        "type": "register_biome",
        "name": "neon_swamp",
        "rules": {
            "terrain": "alien",
            "material": ["neon_block", "moss"],
            "emission": "bio",
            "density": 0.65,
            "humidity": 0.9
        }
    }"#;

    #[test]
    fn wac_biome_chunk_generates_voxels() {
        let mut rt = adapter();
        let result = rt.apply(CRIMSON_FOREST).unwrap();
        let chunk = result.chunk().unwrap();
        assert!(chunk.fill_count() > 0, "crimson forest chunk must have voxels");
        assert_eq!(chunk.meta.biome, "crimson_forest");
    }

    #[test]
    fn wac_builtin_chunk() {
        let mut rt = adapter();
        let result = rt.apply(BUILTIN_CHUNK).unwrap();
        assert!(result.is_chunk());
        let chunk = result.chunk().unwrap();
        assert!(chunk.fill_count() > 0);
    }

    #[test]
    fn wac_custom_material() {
        let mut rt = adapter();
        let result = rt.apply(CUSTOM_MATERIAL).unwrap();
        match result {
            WacResult::MaterialRegistered { name, id } => {
                assert_eq!(name, "alien_ore");
                assert!(id >= crate::core::materials::FIRST_CUSTOM);
                // Verify it's in the palette
                assert!(rt.world.palette.get(id).is_some());
            }
            _ => panic!("expected MaterialRegistered"),
        }
    }

    #[test]
    fn wac_register_biome() {
        let mut rt = adapter();
        let result = rt.apply(REGISTER_BIOME).unwrap();
        match result {
            WacResult::BiomeRegistered(name) => {
                assert_eq!(name, "neon_swamp");
                assert!(rt.world.biomes.get("neon_swamp").is_some());
            }
            _ => panic!("expected BiomeRegistered"),
        }
    }

    #[test]
    fn wac_register_then_use() {
        let mut rt = adapter();
        rt.apply(REGISTER_BIOME).unwrap();
        // Now use the registered biome
        let chunk_wac = r#"{"type":"chunk","pos":{"x":1,"y":0,"z":1},"biome":"neon_swamp"}"#;
        let result = rt.apply(chunk_wac).unwrap();
        assert!(result.is_chunk());
    }

    #[test]
    fn wac_determinism() {
        // Same spec twice → same state_hash
        let result1 = adapter().apply(CRIMSON_FOREST).unwrap().chunk().unwrap();
        let result2 = adapter().apply(CRIMSON_FOREST).unwrap().chunk().unwrap();
        assert_eq!(result1.state_hash, result2.state_hash,
            "identical WAC spec must produce identical chunk hash");
    }

    #[test]
    fn wac_region() {
        let mut rt = adapter();
        let region_wac = r#"{
            "type": "region",
            "center": {"x": 0, "y": 0, "z": 0},
            "radius": 1,
            "biome": "forest"
        }"#;
        let result = rt.apply(region_wac).unwrap();
        match result {
            WacResult::RegionGenerated(chunks) => {
                assert_eq!(chunks.len(), 3*3*3, "radius=1 should produce 27 chunks");
                assert!(rt.world.chunk_count() == 27);
            }
            _ => panic!("expected RegionGenerated"),
        }
    }

    #[test]
    fn wac_biome_ir_direct() {
        use bifrost_wac::types::LootGraphRef;
        let mut rt = adapter();
        // BiomeIR now comes from bifrost-wac (R1 — single definition).
        let ir = BiomeIR {
            id:                 "test_biome".into(),
            display_name:       "Test Biome".into(),
            temperature:        0.7,
            humidity:           0.4,
            elevation:          0.3,
            tree_density:       0.6,
            dominant_material:  "grass".into(),
            secondary_material: "dirt".into(),
            accent_material:    "stone".into(),
            light_emission:     None,
            ambient_color:      "#1e4820".into(),
            entity_spawns:      vec![],
            loot_distribution:  LootGraphRef { loot_table_id: "lt_test_biome".into() },
        };
        let chunk = rt.apply_biome_ir(ir, ChunkPos::default());
        assert!(chunk.fill_count() > 0);
        assert_eq!(chunk.meta.biome, "test_biome");
    }

    #[test]
    fn invalid_terrain_style_error() {
        let mut rt = adapter();
        let bad_wac = r#"{
            "type": "biome_chunk",
            "pos": {"x": 0, "y": 0, "z": 0},
            "name": "bad",
            "rules": {"terrain": "nonexistent_style"}
        }"#;
        assert!(matches!(rt.apply(bad_wac), Err(WacError::UnknownTerrain(_))));
    }

    #[test]
    fn wac_chunk_inserted_into_world() {
        let mut rt = adapter();
        rt.apply(CRIMSON_FOREST).unwrap();
        // The chunk should now be accessible from the world
        let pos = ChunkPos::new(0, 0, 0);
        assert!(rt.world.get_chunk(pos).is_some(),
            "generated chunk should be in world runtime");
    }
}
