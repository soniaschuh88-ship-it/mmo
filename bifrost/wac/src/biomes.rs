//! Canonical biome registry — **single source of truth** for all world biomes.
//!
//! Every system in the stack that references a biome **must** use the types
//! defined here.  Before this module the following systems each maintained
//! their own diverging copies:
//!
//! | System | Was using |
//! |---|---|
//! | `app/game.html` | 14 integer constants `GD,WA,SA,GR,FO,CF,…` |
//! | `bifrost/wac` tilemap palette | 11 strings `"empty","grass","stone_floor",…` |
//! | `bifrost/server` SimState zones | `"forest"`, `"desert"`, `"dungeon"`, `"plains"` |
//!
//! ## Architecture role
//!
//! `bifrost-wac` is the **World Type Authority**.  Every downstream system
//! (nova-render, synthesis AI, loot system, world director) queries biome
//! definitions from [`BiomeRegistry::global()`] instead of hard-coding
//! properties locally.  This makes the following possible without code changes:
//!
//! * WAC compiles biome definitions
//! * nova-render reads ambient FX from the same registry
//! * Synthesis AI reads risk tier and strategic value
//! * Loot system reads drop weighting
//! * World Director reads mutation rules
//!
//! ## JS sync
//!
//! `app/game.html` BIOME constant **must** match [`BiomeKey`] ordinal order:
//!
//! ```js
//! const BIOME = {
//!   deep_water:0, water:1, sand:2, grass:3, dark_forest:4,
//!   crimson_forest:5, rock:6, mountain:7, snow:8,
//!   dungeon:9, village:10, building:11, swamp:12, volcanic:13,
//! };
//! ```

use serde::{Deserialize, Serialize};

// ─── BiomeKey ─────────────────────────────────────────────────────────────────

/// Canonical biome identifier.
///
/// The `u8` discriminant **is** the tile-palette index — it is stable and
/// serialised directly into world-data.json.  Do not change the order.
///
/// Use [`BiomeKey::from_str`] to parse untrusted input (e.g. from JSON or
/// LLM output) and fall back to [`BiomeKey::Grass`] on unknown values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum BiomeKey {
    DeepWater    = 0,
    Water        = 1,
    Sand         = 2,
    Grass        = 3,
    DarkForest   = 4,
    CrimsonForest= 5,
    Rock         = 6,
    Mountain     = 7,
    Snow         = 8,
    Dungeon      = 9,
    Village      = 10,
    Building     = 11,
    Swamp        = 12,
    Volcanic     = 13,
}

