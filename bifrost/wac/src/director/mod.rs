//! World Director — reads the pressure graph and emits [`AssetBlueprint`]s.
//!
//! The director is the **only** thing allowed to initiate world mutations at
//! the macro level.  It never mutates state directly — it emits blueprints
//! that flow through the WAC pipeline (validate → compile → runtime).
//!
//! ## Three policies
//!
//! | Policy | Trigger | Blueprint type |
//! |---|---|---|
//! | Biome evolution | zone combat pressure > threshold | `BiomeDefinition` |
//! | Loot economy    | global inflation / deflation     | `LootTable` |
//! | Narrative event | story momentum stalled           | `EntityPrefab` |
//!
//! Each policy has an independent cooldown to prevent blueprint storms.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::pressure::{GlobalPressure, PressureGraph, ZonePressure};
use crate::types::{AssetBlueprint, AssetIntent};

// ─── Config ───────────────────────────────────────────────────────────────────

/// Tuning knobs for the World Director.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DirectorConfig {
    /// Combat pressure above which a biome evolution blueprint is emitted.
    pub biome_evolution_threshold: f32,

    /// Economy delta magnitude above which a loot table blueprint is emitted.
    pub economy_adjustment_threshold: f32,

    /// Narrative momentum below which a narrative event blueprint is emitted.
    pub narrative_stall_threshold: f32,

    /// Minimum ticks between biome evolution blueprints per zone.
    pub biome_cooldown_ticks: u64,

    /// Minimum ticks between loot economy adjustments.
    pub economy_cooldown_ticks: u64,

    /// Minimum ticks between narrative event triggers.
    pub narrative_cooldown_ticks: u64,

    /// Maximum [`AssetBlueprint`]s emitted in a single tick.
    pub max_blueprints_per_tick: usize,
}

impl Default for DirectorConfig {
    fn default() -> Self {
        Self {
            biome_evolution_threshold:       0.75,
            economy_adjustment_threshold:    0.20,
            narrative_stall_threshold:       0.10,
            biome_cooldown_ticks:            500,
            economy_cooldown_ticks:          200,
            narrative_cooldown_ticks:        300,
            max_blueprints_per_tick:         3,
        }
    }
}

// ─── State ────────────────────────────────────────────────────────────────────

/// Mutable runtime state tracked by the director.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DirectorState {
    /// Last tick a biome evolution blueprint was emitted per zone.
    pub last_biome_mutation: BTreeMap<String, u64>,
    /// Last tick a loot economy blueprint was emitted.
    pub last_economy_adjustment: u64,
    /// Last tick a narrative event blueprint was emitted.
    pub last_narrative_event: u64,
    /// Total blueprints emitted lifetime.
    pub total_blueprints_emitted: u64,
}

// ─── Decision record ─────────────────────────────────────────────────────────

/// Records why a blueprint was emitted — useful for audit / debug.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectorDecision {
    pub blueprint: AssetBlueprint,
    pub reason:    DirectorReason,
    pub at_tick:   u64,
}

