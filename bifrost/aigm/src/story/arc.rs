//! StoryArc — an ordered sequence of story beats forming a complete narrative.

use serde::{Deserialize, Serialize};

use super::beat::{BeatState, StoryBeat};

/// Lifecycle of a story arc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArcState {
    /// Not yet started.
    Pending,
    /// At least one beat has fired.
    Active,
    /// All beats have fired — the arc is complete.
    Completed,
    /// The arc was abandoned (player count dropped, world mood changed, etc.).
    Abandoned,
}

/// A complete narrative arc composed of ordered [`StoryBeat`]s.
///
/// Arcs are the AI GM's primary tool for long-form storytelling. A world can
/// have multiple arcs running in parallel, each targeting different zones or
/// player groups.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoryArc {
    pub arc_id: String,
    pub title: String,
    pub synopsis: String,

    /// Zones primarily affected by this arc.
    pub primary_zones: Vec<String>,

    /// Ordered beats. The `order` field on each beat governs sequence.
    pub beats: Vec<StoryBeat>,

    pub state: ArcState,

    /// Ledger seq when the arc became active.
    pub started_at_seq: Option<u64>,

    /// Ledger seq when the arc completed or was abandoned.
    pub ended_at_seq: Option<u64>,
}

impl StoryArc {
    pub fn new(
        arc_id: impl Into<String>,
        title: impl Into<String>,
        synopsis: impl Into<String>,
        primary_zones: Vec<String>,
        mut beats: Vec<StoryBeat>,
    ) -> Self {
        // Ensure beats are sorted by their declared order.
        beats.sort_by_key(|b| b.order);
        Self {
            arc_id: arc_id.into(),
            title: title.into(),
            synopsis: synopsis.into(),
            primary_zones,
            beats,
            state: ArcState::Pending,
            started_at_seq: None,
            ended_at_seq: None,
        }
    }

    /// IDs of beats that have already fired.
    pub fn fired_beat_ids(&self) -> impl Iterator<Item = &str> {
        self.beats
            .iter()
            .filter(|b| b.state == BeatState::Fired)
            .map(|b| b.beat_id.as_str())
    }

    /// The next beat in sequence that is still `Pending`.
    pub fn next_pending_beat(&self) -> Option<&StoryBeat> {
        self.beats.iter().find(|b| b.state == BeatState::Pending)
    }

    /// Mark the beat with `beat_id` as fired.
    ///
    /// Updates arc state to `Active` if it was `Pending`, and to `Completed`
    /// if all beats have now fired.
    pub fn mark_beat_fired(&mut self, beat_id: &str, at_seq: u64) {
        if let Some(beat) = self.beats.iter_mut().find(|b| b.beat_id == beat_id) {
            beat.state = BeatState::Fired;
            beat.fired_at_seq = Some(at_seq);
        }
        if self.state == ArcState::Pending {
            self.state = ArcState::Active;
            self.started_at_seq = Some(at_seq);
        }
        if self.beats.iter().all(|b| b.state == BeatState::Fired) {
            self.state = ArcState::Completed;
            self.ended_at_seq = Some(at_seq);
        }
    }

    /// Abandon this arc.
    pub fn abandon(&mut self, at_seq: u64) {
        self.state = ArcState::Abandoned;
        self.ended_at_seq = Some(at_seq);
        for beat in &mut self.beats {
            if beat.state == BeatState::Pending {
                beat.state = BeatState::Skipped;
            }
        }
    }

    /// True if the arc is still producing beats.
    pub fn is_active(&self) -> bool {
        matches!(self.state, ArcState::Pending | ArcState::Active)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::story::beat::TriggerCondition;

    fn beat(id: &str, order: u32) -> StoryBeat {
        StoryBeat {
            beat_id:        id.into(),
            arc_id:         "arc-1".into(),
            order,
            title:          id.into(),
            description:    String::new(),
            affected_zones: vec![],
            triggers:       vec![TriggerCondition::MinTick { tick: 0 }],
            consequences:   vec![],
            state:          BeatState::Pending,
            fired_at_seq:   None,
            ai_context:     String::new(),
        }
    }

    #[test]
    fn beats_sorted_on_construction() {
        let arc = StoryArc::new("a", "T", "S", vec![], vec![beat("b", 2), beat("a", 1)]);
        assert_eq!(arc.beats[0].beat_id, "a");
        assert_eq!(arc.beats[1].beat_id, "b");
    }

    #[test]
    fn mark_fired_advances_state() {
        let mut arc = StoryArc::new("a", "T", "S", vec![], vec![beat("b1", 0), beat("b2", 1)]);
        arc.mark_beat_fired("b1", 10);
        assert_eq!(arc.state, ArcState::Active);

        arc.mark_beat_fired("b2", 20);
        assert_eq!(arc.state, ArcState::Completed);
        assert_eq!(arc.ended_at_seq, Some(20));
    }

    #[test]
    fn abandon_skips_pending_beats() {
        let mut arc = StoryArc::new("a", "T", "S", vec![], vec![beat("b1", 0), beat("b2", 1)]);
        arc.abandon(5);
        assert_eq!(arc.state, ArcState::Abandoned);
        assert!(arc.beats.iter().all(|b| b.state == BeatState::Skipped));
    }
}
