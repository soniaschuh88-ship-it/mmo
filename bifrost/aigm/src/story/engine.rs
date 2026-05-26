//! StoryEngine — drives narrative arcs and decides when beats fire.
//!
//! The engine is a pure function of its state + incoming events:
//!
//! ```text
//! StoryEngine::tick(events_this_tick) → Vec<StoryBeatPayload>
//! ```
//!
//! It maintains:
//! - All registered [`StoryArc`]s
//! - The set of completed quest IDs
//! - The set of dead NPC IDs  
//! - The current [`WorldMood`]
//!
//! Only one story beat may fire per tick (see [`MAX_BEATS_PER_TICK`]).

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::{
    arc::{ArcState, StoryArc},
    beat::BeatState,
    mood::WorldMood,
};
use crate::event::{
    EventPayload, EventType, StoryBeatPayload, WorldEvent,
};

/// Hard cap: at most this many beats can fire in one tick.
const MAX_BEATS_PER_TICK: usize = 1;

/// Minimum ticks between consecutive beat fires (cooldown).
const BEAT_COOLDOWN_TICKS: u64 = 60;

/// The narrative state machine for one world.
#[derive(Debug, Serialize, Deserialize)]
pub struct StoryEngine {
    pub arcs: Vec<StoryArc>,
    pub world_mood: WorldMood,

    /// Quest IDs that have been completed.
    pub completed_quests: BTreeSet<String>,

    /// NPC IDs that have died.
    pub dead_npcs: BTreeSet<String>,

    /// Beat IDs that have fired (across all arcs).
    pub fired_beats: BTreeSet<String>,

    /// Tick at which the last beat fired.
    last_beat_tick: u64,
}

impl StoryEngine {
    pub fn new() -> Self {
        Self {
            arcs:             Vec::new(),
            world_mood:       WorldMood::default(),
            completed_quests: BTreeSet::new(),
            dead_npcs:        BTreeSet::new(),
            fired_beats:      BTreeSet::new(),
            last_beat_tick:   0,
        }
    }

    /// Register a new story arc.
    pub fn add_arc(&mut self, arc: StoryArc) {
        self.arcs.push(arc);
    }

    // ── Ledger replay ─────────────────────────────────────────────────────────

    /// Update internal state from a single ledger event.
    ///
    /// Call in ascending `seq` order to rebuild deterministically.
    pub fn apply_event(&mut self, event: &WorldEvent) {
        match &event.event_type {
            EventType::AigmQuestComplete => {
                if let EventPayload::AigmQuestComplete(p) = &event.payload {
                    self.completed_quests.insert(p.quest_id.clone());
                }
            }
            EventType::CombatDeath => {
                if let EventPayload::CombatDeath(p) = &event.payload {
                    self.dead_npcs.insert(p.entity_id.clone());
                }
            }
            EventType::AigmStoryBeat => {
                if let EventPayload::AigmStoryBeat(p) = &event.payload {
                    self.fired_beats.insert(p.beat_id.clone());
                    self.last_beat_tick = event.seq; // seq used as proxy for tick
                    // Mark the beat in the arc data structure.
                    if let Some(arc) = self.arcs.iter_mut().find(|a| a.arc_id == p.arc_id) {
                        arc.mark_beat_fired(&p.beat_id, event.seq);
                    }
                }
            }
            EventType::AigmEventWorld => {
                // World events can shift mood.
                if let EventPayload::AigmEventWorld(p) = &event.payload {
                    if let Ok(mood) = serde_json::from_str::<WorldMood>(
                        &format!("\"{}\"", p.event_name)
                    ) {
                        // Only shift mood if the event name happens to be a mood tag.
                        // Proper mood changes come from StoryConsequence::ChangeWorldMood.
                        let _ = mood;
                    }
                }
            }
            EventType::CombatResurrect => {
                if let EventPayload::Raw(v) = &event.payload {
                    if let Some(npc_id) = v.get("entity_id").and_then(|v| v.as_str()) {
                        self.dead_npcs.remove(npc_id);
                    }
                }
            }
            _ => {}
        }
    }

    // ── Tick ──────────────────────────────────────────────────────────────────

