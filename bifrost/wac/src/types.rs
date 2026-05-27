//! Core WAC types: intent, blueprints, and all asset IR variants.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Asset intent ────────────────────────────────────────────────────────────

/// What kind of world asset a blueprint describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetIntent {
    /// A 3-D voxel structure (building, cave, terrain feature).
    VoxelStructure,
    /// Biome rules: terrain generation, materials, entity spawns.
    BiomeDefinition,
    /// Loot drop table with conditional entries.
    LootTable,
    /// Entity animation state machine.
    AnimationGraph,
    /// Fully-specified entity prefab (stats + behavior + animation).
    EntityPrefab,
}

// ─── Blueprint ───────────────────────────────────────────────────────────────

/// The raw input to the WAC pipeline.
///
/// This is what an LLM or designer produces.  It is **never** applied to the
/// world directly — it must pass through [`validate`][crate::validate] and
/// [`compile`][crate::compile] first.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetBlueprint {
    /// Stable identifier (persisted in world-data.json).
    pub id: Uuid,

    /// What kind of asset to compile.
    pub asset_type: AssetIntent,

    /// Free-text specification from the designer / LLM.
    ///
    /// Example: `"ein biome mit roten kristallwäldern die nachts leuchten"`
    pub natural_language_spec: String,

    /// Hard invariants the compiler must not violate.
    ///
    /// Examples: `"no floating voxels"`, `"max_drop_rate <= 0.05"`.
    pub constraints: Vec<String>,

    /// Determinism seed — **must not be zero**.
    ///
    /// Same (spec, constraints, seed) ⟹ identical compiled output.
    pub seed: u64,
}

impl AssetBlueprint {
    pub fn new(
        asset_type: AssetIntent,
        spec: impl Into<String>,
        constraints: Vec<String>,
        seed: u64,
    ) -> Self {
        Self {
            id:                    Uuid::new_v4(),
            asset_type,
            natural_language_spec: spec.into(),
            constraints,
            seed,
        }
    }
}

// ─── Compiled IR types ───────────────────────────────────────────────────────

/// The compiled, validated output — what BIFROST actually consumes.
///
/// Wraps every IR variant under a common version-stamped envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetIR {
    /// Blueprint this was compiled from.
    pub blueprint_id: Uuid,

    /// IR format version — increment on breaking schema changes.
    pub ir_version: u32,

    /// BLAKE3 of the semantic inputs (spec + constraints) — cache key.
    pub semantic_hash: [u8; 32],

    /// The actual compiled asset.
    pub asset: CompiledAsset,
}

/// One of the five compiled asset variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CompiledAsset {
    VoxelChunk(VoxelChunkIR),
    BiomeDefinition(BiomeIR),
    LootTable(LootTableIR),
    AnimationGraph(AnimationGraphIR),
    EntityPrefab(EntityPrefabIR),
}

// ─── Voxel chunk IR ──────────────────────────────────────────────────────────

/// A deterministically generated voxel structure.
///
/// LLM never sets voxels directly — the compiler generates them from rules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoxelChunkIR {
    pub id:         String,
    pub size:       (u32, u32, u32),
    pub seed:       u64,
    pub material_palette: Vec<String>,
    /// Flat list of (x, y, z, material_index) packed as u32.
    pub blocks:     Vec<u32>,
}

// ─── Biome IR ────────────────────────────────────────────────────────────────

/// A complete biome definition ready for the world generator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BiomeIR {
    pub id:          String,
    pub display_name: String,

    // ── Terrain parameters ────────────────────────────────────────────────
    pub temperature:   f32,   // 0.0 = arctic, 1.0 = tropical
    pub humidity:      f32,   // 0.0 = arid,   1.0 = rainforest
    pub elevation:     f32,   // 0.0 = sea,     1.0 = mountain peak
    pub tree_density:  f32,   // 0.0 = none,    1.0 = impenetrable

    // ── Materials ─────────────────────────────────────────────────────────
    pub dominant_material:    String,
    pub secondary_material:   String,
    pub accent_material:      String,

    // ── Visual ────────────────────────────────────────────────────────────
    pub light_emission:       Option<LightEmission>,
    pub ambient_color:        String,   // CSS hex

    // ── Entity spawns ─────────────────────────────────────────────────────
    pub entity_spawns:        Vec<SpawnRule>,

    // ── Loot ──────────────────────────────────────────────────────────────
    pub loot_distribution:    LootGraphRef,
}

