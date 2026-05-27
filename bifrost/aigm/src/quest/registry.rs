//! QuestRegistry — in-memory projection of all quest ledger events.
//!
//! Rebuilt deterministically from the event ledger on startup.
//! Keyed by `quest_id`; scoped to a single zone.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::types::{Quest, QuestObjective, QuestReward, QuestState};
use crate::event::{
    EventPayload, EventType, QuestObjectivePayload, WorldEvent,
};

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("quest not found: {0}")]
    NotFound(String),
    #[error("quest {0} is not in a state that allows this operation (current: {1:?})")]
    InvalidState(String, QuestState),
    #[error("player {0} has not accepted quest {1}")]
    PlayerNotAccepted(String, String),
}

/// Maximum number of completed/failed quests kept in the history ring buffer.
const HISTORY_RETENTION: usize = 256;

/// All quest state for one zone.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct QuestRegistry {
    /// Active + available quests.
    active: BTreeMap<String, Quest>,
    /// Completed / failed quests (ring buffer — oldest pruned first).
    history: Vec<Quest>,
}

impl QuestRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    // ── Replay from ledger ───────────────────────────────────────────────────

    /// Apply a single `WorldEvent` to update quest state.
    ///
    /// Call this in ledger-replay order (ascending `seq`) to rebuild the
    /// registry deterministically.
    pub fn apply_event(&mut self, event: &WorldEvent) -> Result<(), RegistryError> {
        match &event.event_type {
            EventType::AigmQuestCreate => {
                if let EventPayload::AigmQuestCreate(p) = &event.payload {
                    let objectives: Vec<QuestObjective> = p
                        .objectives
                        .iter()
                        .map(|o| obj_from_payload(o))
                        .collect();
                    let reward = QuestReward {
                        xp:         p.reward.xp,
                        gold:       p.reward.gold,
                        items:      p.reward.items.clone(),
                        reputation: p.reward.reputation
                            .iter()
                            .map(|r| (r.faction_id.clone(), r.delta))
                            .collect(),
                    };
                    let quest = Quest::new(
                        &p.quest_id,
                        &p.title,
                        &p.description,
                        &p.giver_npc_id,
                        objectives,
                        reward,
                        event.instant.seq,
                        p.expires_at,
                        &p.ai_context,
                    );
                    self.active.insert(quest.quest_id.clone(), quest);
                }
            }

            EventType::AigmQuestUpdate => {
                if let EventPayload::AigmQuestUpdate(p) = &event.payload {
                    let quest = self.active
                        .get_mut(&p.quest_id)
                        .ok_or_else(|| RegistryError::NotFound(p.quest_id.clone()))?;

                    // Accept the player implicitly on first progress event.
                    if !quest.progress.contains_key(&p.player_id) {
                        quest.accept(&p.player_id);
                    }
                    // Set absolute progress (the event carries the new value).
                    if let Some(player_prog) = quest.progress.get_mut(&p.player_id) {
                        if let Some(obj_prog) = player_prog.get_mut(&p.objective_id) {
                            obj_prog.current = p.progress.min(p.required);
                            if obj_prog.current >= obj_prog.required
                                && quest.state == QuestState::Active
                                && player_prog.values().all(|pp| pp.is_complete())
                            {
                                quest.state = QuestState::ReadyToComplete;
                            }
                        }
                    }
                }
            }

            EventType::AigmQuestComplete => {
                if let EventPayload::AigmQuestComplete(p) = &event.payload {
                    if let Some(mut quest) = self.active.remove(&p.quest_id) {
                        quest.complete();
                        self.push_history(quest);
                    }
                }
            }

            EventType::AigmQuestFail => {
                if let EventPayload::AigmQuestFail(p) = &event.payload {
                    if let Some(mut quest) = self.active.remove(&p.quest_id) {
                        quest.fail();
                        self.push_history(quest);
                    }
                }
            }

            _ => {} // Unrelated events — ignore.
        }
        Ok(())
    }

    // ── Queries ──────────────────────────────────────────────────────────────

    /// All active (available + in-progress) quests, sorted by `quest_id`.
    pub fn active_quests(&self) -> impl Iterator<Item = &Quest> {
        self.active.values()
    }

    /// Quests currently accepted by `player_id`.
    pub fn quests_for_player<'a>(
        &'a self,
        player_id: &'a str,
    ) -> impl Iterator<Item = &'a Quest> {
        self.active
            .values()
            .filter(move |q| q.target_player_ids.iter().any(|pid| pid == player_id))
    }

    /// Look up a quest by ID.
    pub fn get(&self, quest_id: &str) -> Option<&Quest> {
        self.active.get(quest_id)
    }

    /// Number of active quests.
    pub fn active_count(&self) -> usize {
        self.active.len()
    }

    /// Expire quests whose deadline has passed.
    ///
    /// Returns the IDs of quests that were expired.
    pub fn expire_quests(&mut self, now_ms: u64) -> Vec<String> {
        let expired: Vec<String> = self
            .active
            .values()
            .filter(|q| q.is_expired(now_ms))
            .map(|q| q.quest_id.clone())
            .collect();

        for id in &expired {
            if let Some(mut q) = self.active.remove(id) {
                q.fail();
                self.push_history(q);
            }
        }
        expired
    }

    // ── Internals ────────────────────────────────────────────────────────────

    fn push_history(&mut self, quest: Quest) {
        if self.history.len() >= HISTORY_RETENTION {
            self.history.remove(0);
        }
        self.history.push(quest);
    }
}

