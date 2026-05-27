//! Faction memory systems — in-run and cross-run.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::faction::{ZoneId, RunSummary};

// ─── FactionMemory (in-run) ──────────────────────────────────────────────────

/// In-run memory graph for the Synthesis faction.
///
/// Records events, player patterns, and outcomes within the current world run.
/// Cleared (or decayed) at run end; key patterns are promoted to [`RunMemoryGraph`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FactionMemory {
    /// All observed events this run.
    pub entries: Vec<MemoryEntry>,

    /// Maximum number of entries before oldest are evicted.
    pub capacity: usize,
}

impl FactionMemory {
    pub fn new(capacity: usize) -> Self {
        Self { entries: vec![], capacity }
    }

    /// Record a new memory entry.
    ///
    /// Evicts the oldest entry if capacity is reached (FIFO ring buffer).
    pub fn record(&mut self, entry: MemoryEntry) {
        if self.capacity > 0 && self.entries.len() >= self.capacity {
            self.entries.remove(0);
        }
        self.entries.push(entry);
    }

    /// Return entries matching a specific event type.
    pub fn filter(&self, event_type: MemoryEventType) -> Vec<&MemoryEntry> {
        self.entries.iter().filter(|e| e.event_type == event_type).collect()
    }

    /// Decay (halve) importance scores of all entries.
    ///
    /// Called each tick to simulate memory fading.  Entries below a threshold
    /// can be pruned before run end.
    pub fn decay(&mut self, threshold: f32) {
        for e in &mut self.entries { e.importance *= 0.95; }
        self.entries.retain(|e| e.importance >= threshold);
    }
}

// ─── MemoryEntry ─────────────────────────────────────────────────────────────

/// A single entry in the faction's in-run memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id:          Uuid,
    pub tick:        u64,
    pub event_type:  MemoryEventType,
    pub zone_id:     Option<ZoneId>,
    pub description: String,
    /// 0.0–1.0; decays over time.
    pub importance:  f32,
}

impl MemoryEntry {
    pub fn new(
        tick:        u64,
        event_type:  MemoryEventType,
        zone_id:     Option<ZoneId>,
        description: impl Into<String>,
        importance:  f32,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            tick,
            event_type,
            zone_id,
            description: description.into(),
            importance,
        }
    }
}

/// Categories of events the faction remembers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryEventType {
    /// Player built a structure in a zone.
    PlayerBuild,
    /// Player attacked a synthesis zone.
    PlayerAttack,
    /// Synthesis gained control of a zone.
    ZoneCaptured,
    /// Synthesis lost control of a zone.
    ZoneLost,
    /// Economy anomaly detected (loot flooding, price spike).
    EconomyAnomaly,
    /// Player crafting pattern observed in Safe City.
    CraftingPattern,
    /// Synthesis biome adaptation executed.
    BiomeAdapted,
}

// ─── RunMemoryGraph (cross-run) ──────────────────────────────────────────────

/// Cross-run memory for the `AiMetaFaction`.
///
/// Persists strategic insights across world resets.
/// Key inputs to the [`crate::strategy::StrategyEngine`] counter-strategy logic.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunMemoryGraph {
    /// Summaries of completed runs.
    pub entries: Vec<RunSummary>,
}

impl RunMemoryGraph {
    /// Return the dominant player strategy across recent runs.
    pub fn dominant_player_strategy(&self, window: usize) -> Option<&str> {
        let recent: Vec<_> = self.entries.iter().rev().take(window).collect();
        if recent.is_empty() { return None; }

        // Simple frequency count over strategy tags.
        let mut counts: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
        for r in &recent {
            *counts.entry(r.player_strategy.as_str()).or_insert(0) += 1;
        }
        counts.into_iter().max_by_key(|&(_, c)| c).map(|(s, _)| s)
    }

    /// Return the cross-run win rate for a given player strategy.
    pub fn synthesis_win_rate_vs(&self, player_strategy: &str) -> f32 {
        let relevant: Vec<_> = self.entries.iter()
            .filter(|r| r.player_strategy == player_strategy)
            .collect();
        if relevant.is_empty() { return 0.0; }
        let wins = relevant.iter().filter(|r| r.synthesis_won).count();
        wins as f32 / relevant.len() as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_evicts_at_capacity() {
        let mut mem = FactionMemory::new(2);
        for i in 0..3u64 {
            mem.record(MemoryEntry::new(i, MemoryEventType::ZoneCaptured, None, "test", 1.0));
        }
        assert_eq!(mem.entries.len(), 2);
        assert_eq!(mem.entries[0].tick, 1); // oldest (tick=0) evicted
    }

    #[test]
    fn memory_decay_removes_below_threshold() {
        let mut mem = FactionMemory::new(100);
        mem.record(MemoryEntry::new(0, MemoryEventType::EconomyAnomaly, None, "test", 0.01));
        mem.decay(0.1); // threshold above 0.01 * 0.95
        assert!(mem.entries.is_empty());
    }

    #[test]
    fn run_memory_dominant_strategy() {
        let mut rmg = RunMemoryGraph::default();
        for _ in 0..3 {
            rmg.entries.push(crate::faction::RunSummary {
                run_id: uuid::Uuid::new_v4(), synthesis_won: true,
                player_strategy: "zone_rush".into(), synthesis_strategy: "economy".into(),
                world_seed: 1, ticks_elapsed: 500,
            });
        }
        rmg.entries.push(crate::faction::RunSummary {
            run_id: uuid::Uuid::new_v4(), synthesis_won: false,
            player_strategy: "economy".into(), synthesis_strategy: "biome".into(),
            world_seed: 2, ticks_elapsed: 800,
        });
        assert_eq!(rmg.dominant_player_strategy(4), Some("zone_rush"));
    }
}
