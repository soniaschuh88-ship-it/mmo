//! Quest domain types.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

// ─── Quest state machine ──────────────────────────────────────────────────────

/// Lifecycle state of a quest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestState {
    /// Quest is available to be picked up (offered by NPC).
    Available,
    /// Player has accepted the quest and is working on it.
    Active,
    /// All objectives met — awaiting turn-in.
    ReadyToComplete,
    /// Successfully completed.
    Completed,
    /// Failed (expired, player died, conditions not met).
    Failed,
}

// ─── Objective ────────────────────────────────────────────────────────────────

/// The specific task a player must accomplish.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestObjectiveKind {
    /// Kill `required_count` of `target_id`.
    Kill,
    /// Collect `required_count` of item `target_id`.
    Collect,
    /// Visit the zone / coordinate identified by `target_id`.
    Explore,
    /// Speak to NPC `target_id`.
    Speak,
    /// Place `required_count` voxels of material `target_id`.
    Build,
    /// Deliver item `target_id` to NPC / location.
    Deliver,
}

/// A single objective within a quest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuestObjective {
    pub objective_id: String,
    pub kind: QuestObjectiveKind,
    pub description: String,
    /// Entity / item / zone this objective references.
    pub target_id: Option<String>,
    pub required_count: u32,
}

/// Per-player progress on a single objective.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ObjectiveProgress {
    pub current: u32,
    pub required: u32,
}

impl ObjectiveProgress {
    pub fn new(required: u32) -> Self {
        Self { current: 0, required }
    }

    pub fn is_complete(&self) -> bool {
        self.current >= self.required
    }

    pub fn advance(&mut self, by: u32) {
        self.current = self.current.saturating_add(by).min(self.required);
    }
}

// ─── Reward ───────────────────────────────────────────────────────────────────

/// Reward granted on quest completion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct QuestReward {
    pub xp: u32,
    pub gold: u32,
    /// Item IDs granted.
    pub items: Vec<String>,
    /// Reputation changes: faction_id → delta.
    pub reputation: BTreeMap<String, i32>,
}

// ─── Quest ────────────────────────────────────────────────────────────────────

/// A fully hydrated quest, reconstructed from ledger events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Quest {
    pub quest_id: String,
    pub title: String,
    pub description: String,

    /// NPC that offered this quest.
    pub giver_npc_id: String,

    /// Players this quest targets (multi-player quests possible).
    pub target_player_ids: Vec<String>,

    pub objectives: Vec<QuestObjective>,
    pub reward: QuestReward,

    /// Ledger seq at which this quest was created.
    pub created_at_seq: u64,

    /// Unix-ms deadline. `None` = no expiry.
    pub expires_at_ms: Option<u64>,

    pub state: QuestState,

    /// Per-player, per-objective progress.
    /// Key: player_id → objective_id → progress.
    pub progress: BTreeMap<String, BTreeMap<String, ObjectiveProgress>>,

    /// AI reasoning trace (audit only).
    pub ai_context: String,
}

impl Quest {
    /// Initialise a new quest in `Available` state with zero progress.
    pub fn new(
        quest_id: impl Into<String>,
        title: impl Into<String>,
        description: impl Into<String>,
        giver_npc_id: impl Into<String>,
        objectives: Vec<QuestObjective>,
        reward: QuestReward,
        created_at_seq: u64,
        expires_at_ms: Option<u64>,
        ai_context: impl Into<String>,
    ) -> Self {
        Self {
            quest_id: quest_id.into(),
            title: title.into(),
            description: description.into(),
            giver_npc_id: giver_npc_id.into(),
            target_player_ids: Vec::new(),
            objectives,
            reward,
            created_at_seq,
            expires_at_ms,
            state: QuestState::Available,
            progress: BTreeMap::new(),
            ai_context: ai_context.into(),
        }
    }

    /// Record that `player_id` has accepted the quest.
    pub fn accept(&mut self, player_id: impl Into<String>) {
        let pid = player_id.into();
        if !self.target_player_ids.contains(&pid) {
            self.target_player_ids.push(pid.clone());
        }
        // Initialise progress for this player.
        let player_progress = self.progress.entry(pid).or_default();
        for obj in &self.objectives {
            player_progress
                .entry(obj.objective_id.clone())
                .or_insert_with(|| ObjectiveProgress::new(obj.required_count));
        }
        if self.state == QuestState::Available {
            self.state = QuestState::Active;
        }
    }

    /// Advance `player_id`'s progress on `objective_id` by `by`.
    ///
    /// Returns `true` if all objectives for this player are now complete.
    pub fn advance_objective(
        &mut self,
        player_id: &str,
        objective_id: &str,
        by: u32,
    ) -> bool {
        if let Some(player_progress) = self.progress.get_mut(player_id) {
            if let Some(prog) = player_progress.get_mut(objective_id) {
                prog.advance(by);
            }
            let all_done = player_progress.values().all(|p| p.is_complete());
            if all_done && self.state == QuestState::Active {
                self.state = QuestState::ReadyToComplete;
            }
            return player_progress.values().all(|p| p.is_complete());
        }
        false
    }

    /// Mark the quest as completed for `player_id`.
    pub fn complete(&mut self) {
        self.state = QuestState::Completed;
    }

    /// Mark the quest as failed.
    pub fn fail(&mut self) {
        self.state = QuestState::Failed;
    }

    /// True if the quest has passed its deadline.
    pub fn is_expired(&self, now_ms: u64) -> bool {
        self.expires_at_ms.map_or(false, |exp| now_ms > exp)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn kill_objective(id: &str) -> QuestObjective {
        QuestObjective {
            objective_id: id.into(),
            kind: QuestObjectiveKind::Kill,
            description: "Kill the bandit".into(),
            target_id: Some("bandit_chief".into()),
            required_count: 3,
        }
    }

    fn simple_quest() -> Quest {
        Quest::new(
            "q-001", "Bandit Hunt", "Slay the bandits",
            "npc-aldric",
            vec![kill_objective("obj-1")],
            QuestReward { xp: 100, gold: 50, ..Default::default() },
            0, None, "test",
        )
    }

    #[test]
    fn accept_initialises_progress() {
        let mut q = simple_quest();
        q.accept("player-1");
        assert_eq!(q.state, QuestState::Active);
        assert!(q.progress.contains_key("player-1"));
    }

    #[test]
    fn objective_advance_and_complete() {
        let mut q = simple_quest();
        q.accept("player-1");
        q.advance_objective("player-1", "obj-1", 2);
        assert_eq!(q.state, QuestState::Active); // not done yet

        q.advance_objective("player-1", "obj-1", 1);
        assert_eq!(q.state, QuestState::ReadyToComplete);
    }

    #[test]
    fn progress_caps_at_required() {
        let mut q = simple_quest();
        q.accept("player-1");
        q.advance_objective("player-1", "obj-1", 999);
        let prog = &q.progress["player-1"]["obj-1"];
        assert_eq!(prog.current, 3);
    }

    #[test]
    fn expiry_check() {
        let mut q = simple_quest();
        q.expires_at_ms = Some(1000);
        assert!(!q.is_expired(999));
        assert!(q.is_expired(1001));
    }
}