fn obj_from_payload(p: &QuestObjectivePayload) -> QuestObjective {
    use super::types::QuestObjectiveKind;
    let kind = match p.kind.as_str() {
        "kill"     => QuestObjectiveKind::Kill,
        "collect"  => QuestObjectiveKind::Collect,
        "explore"  => QuestObjectiveKind::Explore,
        "speak"    => QuestObjectiveKind::Speak,
        "build"    => QuestObjectiveKind::Build,
        "deliver"  => QuestObjectiveKind::Deliver,
        _          => QuestObjectiveKind::Kill,  // safe default
    };
    QuestObjective {
        objective_id:   p.objective_id.clone(),
        kind,
        description:    p.description.clone(),
        target_id:      p.target_id.clone(),
        required_count: p.required_count,
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use bifrost_kernel::SequencedInstant;
    use crate::event::{
        AuthorId, EventPayload, EventType, QuestCreatePayload,
        QuestObjectivePayload, QuestOutcomePayload, QuestRewardPayload,
        QuestUpdatePayload, ReputationChangePayload, WorldEvent,
    };

    fn genesis() -> [u8; 32] { [0u8; 32] }

    fn create_event(seq: u64, quest_id: &str, prev: &[u8; 32]) -> WorldEvent {
        WorldEvent::new(
            SequencedInstant::new(0, seq),
            EventType::AigmQuestCreate,
            EventPayload::AigmQuestCreate(QuestCreatePayload {
                quest_id:    quest_id.into(),
                title:       "Test Quest".into(),
                description: "Do the thing".into(),
                giver_npc_id: "npc-1".into(),
                target_ids:  vec!["player-1".into()],
                objectives:  vec![QuestObjectivePayload {
                    objective_id:   "obj-1".into(),
                    kind:           "kill".into(),
                    description:    "Kill 3 bandits".into(),
                    target_id:      Some("bandit".into()),
                    required_count: 3,
                }],
                reward: QuestRewardPayload {
                    xp: 100, gold: 50,
                    items: vec![],
                    reputation: vec![ReputationChangePayload {
                        faction_id: "guards".into(),
                        delta: 10,
                        reason: "quest".into(),
                    }],
                },
                expires_at: None,
                ai_context: "test".into(),
            }),
            AuthorId::AiGm,
            prev,
            "zone-1",
            0,
        )
    }

    fn update_event(seq: u64, quest_id: &str, progress: u32, prev: &[u8; 32]) -> WorldEvent {
        WorldEvent::new(
            SequencedInstant::new(0, seq),
            EventType::AigmQuestUpdate,
            EventPayload::AigmQuestUpdate(QuestUpdatePayload {
                quest_id: quest_id.into(),
                objective_id: "obj-1".into(),
                player_id: "player-1".into(),
                progress,
                required: 3,
            }),
            AuthorId::Player("player-1".into()),
            prev,
            "zone-1",
            0,
        )
    }

    fn complete_event(seq: u64, quest_id: &str, prev: &[u8; 32]) -> WorldEvent {
        WorldEvent::new(
            SequencedInstant::new(0, seq),
            EventType::AigmQuestComplete,
            EventPayload::AigmQuestComplete(QuestOutcomePayload {
                quest_id: quest_id.into(),
                player_id: "player-1".into(),
                reason: None,
            }),
            AuthorId::System,
            prev,
            "zone-1",
            0,
        )
    }

    #[test]
    fn create_adds_to_active() {
        let mut reg = QuestRegistry::new();
        let e = create_event(0, "q-1", &genesis());
        reg.apply_event(&e).unwrap();
        assert_eq!(reg.active_count(), 1);
        assert!(reg.get("q-1").is_some());
    }

    #[test]
    fn progress_advances_state() {
        let mut reg = QuestRegistry::new();
        let e0 = create_event(0, "q-1", &genesis());
        reg.apply_event(&e0).unwrap();

        let e1 = update_event(1, "q-1", 3, &e0.world_hash);
        reg.apply_event(&e1).unwrap();

        assert_eq!(reg.get("q-1").unwrap().state, QuestState::ReadyToComplete);
    }

    #[test]
    fn complete_moves_to_history() {
        let mut reg = QuestRegistry::new();
        let e0 = create_event(0, "q-1", &genesis());
        reg.apply_event(&e0).unwrap();
        let e1 = update_event(1, "q-1", 3, &e0.world_hash);
        reg.apply_event(&e1).unwrap();
        let e2 = complete_event(2, "q-1", &e1.world_hash);
        reg.apply_event(&e2).unwrap();

        assert_eq!(reg.active_count(), 0);
        assert_eq!(reg.history.len(), 1);
    }
}