/// How a biome emits ambient light.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LightEmission {
    pub pattern:   EmissionPattern,
    pub intensity: f32,      // 0.0 – 1.0
    pub color:     String,   // CSS hex
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmissionPattern {
    Constant,
    NocturnalGlow,    // emits only at night
    SineFlicker,      // sine-wave intensity oscillation
    PulseOnPlayer,    // pulse when player is near
}

/// A single entity spawn rule inside a biome.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpawnRule {
    pub entity_id:      String,
    pub density:        f32,    // spawns per 10×10 tile area
    pub time_condition: Option<TimeCondition>,
    pub min_elevation:  Option<f32>,
    pub faction:        String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeCondition { Day, Night, Always }

/// Reference to an externally-compiled loot table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LootGraphRef {
    pub loot_table_id: String,
}

// ─── Loot table IR ───────────────────────────────────────────────────────────

/// A validated loot table with conditional drop entries.
///
/// LLM may only define *logic* ("rare crystals from bats at night").
/// The compiler converts this to typed entries with validated drop rates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LootTableIR {
    pub id:           String,
    pub display_name: String,
    /// Sum of `base_rate` across all entries must not exceed 1.0.
    pub entries:      Vec<LootEntry>,
}

/// One item in a loot table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LootEntry {
    pub item_id:     String,
    pub base_rate:   f32,      // 0.0 – 1.0
    pub min_qty:     u32,
    pub max_qty:     u32,
    pub conditions:  Vec<DropCondition>,
}

/// A condition that must be true for this entry to be eligible.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DropCondition {
    Night,
    Day,
    InBiome  { biome_id: String },
    KillType { entity_id: String },
    MinLevel { level: u32 },
}

// ─── Animation graph IR ──────────────────────────────────────────────────────

/// A finite-state animation graph for an entity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimationGraphIR {
    pub id:          String,
    pub entity_type: String,
    pub states:      Vec<AnimState>,
    pub transitions: Vec<AnimTransition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimState {
    pub id:       String,
    pub is_loop:  bool,
    pub duration_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimTransition {
    pub from:      String,
    pub to:        String,
    pub condition: TransitionCondition,
    pub priority:  u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TransitionCondition {
    PlayerNear  { radius: f32 },
    EnemyVisible,
    HealthBelow { fraction: f32 },
    HealthAbove { fraction: f32 },
    AttackHit,
    TargetLost,
    Timeout     { ms: u32 },
    OnDeath,
    OnRespawn,
}

// ─── Entity prefab IR ────────────────────────────────────────────────────────

/// A fully compiled entity prefab (NPC / mob / boss) ready for spawning.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityPrefabIR {
    pub id:             String,
    pub display_name:   String,
    pub entity_class:   EntityClass,

    // ── Combat stats ──────────────────────────────────────────────────────
    pub hp_base:        u32,
    pub atk_base:       u32,
    pub def_base:       u32,
    pub xp_reward:      u32,
    pub gold_reward:    u32,

    // ── Behaviour ─────────────────────────────────────────────────────────
    pub faction:        String,
    pub aggro_range:    f32,
    pub leash_range:    f32,
    pub flee_hp_frac:   f32,

    // ── References ────────────────────────────────────────────────────────
    pub animation_graph_id: String,
    pub loot_table_id:      String,

    // ── Visual ────────────────────────────────────────────────────────────
    pub icon:           String,
    pub color:          String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityClass { Mob, Boss, NpcFriendly, NpcNeutral, NpcHostile }
