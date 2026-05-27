//! [`Zone`] — spatial region with ownership and state.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type ZoneId    = String;
pub type FactionId = String;

// ─── ResourceMap ─────────────────────────────────────────────────────────────

/// Resource amounts present in a zone.
pub type ResourceMap = BTreeMap<String, f32>;

// ─── ZoneState ───────────────────────────────────────────────────────────────

/// Ownership state of a [`Zone`].
///
/// Zones progress through states based on faction influence accumulation,
/// infrastructure buildup, combat resolution, and economic pressure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ZoneState {
    /// No faction can capture or destroy here (Safe City + surroundings).
    Safe,
    /// Two or more factions are competing for control.
    Contested {
        /// Leading faction (if any).
        leader:           Option<FactionId>,
        /// How close the leader is to capturing (0.0–1.0).
        contest_strength: f32,
    },
    /// Fully controlled by one faction.
    Controlled { faction_id: FactionId },
    /// Control is breaking down due to sustained pressure or neglect.
    ///
    /// If not stabilised within N ticks, the zone becomes Contested.
    Collapsing {
        /// Faction that held control before collapse began.
        former_owner: FactionId,
        /// Ticks remaining before the zone becomes Contested.
        ticks_remaining: u32,
    },
}

impl ZoneState {
    pub fn is_safe(&self) -> bool { matches!(self, ZoneState::Safe) }
    pub fn controller(&self) -> Option<&FactionId> {
        if let ZoneState::Controlled { faction_id } = self { Some(faction_id) } else { None }
    }
}

// ─── Zone ────────────────────────────────────────────────────────────────────

/// A spatial zone in the game world.
///
/// Zones are the unit of territorial control.  Both human players and the
/// Synthesis AI compete to control outer and deep zones; Safe City zones
/// are permanently protected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Zone {
    pub id:    ZoneId,
    pub state: ZoneState,

    /// Biome type governing terrain + loot generation.
    pub biome_id: String,

    /// Current resources available for extraction.
    pub resources: ResourceMap,

    /// Influence score per faction (0.0–1.0).
    ///
    /// When a faction's influence reaches 1.0 in a Contested zone, the
    /// zone transitions to Controlled.
    pub influence: BTreeMap<FactionId, f32>,

    /// Current risk tier: 0 = safe city, 1 = outer, 2 = deep.
    pub risk_tier: u8,
}

impl Zone {
    /// Create a new zone in Safe state (for Safe City zones).
    pub fn safe(id: impl Into<ZoneId>, biome_id: impl Into<String>) -> Self {
        Self {
            id:       id.into(),
            state:    ZoneState::Safe,
            biome_id: biome_id.into(),
            resources: BTreeMap::new(),
            influence: BTreeMap::new(),
            risk_tier: 0,
        }
    }

    /// Create a contested outer zone.
    pub fn contested(id: impl Into<ZoneId>, biome_id: impl Into<String>, risk_tier: u8) -> Self {
        Self {
            id:       id.into(),
            state:    ZoneState::Contested { leader: None, contest_strength: 0.0 },
            biome_id: biome_id.into(),
            resources: BTreeMap::new(),
            influence: BTreeMap::new(),
            risk_tier,
        }
    }

    /// Apply a faction influence delta this tick.
    ///
    /// If influence reaches 1.0, the zone transitions to Controlled.
    pub fn apply_influence(&mut self, faction_id: &str, delta: f32) {
        let score = self.influence.entry(faction_id.to_string()).or_insert(0.0);
        *score = (*score + delta).clamp(0.0, 1.0);

        // Decay other factions' influence when one faction pushes.
        let factions: Vec<_> = self.influence.keys().cloned().collect();
        for other in factions {
            if other != faction_id {
                if let Some(v) = self.influence.get_mut(&other) { *v *= 0.98; }
            }
        }

        // Transition to Controlled if influence hits cap.
        if *self.influence.get(faction_id).unwrap_or(&0.0) >= 1.0 {
            self.state = ZoneState::Controlled { faction_id: faction_id.to_string() };
        } else if matches!(self.state, ZoneState::Safe) {
            // Safe zones cannot be captured.
        } else {
            self.state = ZoneState::Contested {
                leader: Some(faction_id.to_string()),
                contest_strength: *self.influence.get(faction_id).unwrap_or(&0.0),
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_zone_cannot_be_contested() {
        let mut z = Zone::safe("hub", "village");
        z.apply_influence("synthesis", 0.5);
        // State remains Safe — Safe zones are immune.
        assert!(z.state.is_safe());
    }

    #[test]
    fn outer_zone_transitions_to_controlled() {
        let mut z = Zone::contested("outer-1", "forest", 1);
        for _ in 0..100 {
            z.apply_influence("humans", 0.02);
        }
        assert_eq!(z.state.controller(), Some(&"humans".to_string()));
    }

    #[test]
    fn zone_state_serialises() {
        let z = Zone::contested("z1", "dungeon", 2);
        let j = serde_json::to_string(&z.state).unwrap();
        assert!(j.contains("contested"));
    }
}
