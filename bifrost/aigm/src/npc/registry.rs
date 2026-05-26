//! NPC registry — per-zone collection of NPC state.
//!
//! [`NpcRegistry`] owns every [`NpcState`] in a zone and drives the Layer 1
//! state machine for all of them on each tick.  The results of that drive are
//! returned as a list of [`PendingDialogue`] items to be enqueued and
//! dispatched by the AI GM layer.
//!
//! ## Usage
//!
//! ```text
//! // Each game tick:
//! let triggers = registry.tick(&tick_input, now_ms);
//! for pending in triggers {
//!     // Respect Layer 2 cooldown before enqueuing.
//!     if registry.get(&pending.npc_id)
//!         .map(|s| s.ai_context.can_speak(now_ms))
//!         .unwrap_or(false)
//!     {
//!         dialogue_queue.push(pending);
//!     }
//! }
//! ```

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::behavior::{BehaviorConfig, NpcBehavior, NpcFaction};
use super::context::AiContext;
use super::dialogue::{
    DialogueQueue, NpcDialogueTrigger, NpcLlmRequest, PendingDialogue,
    PlayerDialogueContext,
};

// ─── NpcState ─────────────────────────────────────────────────────────────────

/// Full state of a single NPC in the ECS.
///
/// Combines all three NPC layers:
/// - [`NpcBehavior`]  — Layer 1: reactive state machine
/// - [`AiContext`]    — Layer 2/3: LLM personality, short-term memory, prompt cache key
/// - Health + position — raw simulation data used as inputs to the state machine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcState {
    /// Stable identifier — matches the NPC definition in the blueprint database.
    pub npc_id: String,

    // ── Layer 1 ────────────────────────────────────────────────────────────
    /// Reactive behaviour state machine.
    pub behavior: NpcBehavior,

    // ── Layer 2 / 3 ────────────────────────────────────────────────────────
    /// AI personality, memory, and LLM configuration.
    pub ai_context: AiContext,

    // ── Simulation data ────────────────────────────────────────────────────
    /// Current hit points.
    pub hp_current: u32,
    /// Maximum hit points.
    pub hp_max: u32,
    /// Zone-relative position (voxel coordinates).
    pub position: [f32; 3],
    /// Zone this NPC belongs to.
    pub zone_id: String,
}

impl NpcState {
    /// Create a new NPC with full hp and default idle behaviour.
    pub fn new(
        npc_id: impl Into<String>,
        ai_context: AiContext,
        behavior_config: BehaviorConfig,
        hp_max: u32,
        position: [f32; 3],
        zone_id: impl Into<String>,
    ) -> Self {
        Self {
            npc_id: npc_id.into(),
            behavior: NpcBehavior::new(behavior_config),
            ai_context,
            hp_current: hp_max,
            hp_max,
            position,
            zone_id: zone_id.into(),
        }
    }

    /// Current health as a fraction of maximum [0.0, 1.0].
    pub fn hp_fraction(&self) -> f32 {
        if self.hp_max == 0 {
            0.0
        } else {
            self.hp_current as f32 / self.hp_max as f32
        }
    }

    /// True when the NPC is not dead or respawning.
    pub fn is_alive(&self) -> bool {
        use super::behavior::BehaviorState;
        !matches!(
            self.behavior.state,
            BehaviorState::Dead | BehaviorState::Respawning
        )
    }

    /// Apply damage to this NPC.  Returns `true` if the NPC died.
    pub fn apply_damage(&mut self, amount: u32) -> bool {
        self.hp_current = self.hp_current.saturating_sub(amount);
        if self.hp_current == 0 && self.is_alive() {
            self.behavior.on_death();
            return true;
        }
        false
    }

    /// Restore hit points (e.g. after respawn).
    pub fn restore_hp(&mut self, amount: u32) {
        self.hp_current = (self.hp_current + amount).min(self.hp_max);
    }
}

// ─── Tick I/O ─────────────────────────────────────────────────────────────────

