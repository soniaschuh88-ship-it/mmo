//! [`WorldRunDirector`] — orchestrates run lifecycle and world mutation.
//!
//! The Director is the central authority over the run loop:
//!
//! ```text
//! Run N ends
//!   ↓
//! WorldRunDirector::evaluate_tick()
//!   ↓ (if EndCondition met)
//! finalize_run()  →  generate_next_world_seed()
//!   ↓
//! Run N+1 begins with counter-adapted world
//! ```
//!
//! ## AI counter-world generation
//!
//! The Director analyses the previous run's dominant strategy and generates
//! a counter-adapted world seed for the next run:
//!
//! > "If players dominated via economy exploitation → next world: scarcity
//! >  biomes, unstable loot markets, roaming AI traders."

use serde::{Deserialize, Serialize};

use crate::end_condition::{EndConditionEval, WorldSnapshot};
use crate::meta::MetaProgression;
use crate::run::{FactionId, RunResult, RunState, WorldRun};

// ─── WorldRunDirector ────────────────────────────────────────────────────────

/// Orchestrates world run lifecycle and cross-run world mutation.
#[derive(Debug, Default)]
pub struct WorldRunDirector {
    /// All runs (active and historical).
    pub runs:          Vec<WorldRun>,

    /// Persistent meta-progression state per faction.
    pub meta:          std::collections::BTreeMap<FactionId, MetaProgression>,

    /// Previous run dominant strategy tag (used for counter-world generation).
    last_dominant_strategy: Option<DominantStrategy>,
}

impl WorldRunDirector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new run with the director.
    pub fn add_run(&mut self, run: WorldRun) {
        self.runs.push(run);
    }

    /// Returns the currently active run (if any).
    pub fn active_run(&self) -> Option<&WorldRun> {
        self.runs.iter().find(|r| r.is_active())
    }

    /// Returns a mutable reference to the currently active run.
    pub fn active_run_mut(&mut self) -> Option<&mut WorldRun> {
        self.runs.iter_mut().find(|r| r.is_active())
    }

    /// Start the next pending run.
    ///
    /// Should be called once the world has finished generating for the new epoch.
    pub fn start_next_run(&mut self, tick: u64) -> Option<&WorldRun> {
        let run = self.runs.iter_mut().find(|r| matches!(r.state, RunState::Pending))?;
        run.start(tick);
        let id = run.id;
        self.runs.iter().find(|r| r.id == id)
    }

    /// Evaluate the active run's end condition against the current world snapshot.
    ///
    /// If the condition is met, the run is finalized and meta-progression is
    /// updated.  Returns the `RunResult` if the run ended, `None` if it continues.
    pub fn evaluate_tick(&mut self, tick: u64, snap: &WorldSnapshot) -> Option<RunResult> {
        let eval = {
            let run = self.active_run()?;
            run.end_condition.evaluate(snap)
        };

        match eval {
            EndConditionEval::Continue => None,

            EndConditionEval::FactionWon(winner) => {
                let run = self.active_run()?;
                let losers: Vec<_> = run.player_factions
                    .iter()
                    .chain(&run.ai_factions)
                    .filter(|f| *f != &winner)
                    .cloned()
                    .collect();
                let condition = run.end_condition.clone();
                let result = RunResult::winner(winner, losers, condition);
                self.finalize_run(tick, result.clone());
                Some(result)
            }

            EndConditionEval::TimeExpired => {
                let run = self.active_run()?;
                let all: Vec<_> = run.player_factions
                    .iter()
                    .chain(&run.ai_factions)
                    .cloned()
                    .collect();
                let condition = run.end_condition.clone();
                let result = RunResult::draw(condition, all);
                self.finalize_run(tick, result.clone());
                Some(result)
            }
        }
    }

    /// Finalize a run: record result + update meta-progression.
    fn finalize_run(&mut self, tick: u64, result: RunResult) {
        if let Some(run) = self.active_run_mut() {
            run.end(tick, result.clone());
        }

        // Update meta-progression for all factions.
        if let Some(winner) = &result.winner {
            let meta = self.meta
                .entry(winner.clone())
                .or_insert_with(|| MetaProgression::new(winner));
            meta.apply_rewards(&result.winner_rewards);
        }
        for loser in &result.losers {
            let meta = self.meta
                .entry(loser.clone())
                .or_insert_with(|| MetaProgression::new(loser));
            meta.apply_penalties(&result.loser_penalties);
        }

        // Record dominant strategy for next world generation.
        self.last_dominant_strategy = Some(DominantStrategy::infer(&result));
    }

    /// Generate a world seed for the next run, counter-adapted to the previous run's meta.
    ///
    /// The Director analyses player/AI dominant strategies and biases the
    /// new world against them to prevent stale metas.
    pub fn generate_next_world_seed(&self, base_seed: u64) -> WorldSeedConfig {
        let strategy = self.last_dominant_strategy.as_ref();
        WorldSeedConfig::counter_adapted(base_seed, strategy)
    }
}

