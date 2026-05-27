//! Top-level [`AiFaction`] and [`AiMetaFaction`] types.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::agent::AgentNode;
use crate::memory::{FactionMemory, RunMemoryGraph};
use crate::strategy::{StrategyGoal, WorldModel};

// R1 — One concept, one crate.
// FactionId and ZoneId are defined once in bifrost_kernel.
// Importing the canonical types rather than re-defining them here.
pub use bifrost_kernel::{FactionId, ZoneId};

// ─── AiFaction ───────────────────────────────────────────────────────────────

/// An active Synthesis AI faction competing in the current world run.
///
/// One run may have one or more `AiFaction` instances (e.g. Synthesis Alpha,
/// Synthesis Beta in team vs team modes).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiFaction {
    pub id: FactionId,

    /// Human-readable name for logs / UI.
    pub display_name: String,

    /// Current territory (zones owned by this faction).
    pub territory: Vec<ZoneId>,

    /// Squad/clan-level agents under this faction.
    pub agents: Vec<AgentNode>,

    /// High-level model of the current world state (faction's perception).
    pub world_model: WorldModel,

    /// In-run memory of events, player patterns, and outcomes.
    pub memory: FactionMemory,

    /// Current strategic goal stack (LIFO — top is highest priority).
    pub goal_stack: Vec<StrategyGoal>,

    /// Global resource score (used for economy competition).
    pub resource_score: f32,

    /// Technology level (used for tech-based win conditions).
    pub tech_level: u32,
}

impl AiFaction {
    pub fn new(id: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            territory: vec![],
            agents: vec![],
            world_model: WorldModel::default(),
            memory: FactionMemory::default(),
            goal_stack: vec![],
            resource_score: 0.0,
            tech_level: 1,
        }
    }

    /// Push a strategic goal onto the faction's stack.
    pub fn push_goal(&mut self, goal: StrategyGoal) {
        self.goal_stack.push(goal);
    }

    /// Pop the current top-priority goal.
    pub fn pop_goal(&mut self) -> Option<StrategyGoal> {
        self.goal_stack.pop()
    }

    /// Return the current top-priority goal (without removing it).
    pub fn current_goal(&self) -> Option<&StrategyGoal> {
        self.goal_stack.last()
    }

    /// Number of zones currently controlled.
    pub fn zone_count(&self) -> u32 {
        self.territory.len() as u32
    }
}

// ─── AiMetaFaction ───────────────────────────────────────────────────────────

/// Persistent cross-run state for the Synthesis AI civilisation.
///
/// Synthesis persists memory and strategy evolution across world resets.
/// It *learns the meta* between worlds and generates increasingly sophisticated
/// counter-strategies.
///
/// > "KI 'lernt die Meta' zwischen Welten" — `docs/CORE.md`
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AiMetaFaction {
    pub id: FactionId,

    /// Strategic memory accumulated across all completed runs.
    ///
    /// Each entry summarises a completed run: dominant player strategy,
    /// Synthesis win/loss, world configuration that was used.
    pub memory_across_runs: RunMemoryGraph,

    /// Strategy evolution tree: nodes are strategy archetypes; edges are
    /// observed effectiveness transitions across runs.
    pub strategy_evolution: StrategyEvolutionTree,

    /// Number of runs Synthesis has participated in.
    pub runs_played: u32,

    /// Number of runs Synthesis has won.
    pub runs_won: u32,
}

impl AiMetaFaction {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            ..Default::default()
        }
    }

    /// Record the outcome of a completed run.
    pub fn record_run(&mut self, run_summary: RunSummary) {
        self.runs_played += 1;
        if run_summary.synthesis_won {
            self.runs_won += 1;
        }
        self.memory_across_runs.entries.push(run_summary);
    }

    /// Win rate across all completed runs.
    pub fn win_rate(&self) -> f32 {
        if self.runs_played == 0 { return 0.0; }
        self.runs_won as f32 / self.runs_played as f32
    }
}

// ─── RunSummary ──────────────────────────────────────────────────────────────

/// Summary of a completed run stored in cross-run memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSummary {
    pub run_id:             Uuid,
    pub synthesis_won:      bool,
    pub player_strategy:    String,
    pub synthesis_strategy: String,
    pub world_seed:         u64,
    pub ticks_elapsed:      u64,
}

// ─── StrategyEvolutionTree ───────────────────────────────────────────────────

/// Tracks how Synthesis strategy archetypes evolved across runs.
///
/// Future: backed by a real graph DB for long-running campaigns.
/// For now: a simple list of strategy transitions with effectiveness scores.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StrategyEvolutionTree {
    pub nodes: Vec<StrategyNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyNode {
    pub strategy_id:   String,
    pub description:   String,
    /// Win rate when this strategy was used.
    pub effectiveness: f32,
    /// Run IDs where this strategy was deployed.
    pub run_ids:       Vec<Uuid>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn faction_starts_empty() {
        let f = AiFaction::new("synthesis-alpha", "Synthesis Alpha");
        assert_eq!(f.zone_count(), 0);
        assert!(f.current_goal().is_none());
    }

    #[test]
    fn goal_stack_lifo() {
        let mut f = AiFaction::new("s", "S");
        f.push_goal(StrategyGoal::ExpandTerritory);
        f.push_goal(StrategyGoal::AdaptBiome { zone_id: "A3".into() });
        assert!(matches!(f.current_goal(), Some(StrategyGoal::AdaptBiome { .. })));
        f.pop_goal();
        assert!(matches!(f.current_goal(), Some(StrategyGoal::ExpandTerritory)));
    }

    #[test]
    fn meta_faction_win_rate() {
        let mut meta = AiMetaFaction::new("synthesis");
        meta.record_run(RunSummary {
            run_id: Uuid::new_v4(), synthesis_won: true,
            player_strategy: "zone_rush".into(), synthesis_strategy: "economy".into(),
            world_seed: 1, ticks_elapsed: 500,
        });
        meta.record_run(RunSummary {
            run_id: Uuid::new_v4(), synthesis_won: false,
            player_strategy: "economy".into(), synthesis_strategy: "biome".into(),
            world_seed: 2, ticks_elapsed: 800,
        });
        assert!((meta.win_rate() - 0.5).abs() < f32::EPSILON);
    }
}
