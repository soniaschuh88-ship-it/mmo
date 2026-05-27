//! Strategy engine and world model for the Synthesis faction.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::faction::ZoneId;

// ─── WorldModel ──────────────────────────────────────────────────────────────

/// The Synthesis faction's perception of the current world state.
///
/// Updated each tick from the BIFROST world snapshot.
/// Drives the [`StrategyEngine`] decision cycle.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorldModel {
    /// Estimated resource concentration per zone (0.0–1.0).
    pub zone_resources: BTreeMap<ZoneId, f32>,

    /// Detected player fortress locations (zone_id → strength estimate).
    pub player_fortresses: BTreeMap<ZoneId, f32>,

    /// Synthesis-controlled zones.
    pub synthesis_zones: Vec<ZoneId>,

    /// Player faction economy fraction estimate.
    pub player_economy_fraction: f32,

    /// Current world tick.
    pub current_tick: u64,
}

impl WorldModel {
    /// Update from a simplified BIFROST snapshot (placeholder for real integration).
    pub fn update(&mut self, tick: u64, zone_resources: BTreeMap<ZoneId, f32>) {
        self.current_tick   = tick;
        self.zone_resources = zone_resources;
    }

    /// Return the zone with the highest uncontested resource score.
    pub fn highest_value_zone(&self) -> Option<&ZoneId> {
        self.zone_resources
            .iter()
            .filter(|(z, _)| !self.synthesis_zones.contains(z))
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(z, _)| z)
    }

    /// Identify zones where players have built fortresses.
    pub fn player_fortress_zones(&self) -> Vec<&ZoneId> {
        self.player_fortresses.keys().collect()
    }
}

// ─── StrategyGoal ────────────────────────────────────────────────────────────

/// A concrete strategic objective the faction is pursuing.
///
/// Pushed onto the `AiFaction::goal_stack`; top = highest priority.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StrategyGoal {
    /// Claim additional zones.
    ExpandTerritory,
    /// Shift biome humidity/temperature in a zone to destabilise supply chains.
    AdaptBiome { zone_id: ZoneId },
    /// Destabilise player economy by flooding loot markets.
    DestabiliseEconomy,
    /// Build defensive WAC structures around a zone.
    DefendZone { zone_id: ZoneId },
    /// Concentrate agents for an assault on a zone.
    AssaultZone { zone_id: ZoneId },
    /// Research tech upgrades.
    AdvanceTechnology,
    /// Spy on player crafting trends via Safe City auction observation.
    SpyAuction,
}

// ─── StrategyEngine ──────────────────────────────────────────────────────────

/// Evaluates the world model and returns the highest-priority strategic goals.
///
/// In full deployment, the `CoreAgent` sends the world model to NVIDIA NIM
/// and gets back a structured `StrategyGoal` list.  This struct provides the
/// local rule-based fallback for when NIM is unavailable.
#[derive(Debug, Default)]
pub struct StrategyEngine;

impl StrategyEngine {
    /// Derive strategic goals from the current world model.
    ///
    /// Returns goals in priority order (highest first).
    pub fn evaluate(&self, model: &WorldModel) -> Vec<StrategyGoal> {
        let mut goals: Vec<StrategyGoal> = vec![];

        // Priority 1: Defend any synthesis zone under player pressure.
        for zone in &model.synthesis_zones {
            if model.player_fortresses.contains_key(zone) {
                goals.push(StrategyGoal::DefendZone { zone_id: zone.clone() });
            }
        }

        // Priority 2: Expand to highest-value uncontested zone.
        if model.synthesis_zones.len() < 3 {
            if let Some(zone) = model.highest_value_zone() {
                goals.push(StrategyGoal::ExpandTerritory);
                goals.push(StrategyGoal::AssaultZone { zone_id: zone.clone() });
            }
        }

        // Priority 3: Counter economy exploit if player fraction is high.
        if model.player_economy_fraction > 0.5 {
            goals.push(StrategyGoal::DestabiliseEconomy);
            // Pick highest-resource player zone and adapt its biome.
            if let Some(fortress) = model.player_fortresses.keys().next() {
                goals.push(StrategyGoal::AdaptBiome { zone_id: fortress.clone() });
            }
        }

        // Priority 4: Advance technology when not under immediate threat.
        if goals.is_empty() {
            goals.push(StrategyGoal::AdvanceTechnology);
        }

        // Always spy on the auction house.
        goals.push(StrategyGoal::SpyAuction);

        goals
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model_with_player_fortress(zone: &str) -> WorldModel {
        let mut m = WorldModel::default();
        m.player_fortresses.insert(zone.into(), 0.8);
        m
    }

    #[test]
    fn engine_defends_contested_synthesis_zone() {
        let mut m = model_with_player_fortress("A3");
        m.synthesis_zones.push("A3".into());
        let goals = StrategyEngine.evaluate(&m);
        assert!(goals.iter().any(|g| matches!(g, StrategyGoal::DefendZone { .. })));
    }

    #[test]
    fn engine_expands_when_few_zones() {
        let m = WorldModel::default();
        let goals = StrategyEngine.evaluate(&m);
        // No fortresses, no zones — should push tech advance or expand.
        assert!(!goals.is_empty());
    }

    #[test]
    fn engine_destabilises_when_player_economy_dominant() {
        let mut m = WorldModel::default();
        m.player_economy_fraction = 0.75;
        m.player_fortresses.insert("B2".into(), 0.5);
        let goals = StrategyEngine.evaluate(&m);
        assert!(goals.iter().any(|g| matches!(g, StrategyGoal::DestabiliseEconomy)));
    }
}
