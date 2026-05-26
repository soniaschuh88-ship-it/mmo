//! Layer 1 — Reactive NPC behaviour state machine.
//!
//! Every tick, every NPC runs through a cheap state machine transition.
//! No LLM, no async, no allocation beyond what serde needs.
//!
//! ```text
//! Idle ──────────────────────────────► Patrol
//!   ▲         player_nearby              │
//!   │                                    ▼
//!   │         aggro_range        ┌──► Chase
//!   │         exceeded ◄─────────┘       │
//!   │                                    │  target_lost
//!   │         flee_threshold             ▼
//!   └──────── hp_ok ◄──────────── Flee
//!                                        │
//!                                 player_speaks
//!                                        ▼
//!                                    Speak  ──► (triggers Layer 2)
//! ```

use serde::{Deserialize, Serialize};

/// Current behaviour mode of an NPC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BehaviorState {
    /// Standing still, occasional ambient animations.
    #[default]
    Idle,
    /// Walking a predefined route between waypoints.
    Patrol,
    /// Actively pursuing a target entity.
    Chase,
    /// Running away from a threat.
    Flee,
    /// Engaged in dialogue; movement paused.
    Speak,
    /// NPC has died (terminal state until respawn).
    Dead,
    /// Respawning after death (transitional).
    Respawning,
}

/// What caused a behaviour transition this tick.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BehaviorTransition {
    /// No change this tick.
    None,
    /// NPC detected a player within aggro range.
    PlayerDetected { player_id: String },
    /// NPC lost sight of its target.
    TargetLost,
    /// NPC health dropped below flee threshold.
    FleeThresholdReached,
    /// NPC health recovered / target gone.
    FleeEnded,
    /// Player initiated dialogue.
    PlayerSpoke { player_id: String },
    /// Dialogue concluded.
    DialogueEnded,
    /// Waypoint reached — continuing patrol.
    WaypointReached { waypoint_index: usize },
    /// NPC died.
    Died,
    /// NPC finished respawning.
    Respawned,
}

/// Per-NPC behaviour configuration (tuning knobs).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BehaviorConfig {
    /// Distance (voxels) at which a hostile NPC aggros a player.
    pub aggro_range: f32,
    /// Distance (voxels) at which the NPC gives up chasing.
    pub leash_range: f32,
    /// HP fraction [0.0, 1.0] below which the NPC flees.
    pub flee_hp_fraction: f32,
    /// Faction: "friendly" | "neutral" | "hostile"
    pub faction: NpcFaction,
    /// Waypoints for patrol (zone-relative voxel coords).
    pub patrol_waypoints: Vec<[f32; 3]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NpcFaction {
    #[default]
    Neutral,
    Friendly,
    Hostile,
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            aggro_range:       16.0,
            leash_range:       48.0,
            flee_hp_fraction:  0.20,
            faction:           NpcFaction::Neutral,
            patrol_waypoints:  Vec::new(),
        }
    }
}

/// Mutable per-NPC behaviour state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NpcBehavior {
    pub state: BehaviorState,
    pub config: BehaviorConfig,

    /// Entity the NPC is currently chasing / fleeing from.
    pub target_entity_id: Option<String>,

    /// Index into `config.patrol_waypoints`.
    pub patrol_waypoint_index: usize,

    /// Ticks remaining in current speak interaction.
    pub speak_ticks_remaining: u32,
}

impl NpcBehavior {
    pub fn new(config: BehaviorConfig) -> Self {
        Self {
            state:                  BehaviorState::Idle,
            config,
            target_entity_id:       None,
            patrol_waypoint_index:  0,
            speak_ticks_remaining:  0,
        }
    }

