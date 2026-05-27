//! Meta-progression system — persistent cross-run player state.
//!
//! Two distinct layers of progression:
//!
//! | Layer | Duration | Examples |
//! |---|---|---|
//! | **Run** | Resets per run | skills, gear, bases, territory |
//! | **Meta** | Persistent across runs | unlocks, archetypes, starting perks |
//!
//! No hard wipe — losers suffer `SkillDecay` (soft reset), winners gain
//! `MetaUnlock` (permanent progression).  This creates asymmetry without
//! destroying the fun of losing.

use serde::{Deserialize, Serialize};

// ─── MetaProgression ─────────────────────────────────────────────────────────

/// Persistent cross-run state for a player or AI faction.
///
/// Survives world resets. Influences starting conditions of future runs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetaProgression {
    pub faction_id:   String,

    /// Permanently unlocked abilities / archetypes.
    pub unlocks:      Vec<MetaUnlock>,

    /// Cumulative reputation score (win/loss ledger).
    pub reputation:   f32,

    /// Starting bonus for the next run (reduced after use).
    pub starting_perk: Option<StartingPerk>,

    /// Number of runs completed (wins + losses).
    pub runs_completed: u32,

    /// Number of runs won.
    pub runs_won: u32,
}

impl MetaProgression {
    pub fn new(faction_id: impl Into<String>) -> Self {
        Self {
            faction_id: faction_id.into(),
            ..Default::default()
        }
    }

    /// Apply winner rewards after a successful run.
    pub fn apply_rewards(&mut self, rewards: &[RunReward]) {
        self.runs_completed += 1;
        self.runs_won       += 1;
        self.reputation     += 1.0;
        for r in rewards {
            match r {
                RunReward::Unlock(u)      => self.unlocks.push(u.clone()),
                RunReward::StartingPerk(p)=> self.starting_perk = Some(p.clone()),
                RunReward::ReputationBonus(v) => self.reputation += v,
            }
        }
    }

    /// Apply loser penalties after a failed run.
    pub fn apply_penalties(&mut self, decays: &[SkillDecay]) {
        self.runs_completed += 1;
        self.reputation     = (self.reputation - 0.5).max(0.0);
        // Skill decay is tracked but application is handled by the run itself.
        let _ = decays; // acknowledged
    }
}

// ─── RunReward ───────────────────────────────────────────────────────────────

/// A persistent reward granted to a winning faction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RunReward {
    /// Permanently unlock a capability or archetype.
    Unlock(MetaUnlock),
    /// Bonus applied at the start of the next run.
    StartingPerk(StartingPerk),
    /// Flat reputation bonus.
    ReputationBonus(f32),
}

// ─── MetaUnlock ──────────────────────────────────────────────────────────────

/// A permanently unlocked capability.
///
/// Unlocks persist across runs and influence the player's options in future
/// worlds — skill tree access, starting builds, biome interaction rules, etc.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetaUnlock {
    pub id:          String,
    pub domain:      SkillDomain,
    pub description: String,
}

/// Domains that unlocks affect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillDomain {
    /// Terrain shaping via WAC blueprints.
    Terrain,
    /// Economy + loot table manipulation.
    Economy,
    /// Combat physics advantage.
    Combat,
    /// Biome evolution control.
    BiomeInteraction,
    /// Zone capture speed + faction influence.
    FactionInfluence,
}

// ─── StartingPerk ────────────────────────────────────────────────────────────

/// A one-time bonus applied at the start of the next run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StartingPerk {
    pub id:          String,
    pub description: String,
    /// Stat multiplier applied to starting resources / tech.
    pub stat_bonus:  f32,
}

// ─── SkillDecay ──────────────────────────────────────────────────────────────

/// Soft penalty applied to losers — partial skill regression.
///
/// Not a full wipe: losers retain some progression but start the next run
/// at a slight disadvantage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillDecay {
    pub domain:     SkillDomain,
    /// Fraction of progress lost (0.0–1.0).
    pub decay_rate: f32,
    /// Human-readable reason for the decay.
    pub reason:     String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_rewards_increments_runs() {
        let mut meta = MetaProgression::new("humans");
        meta.apply_rewards(&[RunReward::ReputationBonus(2.0)]);
        assert_eq!(meta.runs_completed, 1);
        assert_eq!(meta.runs_won, 1);
        assert_eq!(meta.reputation, 3.0); // 1.0 base + 2.0 bonus
    }

    #[test]
    fn apply_penalties_does_not_go_negative() {
        let mut meta = MetaProgression::new("synthesis");
        meta.apply_penalties(&[]);
        assert!(meta.reputation >= 0.0);
    }

    #[test]
    fn unlock_stored_after_reward() {
        let mut meta = MetaProgression::new("humans");
        meta.apply_rewards(&[RunReward::Unlock(MetaUnlock {
            id: "biome_mastery".into(),
            domain: SkillDomain::BiomeInteraction,
            description: "Biome evolution 20% faster".into(),
        })]);
        assert_eq!(meta.unlocks.len(), 1);
    }
}
