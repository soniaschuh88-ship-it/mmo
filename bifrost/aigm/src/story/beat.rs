//! StoryBeat — a single narrative event within a StoryArc.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::mood::WorldMood;
use crate::event::StoryConsequence;

/// Lifecycle of a story beat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BeatState {
    /// Waiting for all trigger conditions to be satisfied.
    Pending,
    /// All conditions met — will fire on the next `StoryEngine::tick`.
    Ready,
    /// Beat has fired; its `AigmStoryBeat` event is in the ledger.
    Fired,
    /// Skipped (e.g. its arc was abandoned or superseded).
    Skipped,
}

/// A single condition that must hold for a beat to fire.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TriggerCondition {
    /// World tick must be at least this value.
    MinTick { tick: u64 },
    /// The named quest must be in the completed set.
    QuestCompleted { quest_id: String },
    /// The named NPC must be dead.
    NpcDead { npc_id: String },
    /// At least this many players must be in the affected zones.
    PlayerCount { min: u32 },
    /// Current world mood must equal this value.
    MoodIs { mood: WorldMood },
    /// The named beat must have already fired.
    PreviousBeat { beat_id: String },
}

/// A narrative beat: one atomic moment in a story arc.
///
/// Beats are designed by the AI GM offline (or bootstrapped from templates)
/// and stored as data. The [`StoryEngine`](super::engine::StoryEngine) evaluates
/// their trigger conditions each tick.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoryBeat {
    pub beat_id: String,
    /// The arc this beat belongs to.
    pub arc_id: String,
    /// Position in the arc (lower = earlier).
    pub order: u32,

    pub title: String,
    pub description: String,

    /// Zones affected by this beat.
    pub affected_zones: Vec<String>,

    /// All conditions must hold simultaneously.
    pub triggers: Vec<TriggerCondition>,

    /// What the world does when this beat fires.
    pub consequences: Vec<StoryConsequence>,

    pub state: BeatState,

    /// Ledger seq at which this beat fired (`None` if not yet fired).
    pub fired_at_seq: Option<u64>,

    /// AI reasoning trace (audit only).
    pub ai_context: String,
}

impl StoryBeat {
    /// Evaluate whether all trigger conditions are currently satisfied.
    pub fn is_ready(
        &self,
        current_tick: u64,
        completed_quests: &BTreeSet<String>,
        dead_npcs: &BTreeSet<String>,
        active_player_count: u32,
        world_mood: WorldMood,
        fired_beats: &BTreeSet<String>,
    ) -> bool {
        if self.state != BeatState::Pending {
            return false;
        }
        self.triggers.iter().all(|cond| match cond {
            TriggerCondition::MinTick { tick }         => current_tick >= *tick,
            TriggerCondition::QuestCompleted { quest_id } => completed_quests.contains(quest_id),
            TriggerCondition::NpcDead { npc_id }       => dead_npcs.contains(npc_id),
            TriggerCondition::PlayerCount { min }      => active_player_count >= *min,
            TriggerCondition::MoodIs { mood }          => world_mood == *mood,
            TriggerCondition::PreviousBeat { beat_id } => fired_beats.contains(beat_id),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_beat(triggers: Vec<TriggerCondition>) -> StoryBeat {
        StoryBeat {
            beat_id:        "beat-1".into(),
            arc_id:         "arc-1".into(),
            order:          0,
            title:          "Test Beat".into(),
            description:    "Something happens".into(),
            affected_zones: vec!["zone-1".into()],
            triggers,
            consequences:   vec![],
            state:          BeatState::Pending,
            fired_at_seq:   None,
            ai_context:     "test".into(),
        }
    }

    #[test]
    fn no_conditions_always_ready() {
        let beat = simple_beat(vec![]);
        assert!(beat.is_ready(
            0,
            &BTreeSet::new(),
            &BTreeSet::new(),
            0,
            WorldMood::Calm,
            &BTreeSet::new(),
        ));
    }

    #[test]
    fn min_tick_condition() {
        let beat = simple_beat(vec![TriggerCondition::MinTick { tick: 100 }]);
        assert!(!beat.is_ready(99, &BTreeSet::new(), &BTreeSet::new(), 0, WorldMood::Calm, &BTreeSet::new()));
        assert!( beat.is_ready(100, &BTreeSet::new(), &BTreeSet::new(), 0, WorldMood::Calm, &BTreeSet::new()));
    }

    #[test]
    fn quest_completed_condition() {
        let beat = simple_beat(vec![TriggerCondition::QuestCompleted { quest_id: "q-1".into() }]);
        let mut done = BTreeSet::new();
        assert!(!beat.is_ready(0, &done, &BTreeSet::new(), 0, WorldMood::Calm, &BTreeSet::new()));
        done.insert("q-1".into());
        assert!( beat.is_ready(0, &done, &BTreeSet::new(), 0, WorldMood::Calm, &BTreeSet::new()));
    }

    #[test]
    fn fired_beat_is_not_ready() {
        let mut beat = simple_beat(vec![]);
        beat.state = BeatState::Fired;
        assert!(!beat.is_ready(0, &BTreeSet::new(), &BTreeSet::new(), 0, WorldMood::Calm, &BTreeSet::new()));
    }
}