/// Per-tick inputs provided by the zone simulation to [`NpcRegistry::tick`].
///
/// All maps are keyed by `npc_id`.  Missing entries are treated as
/// "nothing nearby" / "nobody speaking".
#[derive(Debug, Default)]
pub struct NpcTickInput<'a> {
    /// Map of npc_id → (nearest_player_id, distance_in_voxels).
    ///
    /// Only includes the *single* nearest player within the NPC's aggro range.
    pub nearest_players: BTreeMap<&'a str, (&'a str, f32)>,

    /// Map of npc_id → player_id who initiated dialogue this tick.
    ///
    /// Only present when a `PlayerSpeak` or `player.interact` event addressed
    /// this NPC.
    pub speaking_to: BTreeMap<&'a str, (&'a str, &'a str)>, // npc_id → (player_id, text)

    /// Optional player context snapshot for dialogue building.
    ///
    /// Keyed by player_id; used when constructing [`NpcLlmRequest`]s.
    pub player_contexts: BTreeMap<&'a str, PlayerDialogueContext>,

    /// World context string (recent events) to inject into LLM prompts.
    pub world_context: &'a str,
}

// ─── NpcRegistry ─────────────────────────────────────────────────────────────

/// Per-zone registry of all NPC states.
///
/// Uses [`BTreeMap`] instead of `HashMap` to guarantee deterministic iteration
/// order — essential for the witness/quorum hash to match across peers.
#[derive(Debug, Clone, Default)]
pub struct NpcRegistry {
    /// npc_id → full NPC state.
    npcs: BTreeMap<String, NpcState>,
}