impl BiomeKey {
    /// All keys in palette-index order.  Length is always 14.
    pub const ALL: &'static [Self] = &[
        Self::DeepWater, Self::Water, Self::Sand, Self::Grass,
        Self::DarkForest, Self::CrimsonForest, Self::Rock, Self::Mountain,
        Self::Snow, Self::Dungeon, Self::Village, Self::Building,
        Self::Swamp, Self::Volcanic,
    ];

    /// Snake-case string ID — matches the JS `BIOME` object key.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DeepWater     => "deep_water",
            Self::Water         => "water",
            Self::Sand          => "sand",
            Self::Grass         => "grass",
            Self::DarkForest    => "dark_forest",
            Self::CrimsonForest => "crimson_forest",
            Self::Rock          => "rock",
            Self::Mountain      => "mountain",
            Self::Snow          => "snow",
            Self::Dungeon       => "dungeon",
            Self::Village       => "village",
            Self::Building      => "building",
            Self::Swamp         => "swamp",
            Self::Volcanic      => "volcanic",
        }
    }

    /// Parse from any supported string — canonical ID, legacy short code, or
    /// plain English synonym.  Returns `Grass` for anything unrecognised.
    pub fn from_str(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "deep_water" | "gd" | "deep sea" | "ocean"              => Self::DeepWater,
            "water"      | "wa" | "shore" | "river" | "lake"        => Self::Water,
            "sand"       | "sa" | "desert" | "beach" | "arid"       => Self::Sand,
            "grass"      | "gr" | "plains" | "field" | "grassland"  => Self::Grass,
            "dark_forest"| "fo" | "forest" | "woodland" | "jungle"  => Self::DarkForest,
            "crimson_forest" | "cf" | "crimson"                      => Self::CrimsonForest,
            "rock"       | "rk" | "stone" | "stone_floor" | "rocky" => Self::Rock,
            "mountain"   | "mn" | "peak" | "cliff" | "alpine"       => Self::Mountain,
            "snow"       | "sn" | "ice" | "tundra" | "frost"        => Self::Snow,
            "dungeon"    | "dg" | "cave" | "underground"            => Self::Dungeon,
            "village"    | "vi" | "town" | "settlement" | "safe-city" => Self::Village,
            "building"   | "bl" | "castle" | "fortress" | "interior" => Self::Building,
            "swamp"      | "sw" | "marsh" | "bog" | "sumpf"         => Self::Swamp,
            "volcanic"   | "vo" | "lava" | "magma" | "obsidian"     => Self::Volcanic,
            _                                                         => Self::Grass,
        }
    }

    /// Human-readable display name (English).
    pub fn display_name(self) -> &'static str {
        match self {
            Self::DeepWater     => "Deep Sea",
            Self::Water         => "Shore",
            Self::Sand          => "Sandy Banks",
            Self::Grass         => "Green Plains",
            Self::DarkForest    => "Dark Forest",
            Self::CrimsonForest => "Crimson Forest",
            Self::Rock          => "Rocky Highlands",
            Self::Mountain      => "Mountains",
            Self::Snow          => "Frost Peaks",
            Self::Dungeon       => "The Dungeon",
            Self::Village       => "Village",
            Self::Building      => "Building",
            Self::Swamp         => "Swamp",
            Self::Volcanic      => "Volcanic Wastes",
        }
    }
}

impl std::fmt::Display for BiomeKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// Keep the old string-slice constant for legacy callers (e.g. tilemap palette).
/// Ordered canonical biome ID strings.  Index == [`BiomeKey`] discriminant.
pub const BIOME_IDS: &[&str] = &[
    "deep_water","water","sand","grass","dark_forest",
    "crimson_forest","rock","mountain","snow",
    "dungeon","village","building","swamp","volcanic",
];

/// Canonical tile palette for the WAC TileMap compiler (= `BIOME_IDS`).
pub const TILE_PALETTE: &[&str] = BIOME_IDS;

// ─── AmbientFx ────────────────────────────────────────────────────────────────