    /// Evaluate all pending beats and return payloads for any that are ready.
    ///
    /// Returns at most [`MAX_BEATS_PER_TICK`] beat payloads.
    /// Callers convert these to `WorldEvent`s and commit them to the ledger.
    pub fn tick(
        &mut self,
        current_tick: u64,
        active_player_count: u32,
    ) -> Vec<StoryBeatPayload> {
        // Enforce cooldown between beats.
        if current_tick.saturating_sub(self.last_beat_tick) < BEAT_COOLDOWN_TICKS
            && self.last_beat_tick != 0
        {
            return vec![];
        }

        let mut fired = Vec::new();

        // Iterate arcs in registration order (deterministic: Vec).
        'outer: for arc in &mut self.arcs {
            if !arc.is_active() {
                continue;
            }

            for beat in &mut arc.beats {
                if beat.state != BeatState::Pending {
                    continue;
                }

                let ready = beat.is_ready(
                    current_tick,
                    &self.completed_quests,
                    &self.dead_npcs,
                    active_player_count,
                    self.world_mood,
                    &self.fired_beats,
                );

                if ready {
                    let payload = StoryBeatPayload {
                        beat_id:        beat.beat_id.clone(),
                        arc_id:         beat.arc_id.clone(),
                        title:          beat.title.clone(),
                        description:    beat.description.clone(),
                        affected_zones: beat.affected_zones.clone(),
                        consequences:   beat.consequences.clone(),
                        ai_context:     beat.ai_context.clone(),
                    };

                    // Mark as ready (will be committed externally).
                    beat.state = BeatState::Ready;
                    self.fired_beats.insert(beat.beat_id.clone());
                    self.last_beat_tick = current_tick;

                    fired.push(payload);
                    if fired.len() >= MAX_BEATS_PER_TICK {
                        break 'outer;
                    }
                }
            }
        }

        // Apply StoryConsequence::ChangeWorldMood from fired beats.
        for payload in &fired {
            for consequence in &payload.consequences {
                if let crate::event::StoryConsequence::ChangeWorldMood { new_mood } = consequence {
                    if let Ok(mood) = serde_json::from_str::<WorldMood>(
                        &format!("\"{new_mood}\"")
                    ) {
                        self.world_mood = mood;
                    }
                }
            }
        }

        fired
    }

    /// Active arcs (not yet completed or abandoned).
    pub fn active_arcs(&self) -> impl Iterator<Item = &StoryArc> {
        self.arcs.iter().filter(|a| a.is_active())
    }
}

impl Default for StoryEngine {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::story::{
        arc::StoryArc,
        beat::{BeatState, StoryBeat, TriggerCondition},
    };

    fn simple_beat(id: &str, min_tick: u64) -> StoryBeat {
        StoryBeat {
            beat_id:        id.into(),
            arc_id:         "arc-1".into(),
            order:          0,
            title:          id.into(),
            description:    String::new(),
            affected_zones: vec!["zone-1".into()],
            triggers:       vec![TriggerCondition::MinTick { tick: min_tick }],
            consequences:   vec![],
            state:          BeatState::Pending,
            fired_at_seq:   None,
            ai_context:     String::new(),
        }
    }

    #[test]
    fn beat_fires_when_ready() {
        let mut engine = StoryEngine::new();
        let arc = StoryArc::new(
            "arc-1", "Test Arc", "Synopsis",
            vec!["zone-1".into()],
            vec![simple_beat("beat-1", 0)],
        );
        engine.add_arc(arc);

        let beats = engine.tick(0, 1);
        assert_eq!(beats.len(), 1);
        assert_eq!(beats[0].beat_id, "beat-1");
    }

    #[test]
    fn beat_does_not_fire_before_min_tick() {
        let mut engine = StoryEngine::new();
        let arc = StoryArc::new(
            "arc-1", "T", "S",
            vec![],
            vec![simple_beat("beat-1", 100)],
        );
        engine.add_arc(arc);

        let beats = engine.tick(50, 1);
        assert!(beats.is_empty());
    }

    #[test]
    fn cooldown_prevents_rapid_firing() {
        let mut engine = StoryEngine::new();
        let arc = StoryArc::new(
            "arc-1", "T", "S",
            vec![],
            vec![simple_beat("beat-1", 0), simple_beat("beat-2", 0)],
        );
        engine.add_arc(arc);

        let first  = engine.tick(0, 1);
        let second = engine.tick(1, 1); // only 1 tick later — still in cooldown
        assert_eq!(first.len(), 1);
        assert!(second.is_empty());
    }

    #[test]
    fn max_one_beat_per_tick() {
        let mut engine = StoryEngine::new();
        let arc = StoryArc::new(
            "arc-1", "T", "S",
            vec![],
            vec![simple_beat("beat-1", 0), simple_beat("beat-2", 0)],
        );
        engine.add_arc(arc);

        // Reset last_beat_tick to 0 to bypass cooldown for this test
        engine.last_beat_tick = 0;
        let beats = engine.tick(0, 1);
        assert!(beats.len() <= 1);
    }
}
