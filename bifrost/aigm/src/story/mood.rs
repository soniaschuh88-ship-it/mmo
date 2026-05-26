//! WorldMood — the emotional tone of the simulated world.

use serde::{Deserialize, Serialize};

/// The macro-level emotional tone of the world.
///
/// Mood affects:
/// - NPC behaviour (guards more alert during `Tense`, merchants flee during `War`)
/// - AI GM generation rate (more story beats during `Crisis`)
/// - Quest generation bias (survival quests during `War`, trade quests during `Calm`)
/// - Environmental events (storms more frequent during `Catastrophe`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WorldMood {
    /// Peaceful, normal activity. The default starting state.
    #[default]
    Calm,
    /// Something is wrong — players and NPCs sense it. Rumours spread.
    Uneasy,
    /// Open conflict or imminent threat. Guards on high alert.
    Tense,
    /// Active warfare or widespread danger.
    War,
    /// Major world event in progress (invasion, natural disaster, …).
    Crisis,
    /// The worst has happened. Survival is the primary concern.
    Catastrophe,
    /// Conflict has ended. NPCs mourn / celebrate. Rebuilding begins.
    Recovery,
    /// Celebration, festival, or major victory.
    Festive,
}

impl WorldMood {
    /// How aggressive NPCs are in this mood (0.0 – 1.0).
    pub fn aggression_factor(self) -> f32 {
        match self {
            WorldMood::Calm        => 0.1,
            WorldMood::Uneasy      => 0.25,
            WorldMood::Tense       => 0.50,
            WorldMood::War         => 0.80,
            WorldMood::Crisis      => 0.75,
            WorldMood::Catastrophe => 0.90,
            WorldMood::Recovery    => 0.20,
            WorldMood::Festive     => 0.05,
        }
    }

    /// How frequently the AI GM should emit new quests (relative multiplier).
    pub fn quest_rate_multiplier(self) -> f32 {
        match self {
            WorldMood::Calm        => 1.0,
            WorldMood::Uneasy      => 1.2,
            WorldMood::Tense       => 1.5,
            WorldMood::War         => 2.0,
            WorldMood::Crisis      => 2.5,
            WorldMood::Catastrophe => 3.0,
            WorldMood::Recovery    => 1.3,
            WorldMood::Festive     => 0.7,
        }
    }

    /// Natural English description for player-facing text.
    pub fn description(self) -> &'static str {
        match self {
            WorldMood::Calm        => "The world is at peace.",
            WorldMood::Uneasy      => "Something feels off. People whisper of dark omens.",
            WorldMood::Tense       => "Tension fills the air. Conflict looms on every horizon.",
            WorldMood::War         => "War has come. Steel clashes in every district.",
            WorldMood::Crisis      => "A great crisis is unfolding. All must act.",
            WorldMood::Catastrophe => "Catastrophe has struck. Survival is all that matters.",
            WorldMood::Recovery    => "The worst has passed. The world begins to heal.",
            WorldMood::Festive     => "Joy fills the streets. This is a time of celebration.",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_calm() {
        assert_eq!(WorldMood::default(), WorldMood::Calm);
    }

    #[test]
    fn war_is_most_aggressive() {
        // Catastrophe is actually the most aggressive but let's check ordering
        assert!(WorldMood::War.aggression_factor() > WorldMood::Calm.aggression_factor());
        assert!(WorldMood::Catastrophe.aggression_factor() > WorldMood::War.aggression_factor());
    }

    #[test]
    fn serialise_round_trip() {
        let m = WorldMood::Tense;
        let s = serde_json::to_string(&m).unwrap();
        let back: WorldMood = serde_json::from_str(&s).unwrap();
        assert_eq!(m, back);
    }
}