/// How a biome creates visual atmosphere (read by nova-render and game.html).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum AmbientFx {
    None,
    /// Directional tint overlay (e.g. crimson forest red haze).
    ColorTint { color: &'static str, alpha: f32 },
    /// Floating particle motes (e.g. spores, snow).
    Motes     { color: &'static str, count: u8, speed: f32 },
    /// Animated torch sparks + dark vignette (dungeon).
    TorchGlow { vignette_alpha: f32 },
}

// ─── BiomeDefinition ──────────────────────────────────────────────────────────

/// Full world-rule definition for one biome.
///
/// Every subsystem reads from this struct — no more scattered constants.
///
/// | Consumer | Fields used |
/// |---|---|
/// | nova-render | `colors`, `ambient_fx` |
/// | Synthesis AI | `risk_tier`, `strategic_value`, `hostile_density` |
/// | Loot system | `loot_weight_multiplier` |
/// | World Director | `mutation_cost`, `risk_tier` |
/// | WAC compiler | `passable`, `voxel_fill_rate` |
/// `BiomeDefinition` is a compile-time static record — serialized to JSON for
/// APIs but never deserialized from external input.  Use [`BiomeKey`] for
/// wire-format biome references.
#[derive(Debug, Clone, Serialize)]
pub struct BiomeDefinition {
    /// Stable identifier — matches [`BiomeKey::as_str()`].
    pub id:           BiomeKey,

    /// Climate parameters used by nexus terrain generator.
    pub temperature:  f32,   // 0.0 arctic → 1.0 tropical
    pub humidity:     f32,   // 0.0 arid   → 1.0 rainforest

    /// Tile colors `(top_face, left_face, right_face)` as CSS hex.
    /// Mirrors `BIOME_COLORS` and the JS `BC` constant.
    pub colors:       (&'static str, &'static str, &'static str),

    /// Visual atmosphere overlay (nova-render + game.html canvas overlay).
    pub ambient_fx:   AmbientFx,

    /// Combat and spawning parameters.
    pub risk_tier:    u8,    // 0 = safe, 1 = contested, 2 = dangerous, 3 = boss
    /// Approximate hostile entity density (spawns per 10×10 tile area).
    pub hostile_density: f32,

    /// Economy and strategy.
    pub strategic_value:       f32,   // Synthesis AI scoring weight
    pub loot_weight_multiplier: f32,  // applied to all drop tables in this biome

    /// World mutation cost for the World Director (higher = harder to mutate).
    pub mutation_cost: f32,

    /// Whether the biome can be traversed on foot (`false` = water, deep_water).
    pub passable:      bool,

    /// 0-D voxel fill rate used by nexus terrain generator (0.0–1.0).
    pub voxel_fill_rate: f32,
}

// ─── BiomeRegistry ────────────────────────────────────────────────────────────

/// Immutable registry of all built-in biome definitions.
pub struct BiomeRegistry {
    defs: &'static [BiomeDefinition],
}

impl BiomeRegistry {
    /// The single global instance.  Constructed from compile-time data.
    pub fn global() -> &'static Self {
        &GLOBAL_REGISTRY
    }

    pub fn get(&self, key: BiomeKey) -> &BiomeDefinition {
        &self.defs[key as usize]
    }

    pub fn all(&self) -> &[BiomeDefinition] { self.defs }

    pub fn passable_ids(&self) -> impl Iterator<Item = BiomeKey> + '_ {
        self.defs.iter().filter(|d| d.passable).map(|d| d.id)
    }

    pub fn by_risk(&self, tier: u8) -> impl Iterator<Item = &BiomeDefinition> {
        self.defs.iter().filter(move |d| d.risk_tier == tier)
    }
}

// ─── Built-in definitions ─────────────────────────────────────────────────────

macro_rules! def {
    (
        $key:expr,
        temp=$t:expr, hum=$h:expr,
        colors=($c0:expr,$c1:expr,$c2:expr),
        fx=$fx:expr,
        risk=$r:expr, hostile=$hd:expr,
        strat=$sv:expr, loot=$lm:expr, mut_cost=$mc:expr,
        passable=$p:expr, fill=$vf:expr $(,)?
    ) => {
        BiomeDefinition {
            id: $key, temperature: $t, humidity: $h,
            colors: ($c0,$c1,$c2), ambient_fx: $fx,
            risk_tier: $r, hostile_density: $hd,
            strategic_value: $sv, loot_weight_multiplier: $lm,
            mutation_cost: $mc, passable: $p, voxel_fill_rate: $vf,
        }
    };
}