// ─── DominantStrategy ────────────────────────────────────────────────────────

/// Abstracted dominant strategy observed in the previous run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DominantStrategy {
    /// Players dominated via economic exploitation.
    EconomyExploit,
    /// Players dominated via zone rush (fast territorial capture).
    ZoneRush,
    /// AI dominated via biome adaptation pressure.
    BiomeAdaptation,
    /// No clear dominant strategy.
    Balanced,
}

impl DominantStrategy {
    fn infer(result: &RunResult) -> Self {
        use crate::end_condition::EndCondition;
        match &result.condition_triggered {
            EndCondition::EconomicDominance { .. } => DominantStrategy::EconomyExploit,
            EndCondition::FirstToControlZones { .. } => DominantStrategy::ZoneRush,
            EndCondition::FirstToReachTechLevel { .. } => DominantStrategy::BiomeAdaptation,
            EndCondition::SurvivalUntilTick(_) => DominantStrategy::Balanced,
        }
    }
}

// ─── WorldSeedConfig ─────────────────────────────────────────────────────────

/// Counter-adapted world configuration for the next run.
///
/// Consumed by WAC's world generator to bias biomes, loot, and spawns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSeedConfig {
    pub seed: u64,
    /// Suggest scarcity biomes (counters economy exploit).
    pub scarcity_biomes: bool,
    /// Suggest volatile loot markets (counters economy exploit).
    pub volatile_loot: bool,
    /// Increase zone contest frequency (counters zone rush).
    pub contested_zones: bool,
    /// Roaming AI trader NPCs (counters economy exploit).
    pub roaming_ai_traders: bool,
}

impl WorldSeedConfig {
    fn counter_adapted(seed: u64, strategy: Option<&DominantStrategy>) -> Self {
        let mut cfg = WorldSeedConfig {
            seed,
            scarcity_biomes:    false,
            volatile_loot:      false,
            contested_zones:    false,
            roaming_ai_traders: false,
        };
        match strategy {
            Some(DominantStrategy::EconomyExploit) => {
                cfg.scarcity_biomes    = true;
                cfg.volatile_loot      = true;
                cfg.roaming_ai_traders = true;
            }
            Some(DominantStrategy::ZoneRush) => {
                cfg.contested_zones = true;
            }
            _ => {}
        }
        cfg
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::end_condition::{EndCondition, WorldSnapshot};
    use crate::run::WorldRun;

    fn make_run(cond: EndCondition) -> WorldRun {
        WorldRun::new(cond, vec!["humans".into()], vec!["synthesis".into()], 42, "Test")
    }

    #[test]
    fn director_starts_pending_run() {
        let mut dir = WorldRunDirector::new();
        dir.add_run(make_run(EndCondition::SurvivalUntilTick(1000)));
        let run = dir.start_next_run(0);
        assert!(run.is_some());
        assert!(dir.active_run().is_some());
    }

    #[test]
    fn director_evaluates_time_expiry() {
        let mut dir = WorldRunDirector::new();
        dir.add_run(make_run(EndCondition::SurvivalUntilTick(100)));
        dir.start_next_run(0);
        let mut snap = WorldSnapshot::default();
        snap.current_tick = 100;
        let result = dir.evaluate_tick(100, &snap);
        assert!(result.is_some());
        assert!(dir.active_run().is_none()); // run should be ended
    }

    #[test]
    fn counter_seed_scarcity_for_economy_exploit() {
        let mut dir = WorldRunDirector::new();
        dir.last_dominant_strategy = Some(DominantStrategy::EconomyExploit);
        let cfg = dir.generate_next_world_seed(99);
        assert!(cfg.scarcity_biomes);
        assert!(cfg.volatile_loot);
    }
}
