//! [`PlayerBase`] — WAC-asset-backed player-built structures.
//!
//! Players don't place prefabs.  They inject WAC blueprints whose compiled
//! output (tile maps, biome modifiers) become real world structures.
//!
//! > "Base building ist nicht 'placement'.  Es ist: Rule injection into world physics."
//! > — `docs/WORLD.md`

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::zone::ZoneId;

// ─── WacAssetRef ─────────────────────────────────────────────────────────────

/// A reference to a compiled WAC asset injected into the world.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WacAssetRef {
    /// Blueprint ID (from `bifrost-wac`).
    pub blueprint_id: Uuid,
    /// Compiled asset type tag.
    pub asset_type:   String,
    /// World position (tile coordinates) of the injection point.
    pub origin_x:     u32,
    pub origin_y:     u32,
}

// ─── PlayerBase ──────────────────────────────────────────────────────────────

/// A player-owned base: a cluster of WAC-compiled structures in a zone.
///
/// BUILD FLOW:
/// ```text
/// Player Intent
///    ↓
/// WAC Blueprint
///    ↓
/// Validation (IVL)
///    ↓
/// TileMap / Entity / Loot compilation
///    ↓
/// World injection (stored here)
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerBase {
    pub id:       Uuid,
    pub owner_id: String,
    pub zone_id:  ZoneId,

    /// Compiled WAC assets that make up this base.
    pub structures: Vec<WacAssetRef>,

    /// Active biome modification rules injected by this base.
    ///
    /// E.g. a player with high Terrain skill can modify humidity around
    /// their base to deny resources to the Synthesis faction.
    pub biome_modifiers: Vec<BiomeModifier>,

    /// Defense matrix: influence on zone control score each tick.
    pub defense_matrix: DefenseMatrix,
}

impl PlayerBase {
    pub fn new(owner_id: impl Into<String>, zone_id: impl Into<ZoneId>) -> Self {
        Self {
            id:              Uuid::new_v4(),
            owner_id:        owner_id.into(),
            zone_id:         zone_id.into(),
            structures:      vec![],
            biome_modifiers: vec![],
            defense_matrix:  DefenseMatrix::default(),
        }
    }

    /// Add a compiled WAC asset to this base.
    pub fn add_structure(&mut self, asset: WacAssetRef) {
        self.structures.push(asset);
    }

    /// Total influence contributed by this base per tick.
    pub fn influence_per_tick(&self) -> f32 {
        self.defense_matrix.base_influence_rate
            + self.structures.len() as f32 * 0.01
    }
}

// ─── BiomeModifier ───────────────────────────────────────────────────────────

/// A biome rule modification injected by a player base.
///
/// High-level Terrain skill unlocks allow players to modify world physics
/// around their base.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeModifier {
    pub id:               Uuid,
    /// Which biome parameter is affected.
    pub parameter:        BiomeParameter,
    /// Delta to apply each tick.
    pub delta_per_tick:   f32,
    /// Maximum total delta achievable.
    pub max_delta:        f32,
    /// Radius in tiles around the base origin.
    pub radius:           u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BiomeParameter {
    Temperature,
    Humidity,
    TreeDensity,
    ResourceDensity,
}

// ─── DefenseMatrix ───────────────────────────────────────────────────────────

/// Defensive attributes of a player base.
///
/// A stronger defense matrix slows faction influence accumulation by enemies
/// and provides passive zone influence each tick.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefenseMatrix {
    /// Passive faction influence added to the zone per tick.
    pub base_influence_rate: f32,
    /// Multiplier applied to enemy influence decay speed.
    pub enemy_decay_mult:    f32,
    /// HP of the base's core structure (0 = destroyed).
    pub hp:                  u32,
    pub max_hp:              u32,
}

impl Default for DefenseMatrix {
    fn default() -> Self {
        Self {
            base_influence_rate: 0.02,
            enemy_decay_mult:    1.2,
            hp:                  100,
            max_hp:              100,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_influence_grows_with_structures() {
        let mut base = PlayerBase::new("player-1", "zone-A3");
        let base_rate = base.influence_per_tick();
        base.add_structure(WacAssetRef {
            blueprint_id: Uuid::new_v4(),
            asset_type:   "tile_map".into(),
            origin_x: 5, origin_y: 5,
        });
        assert!(base.influence_per_tick() > base_rate);
    }
}