static BUILT_IN_DEFS: &[BiomeDefinition] = &[
    def!(BiomeKey::DeepWater,    temp=0.5,hum=1.0,
        colors=("#0c3080","#082058","#0a2870"), fx=AmbientFx::None,
        risk=0,hostile=0.0,strat=0.1,loot=0.5,mut_cost=8.0,passable=false,fill=0.0),
    def!(BiomeKey::Water,        temp=0.5,hum=1.0,
        colors=("#1860c8","#0e3888","#1450a8"), fx=AmbientFx::None,
        risk=0,hostile=0.1,strat=0.3,loot=0.7,mut_cost=6.0,passable=false,fill=0.0),
    def!(BiomeKey::Sand,         temp=0.9,hum=0.1,
        colors=("#c8a868","#906030","#a87840"), fx=AmbientFx::None,
        risk=1,hostile=0.4,strat=0.5,loot=0.9,mut_cost=3.0,passable=true,fill=0.4),
    def!(BiomeKey::Grass,        temp=0.6,hum=0.5,
        colors=("#487840","#285828","#387030"), fx=AmbientFx::None,
        risk=1,hostile=0.5,strat=1.0,loot=1.0,mut_cost=2.0,passable=true,fill=0.5),
    def!(BiomeKey::DarkForest,   temp=0.5,hum=0.8,
        colors=("#1e4820","#0e2c10","#182e18"), fx=AmbientFx::None,
        risk=1,hostile=0.8,strat=1.2,loot=1.2,mut_cost=3.5,passable=true,fill=0.65),
    def!(BiomeKey::CrimsonForest,temp=0.7,hum=0.6,
        colors=("#3a0810","#200408","#2a0408"),
        fx=AmbientFx::ColorTint{color:"rgba(180,20,40,.12)",alpha:0.12},
        risk=2,hostile=1.0,strat=1.8,loot=1.5,mut_cost=5.0,passable=true,fill=0.70),
    def!(BiomeKey::Rock,         temp=0.3,hum=0.2,
        colors=("#686070","#484058","#585068"), fx=AmbientFx::None,
        risk=1,hostile=0.5,strat=0.6,loot=0.8,mut_cost=4.0,passable=true,fill=0.8),
    def!(BiomeKey::Mountain,     temp=0.2,hum=0.3,
        colors=("#9898a8","#606070","#787888"), fx=AmbientFx::None,
        risk=2,hostile=0.7,strat=1.0,loot=1.1,mut_cost=6.0,passable=true,fill=0.85),
    def!(BiomeKey::Snow,         temp=0.0,hum=0.4,
        colors=("#d8d8f0","#9898b8","#b8b8d8"),
        fx=AmbientFx::Motes{color:"rgba(220,230,255,.6)",count:6,speed:0.3},
        risk=2,hostile=0.6,strat=0.8,loot=1.0,mut_cost=5.0,passable=true,fill=0.75),
    def!(BiomeKey::Dungeon,      temp=0.3,hum=0.5,
        colors=("#181828","#0e0e1a","#141422"),
        fx=AmbientFx::TorchGlow{vignette_alpha:0.48},
        risk=3,hostile=1.5,strat=2.5,loot=2.0,mut_cost=7.0,passable=true,fill=0.9),
    def!(BiomeKey::Village,      temp=0.6,hum=0.5,
        colors=("#80705a","#504438","#686050"), fx=AmbientFx::None,
        risk=0,hostile=0.0,strat=2.0,loot=0.8,mut_cost=9.0,passable=true,fill=0.6),
    def!(BiomeKey::Building,     temp=0.5,hum=0.3,
        colors=("#a08870","#605040","#786050"), fx=AmbientFx::None,
        risk=0,hostile=0.0,strat=1.5,loot=0.6,mut_cost=9.0,passable=true,fill=0.95),
    def!(BiomeKey::Swamp,        temp=0.7,hum=0.9,
        colors=("#1a3a20","#102018","#162818"),
        fx=AmbientFx::Motes{color:"rgba(60,120,30,.2)",count:5,speed:0.2},
        risk=2,hostile=0.9,strat=1.1,loot=1.3,mut_cost=4.0,passable=true,fill=0.6),
    def!(BiomeKey::Volcanic,     temp=1.0,hum=0.0,
        colors=("#2a0800","#1a0400","#200600"),
        fx=AmbientFx::ColorTint{color:"rgba(40,5,0,.15)",alpha:0.15},
        risk=3,hostile=1.2,strat=2.2,loot=1.8,mut_cost=6.5,passable=true,fill=0.7),
];

static GLOBAL_REGISTRY: BiomeRegistry = BiomeRegistry { defs: BUILT_IN_DEFS };

// ─── Colors helper ────────────────────────────────────────────────────────────

/// CSS hex tile colors per biome, index == BiomeKey discriminant.
/// Mirrors the JS `BC` constant in `game.html`.
pub const BIOME_COLORS: &[(&str, &str, &str)] = &[
    ("#0c3080","#082058","#0a2870"), // deep_water
    ("#1860c8","#0e3888","#1450a8"), // water
    ("#c8a868","#906030","#a87840"), // sand
    ("#487840","#285828","#387030"), // grass
    ("#1e4820","#0e2c10","#182e18"), // dark_forest
    ("#3a0810","#200408","#2a0408"), // crimson_forest
    ("#686070","#484058","#585068"), // rock
    ("#9898a8","#606070","#787888"), // mountain
    ("#d8d8f0","#9898b8","#b8b8d8"), // snow
    ("#181828","#0e0e1a","#141422"), // dungeon
    ("#80705a","#504438","#686050"), // village
    ("#a08870","#605040","#786050"), // building
    ("#1a3a20","#102018","#162818"), // swamp
    ("#2a0800","#1a0400","#200600"), // volcanic
];