impl NpcRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    // ── CRUD ─────────────────────────────────────────────────────────────────

    /// Insert or replace an NPC.  Returns the previous state if one existed.
    pub fn insert(&mut self, state: NpcState) -> Option<NpcState> {
        self.npcs.insert(state.npc_id.clone(), state)
    }

    /// Borrow an NPC by id.
    pub fn get(&self, npc_id: &str) -> Option<&NpcState> {
        self.npcs.get(npc_id)
    }

    /// Mutably borrow an NPC by id.
    pub fn get_mut(&mut self, npc_id: &str) -> Option<&mut NpcState> {
        self.npcs.get_mut(npc_id)
    }

    /// Remove an NPC from the registry (e.g. on zone unload or permanent death).
    pub fn remove(&mut self, npc_id: &str) -> Option<NpcState> {
        self.npcs.remove(npc_id)
    }

    /// Number of NPCs in this registry.
    pub fn len(&self) -> usize {
        self.npcs.len()
    }

    /// True if no NPCs are registered.
    pub fn is_empty(&self) -> bool {
        self.npcs.is_empty()
    }

    /// Iterate over all NPCs in deterministic (alphabetical) order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &NpcState)> {
        self.npcs.iter().map(|(k, v)| (k.as_str(), v))
    }

    // ── Tick ──────────────────────────────────────────────────────────────────

    /// Drive all NPCs through the Layer 1 state machine for one tick.
    ///
    /// For each NPC, the behavior machine is advanced with:
    /// - its current HP fraction
    /// - the nearest player in its aggro range (if any)
    /// - whether a player is speaking to it this tick
    ///
    /// Any transition that should trigger a Layer 2 LLM call
    /// (i.e. `PlayerSpoke`) is assembled into a [`PendingDialogue`] and
    /// returned for the caller to enqueue in a [`DialogueQueue`].
    ///
    /// Callers should respect [`AiContext::can_speak`] before enqueuing:
    ///
    /// ```text
    /// for pending in registry.tick(&input, now_ms) {
    ///     if registry.get(&pending.npc_id)
    ///         .map(|s| s.ai_context.can_speak(now_ms))
    ///         .unwrap_or(false)
    ///     {
    ///         dialogue_queue.push(pending);
    ///     }
    /// }
    /// ```
    ///
    /// # Returns
    ///
    /// A [`Vec<PendingDialogue>`] of Layer-2 calls to enqueue this tick.
    pub fn tick(&mut self, input: &NpcTickInput<'_>, now_ms: u64) -> Vec<PendingDialogue> {
        let mut pending = Vec::new();

        for (npc_id, state) in &mut self.npcs {
            let npc_id_str = npc_id.as_str();

            let nearest = input.nearest_players.get(npc_id_str).copied();
            let speaking = input.speaking_to.get(npc_id_str);

            let transition = state.behavior.tick(
                state.hp_fraction(),
                nearest,
                speaking.map(|(pid, _)| *pid),
            );

            use super::behavior::BehaviorTransition;
            let trigger_opt: Option<NpcDialogueTrigger> = match transition {
                BehaviorTransition::PlayerSpoke { ref player_id } => {
                    let text = speaking
                        .map(|(_, t)| t.to_string())
                        .unwrap_or_default();
                    Some(NpcDialogueTrigger::PlayerSpeak {
                        player_id: player_id.clone(),
                        text,
                    })
                }
                BehaviorTransition::PlayerDetected { ref player_id } => {
                    // Friendly NPCs greet approaching players.
                    if state.behavior.config.faction == NpcFaction::Friendly {
                        Some(NpcDialogueTrigger::PlayerApproach {
                            player_id: player_id.clone(),
                            distance: nearest.map(|(_, d)| d).unwrap_or(0.0),
                        })
                    } else {
                        None
                    }
                }
                _ => None,
            };

            if let Some(trigger) = trigger_opt {
                // Only trigger Layer 2 if the cooldown allows it.
                if state.ai_context.can_speak(now_ms) {
                    let player_context = match &trigger {
                        NpcDialogueTrigger::PlayerSpeak { player_id, .. }
                        | NpcDialogueTrigger::PlayerApproach { player_id, .. } => {
                            input.player_contexts.get(player_id.as_str()).cloned()
                        }
                        _ => None,
                    };

                    let request = NpcLlmRequest::new(
                        npc_id_str,
                        trigger,
                        player_context,
                        input.world_context,
                        state.ai_context.clone(),
                    );
                    pending.push(PendingDialogue::new(request, now_ms));
                }
            }
        }

        pending
    }

    /// Convenience: tick and immediately enqueue eligible items into `queue`.
    ///
    /// Respects [`AiContext::can_speak`] — only NPCs past their cooldown are
    /// enqueued.  This is equivalent to calling [`tick`][Self::tick] and then
    /// filtering + pushing manually.
    pub fn tick_and_enqueue(
        &mut self,
        input: &NpcTickInput<'_>,
        queue: &mut DialogueQueue,
        now_ms: u64,
    ) {
        let pending = self.tick(input, now_ms);
        for p in pending {
            queue.push(p);
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::behavior::{BehaviorConfig, NpcFaction};
    use super::super::context::AiContext;
    use super::super::dialogue::DialogueQueue;

    fn hostile_state(id: &str) -> NpcState {
        NpcState::new(
            id,
            AiContext::new(id, "llama3", "You are hostile.", "attack"),
            BehaviorConfig { faction: NpcFaction::Hostile, aggro_range: 10.0, ..Default::default() },
            100,
            [0.0, 0.0, 0.0],
            "zone-a",
        )
    }

    fn friendly_state(id: &str) -> NpcState {
        NpcState::new(
            id,
            AiContext::new(id, "llama3", "You are friendly.", "help players"),
            BehaviorConfig { faction: NpcFaction::Friendly, ..Default::default() },
            100,
            [0.0, 0.0, 0.0],
            "zone-a",
        )
    }

    // ── NpcState ──────────────────────────────────────────────────────────────

    #[test]
    fn hp_fraction_full_health() {
        let state = hostile_state("n1");
        assert!((state.hp_fraction() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn apply_damage_kills_npc() {
        let mut state = hostile_state("n1");
        let died = state.apply_damage(100);
        assert!(died);
        assert!(!state.is_alive());
    }

    #[test]
    fn apply_damage_does_not_underflow() {
        let mut state = hostile_state("n1");
        state.apply_damage(200); // more than max hp
        assert_eq!(state.hp_current, 0);
    }

    #[test]
    fn restore_hp_capped_at_max() {
        let mut state = hostile_state("n1");
        state.hp_current = 50;
        state.restore_hp(200);
        assert_eq!(state.hp_current, state.hp_max);
    }

    // ── NpcRegistry CRUD ──────────────────────────────────────────────────────

    #[test]
    fn insert_and_get() {
        let mut reg = NpcRegistry::new();
        reg.insert(hostile_state("aldric"));
        assert!(reg.get("aldric").is_some());
    }

    #[test]
    fn remove_returns_state() {
        let mut reg = NpcRegistry::new();
        reg.insert(hostile_state("n1"));
        let removed = reg.remove("n1");
        assert!(removed.is_some());
        assert!(reg.is_empty());
    }

    #[test]
    fn iter_deterministic_order() {
        let mut reg = NpcRegistry::new();
        reg.insert(hostile_state("zzz"));
        reg.insert(hostile_state("aaa"));
        reg.insert(hostile_state("mmm"));
        let ids: Vec<&str> = reg.iter().map(|(id, _)| id).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted, "iter must be in alphabetical order for determinism");
    }

    // ── NpcRegistry tick ──────────────────────────────────────────────────────

    #[test]
    fn player_speak_triggers_pending_dialogue() {
        let mut reg = NpcRegistry::new();
        reg.insert(friendly_state("aldric"));

        let mut input = NpcTickInput::default();
        input.speaking_to.insert("aldric", ("p1", "Hello!"));

        let pending = reg.tick(&input, 0);
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].npc_id, "aldric");
        assert!(matches!(
            pending[0].request.trigger,
            NpcDialogueTrigger::PlayerSpeak { .. }
        ));
    }

    #[test]
    fn cooldown_suppresses_duplicate_triggers() {
        let mut reg = NpcRegistry::new();
        let mut state = friendly_state("aldric");
        state.ai_context.cooldown_ms = 5_000;
        // Simulate NPC spoke at t=0
        state.ai_context.mark_spoken(0);
        reg.insert(state);

        let mut input = NpcTickInput::default();
        input.speaking_to.insert("aldric", ("p1", "Hello again!"));

        // t=1000 — still within cooldown
        let pending = reg.tick(&input, 1_000);
        assert!(pending.is_empty(), "cooldown should suppress Layer 2 trigger");
    }

    #[test]
    fn friendly_npc_greets_approaching_player() {
        let mut reg = NpcRegistry::new();
        let state = friendly_state("shopkeeper");
        reg.insert(state);

        let mut input = NpcTickInput::default();
        input.nearest_players.insert("shopkeeper", ("p1", 5.0));

        let pending = reg.tick(&input, 0);
        assert_eq!(pending.len(), 1);
        assert!(matches!(
            pending[0].request.trigger,
            NpcDialogueTrigger::PlayerApproach { .. }
        ));
    }

    #[test]
    fn hostile_npc_does_not_greet_approaching_player() {
        let mut reg = NpcRegistry::new();
        reg.insert(hostile_state("guard"));

        let mut input = NpcTickInput::default();
        input.nearest_players.insert("guard", ("p1", 5.0));

        // Hostile NPCs enter Chase, not Speak — no dialogue trigger.
        let pending = reg.tick(&input, 0);
        assert!(pending.is_empty());
    }

    #[test]
    fn tick_and_enqueue_fills_queue() {
        let mut reg = NpcRegistry::new();
        reg.insert(friendly_state("merchant"));

        let mut input = NpcTickInput::default();
        input.speaking_to.insert("merchant", ("p1", "Do you have potions?"));

        let mut queue = DialogueQueue::new();
        reg.tick_and_enqueue(&input, &mut queue, 0);
        assert_eq!(queue.len(), 1);
    }
}