/// The policy that triggered a director decision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectorReason {
    /// A zone was over-farmed; biome evolves to restore challenge.
    BiomeEvolution {
        zone_id:          String,
        combat_pressure:  f32,
    },
    /// Global economy is inflating or deflating.
    LootEconomyAdjustment {
        economy_delta:  f32,
        direction:      EconomyDirection,
    },
    /// Story narrative has stalled; a new event is triggered.
    NarrativeEvent {
        narrative_momentum: f32,
        target_zone:        String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EconomyDirection { Inflate, Deflate }

// ─── World Director ───────────────────────────────────────────────────────────

/// The macro-level world AI.
///
/// Call [`WorldDirector::tick`] once per game tick with the current
/// [`PressureGraph`].  It returns a list of [`DirectorDecision`]s, each
/// containing a blueprint ready for the WAC pipeline.
#[derive(Debug, Clone)]
pub struct WorldDirector {
    pub config: DirectorConfig,
    pub state:  DirectorState,
    /// History of the last N decisions (capped at 200).
    pub history: Vec<DirectorDecision>,
}

impl Default for WorldDirector {
    fn default() -> Self { Self::new(DirectorConfig::default()) }
}

impl WorldDirector {
    pub fn new(config: DirectorConfig) -> Self {
        Self { config, state: DirectorState::default(), history: Vec::new() }
    }

    // ── Main tick ────────────────────────────────────────────────────────────

    /// Evaluate all policies against the pressure graph and emit blueprints.
    ///
    /// Returns `Vec<DirectorDecision>` capped at `config.max_blueprints_per_tick`.
    pub fn tick(&mut self, pressure: &PressureGraph) -> Vec<DirectorDecision> {
        let tick = pressure.at_tick;
        let mut decisions: Vec<DirectorDecision> = Vec::new();

        // ── Policy 1: Biome evolution (per zone) ─────────────────────────────
        for zone in pressure.zones.values() {
            if decisions.len() >= self.config.max_blueprints_per_tick { break; }
            if let Some(d) = self.evaluate_biome_evolution(zone, tick) {
                decisions.push(d);
            }
        }

        // ── Policy 2: Loot economy ────────────────────────────────────────────
        if decisions.len() < self.config.max_blueprints_per_tick {
            if let Some(d) = self.evaluate_economy(&pressure.global, tick) {
                decisions.push(d);
            }
        }

        // ── Policy 3: Narrative event ─────────────────────────────────────────
        if decisions.len() < self.config.max_blueprints_per_tick {
            if let Some(d) = self.evaluate_narrative(&pressure.global, pressure, tick) {
                decisions.push(d);
            }
        }

        // Record decisions
        self.state.total_blueprints_emitted += decisions.len() as u64;
        for d in &decisions {
            if self.history.len() >= 200 { self.history.remove(0); }
            self.history.push(d.clone());
        }

        decisions
    }

    // ── Policy 1: Biome evolution ─────────────────────────────────────────────

    fn evaluate_biome_evolution(&mut self, zone: &ZonePressure, tick: u64) -> Option<DirectorDecision> {
        let pressure = zone.combat_pressure();
        if pressure < self.config.biome_evolution_threshold { return None; }

        // Cooldown check
        let last = self.state.last_biome_mutation.get(&zone.zone_id).copied().unwrap_or(0);
        if tick.saturating_sub(last) < self.config.biome_cooldown_ticks { return None; }

        // Seed is deterministic: zone_id hash XOR tick.
        let seed = zone_seed(&zone.zone_id, tick);

        // Spec describes how the biome should evolve based on pressure tier.
        let spec = biome_evolution_spec(pressure, &zone.zone_id);
        let constraints = vec![
            "no floating voxels".into(),
            "navmesh must remain connected".into(),
        ];

        let bp = AssetBlueprint::new(AssetIntent::BiomeDefinition, spec, constraints, seed);

        self.state.last_biome_mutation.insert(zone.zone_id.clone(), tick);

        Some(DirectorDecision {
            reason: DirectorReason::BiomeEvolution {
                zone_id:         zone.zone_id.clone(),
                combat_pressure: pressure,
            },
            at_tick: tick,
            blueprint: bp,
        })
    }

    // ── Policy 2: Loot economy ────────────────────────────────────────────────

    fn evaluate_economy(&mut self, global: &GlobalPressure, tick: u64) -> Option<DirectorDecision> {
        if !global.is_inflating() && !global.is_deflating() { return None; }

        let since = tick.saturating_sub(self.state.last_economy_adjustment);
        if since < self.config.economy_cooldown_ticks { return None; }

        let (direction, spec) = if global.is_inflating() {
            (EconomyDirection::Inflate,
             format!("reduce loot rates globally by {:.0}% due to economic inflation",
                     global.economy_delta * 100.0))
        } else {
            (EconomyDirection::Deflate,
             format!("increase loot rates globally by {:.0}% due to economic deflation",
                     global.economy_delta.abs() * 100.0))
        };

        let constraints = vec![
            "max_drop_rate <= 0.50".into(),
            "min_drop_rate >= 0.01".into(),
        ];
        let seed = (tick ^ (global.economy_delta.to_bits() as u64)).max(1);
        let bp   = AssetBlueprint::new(AssetIntent::LootTable, spec, constraints, seed);

        self.state.last_economy_adjustment = tick;

        Some(DirectorDecision {
            reason: DirectorReason::LootEconomyAdjustment {
                economy_delta: global.economy_delta,
                direction,
            },
            at_tick: tick,
            blueprint: bp,
        })
    }

    // ── Policy 3: Narrative event ─────────────────────────────────────────────

    fn evaluate_narrative(
        &mut self,
        global: &GlobalPressure,
        pressure: &PressureGraph,
        tick: u64,
    ) -> Option<DirectorDecision> {
        if !global.narrative_stalled() { return None; }

        let since = tick.saturating_sub(self.state.last_narrative_event);
        if since < self.config.narrative_cooldown_ticks { return None; }

        // Target the coldest zone (least player activity — needs a spark).
        let target_zone = pressure.coldest_zone()
            .map(|z| z.zone_id.clone())
            .unwrap_or_else(|| "village".into());

        // Spawn a named antagonist entity to catalyse new activity.
        let spec = format!(
            "hostile boss entity with high hp atk in {target_zone} zone to revive player activity"
        );
        let seed = (tick ^ 0xDEAD_BEEF).max(1);
        let bp   = AssetBlueprint::new(AssetIntent::EntityPrefab, spec, vec![], seed);

        self.state.last_narrative_event = tick;

        Some(DirectorDecision {
            reason: DirectorReason::NarrativeEvent {
                narrative_momentum: global.narrative_momentum,
                target_zone: target_zone.clone(),
            },
            at_tick: tick,
            blueprint: bp,
        })
    }

    // ── Queries ───────────────────────────────────────────────────────────────

    /// Last N decisions (for the admin monitor panel).
    pub fn recent_decisions(&self, n: usize) -> &[DirectorDecision] {
        let start = self.history.len().saturating_sub(n);
        &self.history[start..]
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Deterministic zone seed from zone id and tick.
fn zone_seed(zone_id: &str, tick: u64) -> u64 {
    let mut h = blake3::Hasher::new();
    h.update(zone_id.as_bytes());
    h.update(&tick.to_le_bytes());
    let bytes = h.finalize();
    u64::from_le_bytes(bytes.as_bytes()[..8].try_into().unwrap()).max(1)
}

/// Generate a biome evolution spec based on the pressure tier.
fn biome_evolution_spec(pressure: f32, zone_id: &str) -> String {
    let tier = if pressure > 0.95 { "extreme volcanic hostile" }
               else if pressure > 0.85 { "dark aggressive hostile" }
               else { "dense hostile forest" };
    format!("{tier} biome evolution for {zone_id} zone with stronger monsters and scarce resources")
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pressure::{GlobalPressure, PressureGraph, ZonePressure};

    fn over_farmed_pressure() -> PressureGraph {
        let mut pg = PressureGraph::new(1000);
        pg.insert_zone(ZonePressure {
            zone_id:        "forest".into(),
            player_density: 5.0,
            kill_rate:      10.0, // pressure = 10/5 = 2.0 → clamped to 1.0
            ..Default::default()
        });
        pg.global = GlobalPressure {
            economy_delta:       0.0,
            narrative_momentum:  0.5,
            total_players:       5,
            ..Default::default()
        };
        pg
    }

    fn stale_narrative_pressure() -> PressureGraph {
        let mut pg = PressureGraph::new(1000);
        pg.insert_zone(ZonePressure {
            zone_id:        "dungeon".into(),
            player_density: 1.0,
            kill_rate:      0.1,
            ..Default::default()
        });
        pg.global = GlobalPressure {
            narrative_momentum: 0.02, // stalled
            economy_delta:      0.0,
            total_players:      3,
            ..Default::default()
        };
        pg
    }

    #[test]
    fn biome_evolution_emitted_when_over_farmed() {
        let mut director = WorldDirector::new(DirectorConfig {
            biome_evolution_threshold: 0.75,
            ..Default::default()
        });
        let decisions = director.tick(&over_farmed_pressure());
        assert!(!decisions.is_empty());
        assert!(matches!(decisions[0].reason, DirectorReason::BiomeEvolution { .. }));
    }

    #[test]
    fn biome_cooldown_prevents_spam() {
        let mut director = WorldDirector::new(DirectorConfig {
            biome_evolution_threshold: 0.75,
            biome_cooldown_ticks: 500,
            ..Default::default()
        });
        let d1 = director.tick(&over_farmed_pressure());
        // Immediately tick again — should be suppressed by cooldown
        let d2 = director.tick(&over_farmed_pressure());
        assert!(!d1.is_empty());
        assert!(!d2.iter().any(|d| matches!(d.reason, DirectorReason::BiomeEvolution { .. })));
    }

    #[test]
    fn economy_inflation_emitted() {
        let mut director = WorldDirector::default();
        let mut pg = PressureGraph::new(1000);
        pg.global = GlobalPressure {
            economy_delta:      0.35, // inflating
            narrative_momentum: 0.5,
            total_players:      5,
            ..Default::default()
        };
        let decisions = director.tick(&pg);
        assert!(decisions.iter().any(|d| matches!(
            d.reason,
            DirectorReason::LootEconomyAdjustment { direction: EconomyDirection::Inflate, .. }
        )));
    }

    #[test]
    fn narrative_event_emitted_when_stalled() {
        let mut director = WorldDirector::default();
        let decisions = director.tick(&stale_narrative_pressure());
        assert!(decisions.iter().any(|d| matches!(d.reason, DirectorReason::NarrativeEvent { .. })));
    }

    #[test]
    fn narrative_event_targets_coldest_zone() {
        let mut director = WorldDirector::default();
        let pg = stale_narrative_pressure();
        let decisions = director.tick(&pg);
        let narrative = decisions.iter()
            .find(|d| matches!(d.reason, DirectorReason::NarrativeEvent { .. }));
        assert!(narrative.is_some());
        if let DirectorReason::NarrativeEvent { target_zone, .. } = &narrative.unwrap().reason {
            assert_eq!(target_zone, "dungeon");
        }
    }

    #[test]
    fn max_blueprints_per_tick_enforced() {
        let config = DirectorConfig {
            biome_evolution_threshold:    0.0,   // always trigger biome
            economy_adjustment_threshold: 0.0,   // always trigger economy
            narrative_stall_threshold:    1.0,   // always trigger narrative
            max_blueprints_per_tick:      2,
            biome_cooldown_ticks:         0,
            economy_cooldown_ticks:       0,
            narrative_cooldown_ticks:     0,
        };
        let mut director = WorldDirector::new(config);
        // Add many zones to stress-test the cap
        let mut pg = PressureGraph::new(1);
        for i in 0..5 {
            pg.insert_zone(ZonePressure {
                zone_id:        format!("zone-{i}"),
                player_density: 5.0, kill_rate: 100.0,
                ..Default::default()
            });
        }
        pg.global = GlobalPressure { economy_delta: 0.5, narrative_momentum: 0.0, ..Default::default() };
        let decisions = director.tick(&pg);
        assert!(decisions.len() <= 2);
    }

    #[test]
    fn history_recorded() {
        let mut director = WorldDirector::default();
        director.tick(&over_farmed_pressure());
        assert!(!director.history.is_empty());
    }

    #[test]
    fn blueprint_seeds_are_nonzero() {
        let mut director = WorldDirector::default();
        let decisions = director.tick(&over_farmed_pressure());
        for d in &decisions {
            assert!(d.blueprint.seed > 0, "seed must not be zero");
        }
    }
}