// ─── Legacy helpers ───────────────────────────────────────────────────────────

/// Migrate a legacy biome string to the canonical `BiomeKey`.
///
/// This is a thin wrapper around [`BiomeKey::from_str`] kept for call sites
/// that previously used the `canonicalize_biome_id(&str) -> &str` API.
pub fn canonicalize_biome_id(legacy: &str) -> &'static str {
    BiomeKey::from_str(legacy).as_str()
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_keys_have_definitions() {
        let reg = BiomeRegistry::global();
        for key in BiomeKey::ALL {
            let def = reg.get(*key);
            assert_eq!(def.id, *key, "definition id mismatch for {key}");
        }
    }

    #[test]
    fn biome_ids_count() {
        assert_eq!(BIOME_IDS.len(), 14);
        assert_eq!(BiomeKey::ALL.len(), 14);
        assert_eq!(BIOME_COLORS.len(), 14);
        assert_eq!(TILE_PALETTE.len(), 14);
        assert_eq!(BUILT_IN_DEFS.len(), 14);
    }

    #[test]
    fn round_trip_as_str_from_str() {
        for &key in BiomeKey::ALL {
            assert_eq!(BiomeKey::from_str(key.as_str()), key,
                "round-trip failed for {key}");
        }
    }

    #[test]
    fn legacy_aliases_map_correctly() {
        assert_eq!(BiomeKey::from_str("GR"),         BiomeKey::Grass);
        assert_eq!(BiomeKey::from_str("CF"),         BiomeKey::CrimsonForest);
        assert_eq!(BiomeKey::from_str("DG"),         BiomeKey::Dungeon);
        assert_eq!(BiomeKey::from_str("forest"),     BiomeKey::DarkForest);
        assert_eq!(BiomeKey::from_str("desert"),     BiomeKey::Sand);
        assert_eq!(BiomeKey::from_str("safe-city"),  BiomeKey::Village);
        assert_eq!(BiomeKey::from_str("stone_floor"),BiomeKey::Rock);
    }

    #[test]
    fn unknown_input_defaults_to_grass() {
        assert_eq!(BiomeKey::from_str("unknown_biome_xyz_999"), BiomeKey::Grass);
    }

    #[test]
    fn canonicalize_biome_id_compat() {
        assert_eq!(canonicalize_biome_id("plains"),  "grass");
        assert_eq!(canonicalize_biome_id("dungeon"), "dungeon");
        assert_eq!(canonicalize_biome_id("forest"),  "dark_forest");
    }

    #[test]
    fn passable_biomes_no_water() {
        let reg = BiomeRegistry::global();
        let passable: Vec<BiomeKey> = reg.passable_ids().collect();
        assert!(!passable.contains(&BiomeKey::DeepWater));
        assert!(!passable.contains(&BiomeKey::Water));
        assert!(passable.contains(&BiomeKey::Grass));
        assert!(passable.contains(&BiomeKey::Dungeon));
    }

    #[test]
    fn dungeon_is_highest_risk() {
        let reg = BiomeRegistry::global();
        let dungeon = reg.get(BiomeKey::Dungeon);
        assert_eq!(dungeon.risk_tier, 3);
        assert!(dungeon.loot_weight_multiplier >= 2.0);
    }

    #[test]
    fn village_is_safe() {
        let reg = BiomeRegistry::global();
        let village = reg.get(BiomeKey::Village);
        assert_eq!(village.risk_tier, 0);
        assert_eq!(village.hostile_density, 0.0);
    }

    #[test]
    fn display_matches_as_str() {
        assert_eq!(BiomeKey::CrimsonForest.to_string(), "crimson_forest");
        assert_eq!(BiomeKey::Dungeon.to_string(),       "dungeon");
    }

    #[test]
    fn serde_round_trip() {
        let key = BiomeKey::Volcanic;
        let json = serde_json::to_string(&key).unwrap();
        let back: BiomeKey = serde_json::from_str(&json).unwrap();
        assert_eq!(key, back);
    }
}
