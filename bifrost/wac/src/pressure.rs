//! Pressure graph — global and per-zone state signals used by the World Director.
//!
//! The pressure graph aggregates metrics from the live world and produces
//! scalar "pressure" values that the director uses to decide when and how to
//! mutate biomes, loot economy, and narrative.

use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};

// ─── Zone pressure ────────────────────────────────────────────────────────────

/// Aggregated signals for one world zone over the last N ticks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ZonePressure {
    pub zone_id: String,

    /// Average number of active players in this zone.
    pub player_density: f32,

    /// Kill events per tick (combat pressure).
    pub kill_rate: f32,

    /// Gold/XP generated per tick (economy flow).
    pub loot_flow: f32,

    /// Quest completions per tick in this zone.
    pub quest_rate: f32,

    /// Fraction of time the zone is "contested" (witness conflicts).
    pub contention: f32,
}

impl ZonePressure {
    /// Scalar combat pressure: how over-farmed this zone is.
    ///
    /// > 0.8 → director should reduce monster density or increase difficulty.
    /// < 0.2 → director should add events to revitalise the zone.
    pub fn combat_pressure(&self) -> f32 {
        // Normalise kill rate against player density to avoid scale bias.
        if self.player_density < f32::EPSILON { return 0.0; }
        (self.kill_rate / self.player_density).min(1.0)
    }

    /// Scalar economy pressure: how much loot is flowing in this zone.
    pub fn economy_pressure(&self) -> f32 {
        // Normalise loot flow per player.
        if self.player_density < f32::EPSILON { return 0.0; }
        (self.loot_flow / self.player_density / 50.0).min(1.0)  // 50 gold/tick per player = max
    }
}

// ─── Global pressure ─────────────────────────────────────────────────────────

/// World-wide aggregated signals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GlobalPressure {
    /// Total active players across all zones.
    pub total_players: u32,

    /// Gold/tick above the calibrated baseline.
    ///
    /// Positive = inflation risk; negative = deflation / too few drops.
    pub economy_delta: f32,

    /// Trending active player count:
    ///   > 0 = growing (good), < 0 = shrinking (intervention needed).
    pub player_trend: f32,

    /// Story beats fired per session across all arcs.
    pub narrative_momentum: f32,

    /// Quests completed per session world-wide.
    pub quest_throughput: f32,
}

impl GlobalPressure {
    /// True if the economy is inflating beyond a safe threshold.
    pub fn is_inflating(&self) -> bool { self.economy_delta > 0.2 }

    /// True if the economy is deflating (players not earning enough).
    pub fn is_deflating(&self) -> bool { self.economy_delta < -0.2 }

    /// True if narrative progression has stalled.
    pub fn narrative_stalled(&self) -> bool { self.narrative_momentum < 0.1 }
}

// ─── Pressure graph ───────────────────────────────────────────────────────────

/// The full pressure graph: per-zone + global signals.
///
/// Produced by the simulation layer each tick and fed to the [`WorldDirector`].
///
/// [`WorldDirector`]: crate::director::WorldDirector
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PressureGraph {
    /// Per-zone pressure, keyed by zone_id.
    pub zones:  BTreeMap<String, ZonePressure>,
    /// World-wide aggregate.
    pub global: GlobalPressure,
    /// Simulation tick this graph was captured at.
    pub at_tick: u64,
}

impl PressureGraph {
    pub fn new(at_tick: u64) -> Self {
        Self { at_tick, ..Default::default() }
    }

    pub fn insert_zone(&mut self, z: ZonePressure) {
        self.zones.insert(z.zone_id.clone(), z);
    }

    /// Find the zone with the highest combat pressure.
    pub fn hottest_zone(&self) -> Option<&ZonePressure> {
        self.zones.values().max_by(|a, b| {
            a.combat_pressure().partial_cmp(&b.combat_pressure()).unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// Find the zone with the lowest activity (candidate for a narrative event).
    pub fn coldest_zone(&self) -> Option<&ZonePressure> {
        self.zones.values().filter(|z| z.player_density > 0.0).min_by(|a, b| {
            a.combat_pressure().partial_cmp(&b.combat_pressure()).unwrap_or(std::cmp::Ordering::Equal)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combat_pressure_zero_when_no_players() {
        let z = ZonePressure::default();
        assert_eq!(z.combat_pressure(), 0.0);
    }

    #[test]
    fn combat_pressure_capped_at_one() {
        let z = ZonePressure { player_density: 1.0, kill_rate: 100.0, ..Default::default() };
        assert!(z.combat_pressure() <= 1.0);
    }

    #[test]
    fn inflation_detection() {
        let mut g = GlobalPressure::default();
        g.economy_delta = 0.3;
        assert!(g.is_inflating());
        assert!(!g.is_deflating());
    }

    #[test]
    fn hottest_zone() {
        let mut pg = PressureGraph::new(0);
        pg.insert_zone(ZonePressure { zone_id: "forest".into(),   player_density: 5.0, kill_rate: 2.0, ..Default::default() });
        pg.insert_zone(ZonePressure { zone_id: "dungeon".into(),  player_density: 2.0, kill_rate: 8.0, ..Default::default() });
        let hot = pg.hottest_zone().unwrap();
        assert_eq!(hot.zone_id, "dungeon");
    }
}