    /// Tick the state machine.
    ///
    /// Returns the transition that occurred (if any).
    /// All inputs are provided by the caller (no world lookups inside).
    pub fn tick(
        &mut self,
        hp_fraction: f32,
        nearest_player: Option<(&str, f32)>,  // (player_id, distance)
        player_speaking: Option<&str>,
    ) -> BehaviorTransition {
        if self.state == BehaviorState::Dead || self.state == BehaviorState::Respawning {
            return BehaviorTransition::None;
        }

        // Player initiates dialogue — highest priority.
        if let Some(pid) = player_speaking {
            if self.state != BehaviorState::Speak {
                self.state = BehaviorState::Speak;
                self.speak_ticks_remaining = 60; // 1 second at 60Hz
                return BehaviorTransition::PlayerSpoke { player_id: pid.into() };
            }
        }

        // Tick down active dialogue.
        if self.state == BehaviorState::Speak {
            self.speak_ticks_remaining = self.speak_ticks_remaining.saturating_sub(1);
            if self.speak_ticks_remaining == 0 {
                self.state = BehaviorState::Idle;
                return BehaviorTransition::DialogueEnded;
            }
            return BehaviorTransition::None;
        }

        // Flee if health critically low.
        if hp_fraction < self.config.flee_hp_fraction
            && self.config.faction != NpcFaction::Friendly
        {
            if self.state != BehaviorState::Flee {
                self.state = BehaviorState::Flee;
                return BehaviorTransition::FleeThresholdReached;
            }
            return BehaviorTransition::None;
        }

        // Recover from flee if healthy again.
        if self.state == BehaviorState::Flee
            && hp_fraction >= self.config.flee_hp_fraction + 0.10
        {
            self.state = BehaviorState::Idle;
            self.target_entity_id = None;
            return BehaviorTransition::FleeEnded;
        }

        // Hostile NPC: aggro nearest player.
        if self.config.faction == NpcFaction::Hostile {
            if let Some((pid, dist)) = nearest_player {
                if dist <= self.config.aggro_range {
                    if self.state != BehaviorState::Chase
                        || self.target_entity_id.as_deref() != Some(pid)
                    {
                        self.state = BehaviorState::Chase;
                        self.target_entity_id = Some(pid.into());
                        return BehaviorTransition::PlayerDetected { player_id: pid.into() };
                    }
                    return BehaviorTransition::None;
                }
                // Leash: give up if too far.
                if self.state == BehaviorState::Chase && dist > self.config.leash_range {
                    self.state = BehaviorState::Idle;
                    self.target_entity_id = None;
                    return BehaviorTransition::TargetLost;
                }
            } else if self.state == BehaviorState::Chase {
                self.state = BehaviorState::Idle;
                self.target_entity_id = None;
                return BehaviorTransition::TargetLost;
            }
        }

        // Friendly NPC: greet nearest player when they enter greeting range.
        //
        // `aggro_range` doubles as "greeting range" for friendly NPCs.
        // `target_entity_id` tracks the last greeted player so repeated
        // triggers for the same player are suppressed until they leave range.
        if self.config.faction == NpcFaction::Friendly {
            if let Some((pid, dist)) = nearest_player {
                if dist <= self.config.aggro_range
                    && self.target_entity_id.as_deref() != Some(pid)
                {
                    self.target_entity_id = Some(pid.into());
                    return BehaviorTransition::PlayerDetected { player_id: pid.into() };
                }
            } else {
                // No player nearby — reset so the next approach triggers again.
                self.target_entity_id = None;
            }
        }

        // Patrol if waypoints defined.
        if !self.config.patrol_waypoints.is_empty() && self.state == BehaviorState::Idle {
            self.state = BehaviorState::Patrol;
        }

        BehaviorTransition::None
    }

    /// Called when the NPC dies.
    pub fn on_death(&mut self) -> BehaviorTransition {
        self.state = BehaviorState::Dead;
        self.target_entity_id = None;
        BehaviorTransition::Died
    }

    /// Called when the NPC finishes respawning.
    pub fn on_respawn(&mut self) -> BehaviorTransition {
        self.state = BehaviorState::Idle;
        self.patrol_waypoint_index = 0;
        BehaviorTransition::Respawned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hostile_npc() -> NpcBehavior {
        NpcBehavior::new(BehaviorConfig {
            faction: NpcFaction::Hostile,
            aggro_range: 10.0,
            leash_range: 30.0,
            flee_hp_fraction: 0.2,
            ..Default::default()
        })
    }

    #[test]
    fn aggro_on_nearby_player() {
        let mut npc = hostile_npc();
        let t = npc.tick(1.0, Some(("p1", 5.0)), None);
        assert_eq!(npc.state, BehaviorState::Chase);
        assert!(matches!(t, BehaviorTransition::PlayerDetected { .. }));
    }

    #[test]
    fn leash_returns_to_idle() {
        let mut npc = hostile_npc();
        npc.tick(1.0, Some(("p1", 5.0)), None);      // aggro
        npc.tick(1.0, Some(("p1", 50.0)), None);      // leash
        assert_eq!(npc.state, BehaviorState::Idle);
    }

    #[test]
    fn flee_on_low_hp() {
        let mut npc = hostile_npc();
        let t = npc.tick(0.1, None, None);
        assert_eq!(npc.state, BehaviorState::Flee);
        assert!(matches!(t, BehaviorTransition::FleeThresholdReached));
    }

    #[test]
    fn dialogue_overrides_chase() {
        let mut npc = hostile_npc();
        npc.tick(1.0, Some(("p1", 5.0)), None);       // start chase
        let t = npc.tick(1.0, Some(("p1", 5.0)), Some("p1")); // player speaks
        assert_eq!(npc.state, BehaviorState::Speak);
        assert!(matches!(t, BehaviorTransition::PlayerSpoke { .. }));
    }
}
