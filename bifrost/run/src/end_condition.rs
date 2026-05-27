//! [`EndCondition`] — win condition variants and evaluator.

use serde::{Deserialize, Serialize};

// ─── EndCondition ─────────────────────────────────────────────────────────────

/// Win condition that ends a [`crate::run::WorldRun`].
///
/// Multiple victory paths prevent stale "best build" metas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EndCondition {
    /// First faction to control N zones wins.
    FirstToControlZones { required_zones: u32 },

    /// First faction to reach a technology level wins.
    FirstToReachTechLevel { level: u32 },

    /// First faction to accumulate `threshold` fraction of total economy wins.
    EconomicDominance { threshold: f32 },

    /// All factions survive until this world tick — then evaluated on score.
    SurvivalUntilTick(u64),
}

// ─── Snapshot ────────────────────────────────────────────────────────────────

/// World snapshot used to evaluate end conditions.
///
/// Populated by the BIFROST world state reader each tick.
#[derive(Debug, Default)]
pub struct WorldSnapshot {
    pub current_tick:      u64,
    /// Map of faction_id → number of controlled zones.
    pub zones_controlled:  std::collections::BTreeMap<String, u32>,
    /// Map of faction_id → current tech level.
    pub tech_levels:       std::collections::BTreeMap<String, u32>,
    /// Map of faction_id → fraction of total economy (0.0–1.0).
    pub economy_fractions: std::collections::BTreeMap<String, f32>,
}

// ─── Evaluator ───────────────────────────────────────────────────────────────

/// Result of evaluating an [`EndCondition`] against a [`WorldSnapshot`].
#[derive(Debug, PartialEq)]
pub enum EndConditionEval {
    /// Run continues normally.
    Continue,
    /// A faction has satisfied the win condition.
    ///
    /// Contains the winning faction ID.
    FactionWon(String),
    /// Time/tick limit reached — no clear winner, evaluate on secondary metrics.
    TimeExpired,
}

impl EndCondition {
    /// Evaluate this condition against a world snapshot.
    ///
    /// Called each tick by the [`crate::director::WorldRunDirector`].
    pub fn evaluate(&self, snap: &WorldSnapshot) -> EndConditionEval {
        match self {
            EndCondition::FirstToControlZones { required_zones } => {
                for (faction, &count) in &snap.zones_controlled {
                    if count >= *required_zones {
                        return EndConditionEval::FactionWon(faction.clone());
                    }
                }
                EndConditionEval::Continue
            }

            EndCondition::FirstToReachTechLevel { level } => {
                for (faction, &tech) in &snap.tech_levels {
                    if tech >= *level {
                        return EndConditionEval::FactionWon(faction.clone());
                    }
                }
                EndConditionEval::Continue
            }

            EndCondition::EconomicDominance { threshold } => {
                for (faction, &fraction) in &snap.economy_fractions {
                    if fraction >= *threshold {
                        return EndConditionEval::FactionWon(faction.clone());
                    }
                }
                EndConditionEval::Continue
            }

            EndCondition::SurvivalUntilTick(end_tick) => {
                if snap.current_tick >= *end_tick {
                    // Survival runs end by time; winner determined by score elsewhere.
                    return EndConditionEval::TimeExpired;
                }
                EndConditionEval::Continue
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn snap_with_zones(faction: &str, zones: u32) -> WorldSnapshot {
        let mut s = WorldSnapshot::default();
        s.zones_controlled.insert(faction.into(), zones);
        s
    }

    #[test]
    fn zone_control_win() {
        let cond = EndCondition::FirstToControlZones { required_zones: 5 };
        assert_eq!(cond.evaluate(&snap_with_zones("humans", 5)),
                   EndConditionEval::FactionWon("humans".into()));
    }

    #[test]
    fn zone_control_not_yet() {
        let cond = EndCondition::FirstToControlZones { required_zones: 5 };
        assert_eq!(cond.evaluate(&snap_with_zones("humans", 4)),
                   EndConditionEval::Continue);
    }

    #[test]
    fn survival_tick_expired() {
        let cond = EndCondition::SurvivalUntilTick(1000);
        let mut snap = WorldSnapshot::default();
        snap.current_tick = 1000;
        assert_eq!(cond.evaluate(&snap), EndConditionEval::TimeExpired);
    }

    #[test]
    fn economic_dominance_win() {
        let cond = EndCondition::EconomicDominance { threshold: 0.6 };
        let mut snap = WorldSnapshot::default();
        snap.economy_fractions.insert("synthesis".into(), 0.65);
        assert_eq!(cond.evaluate(&snap),
                   EndConditionEval::FactionWon("synthesis".into()));
    }
}
