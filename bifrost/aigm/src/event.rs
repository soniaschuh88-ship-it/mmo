//! # WorldEvent — the universal NOVA event ledger entry
//!
//! Every state change in the NOVA engine is expressed as a `WorldEvent`
//! appended to the immutable ledger.  The current world state is always:
//!
//! ```text
//! World = fold(events, ∅)
//! ```
//!
//! ## Integrity chain
//!
//! Each event carries a `world_hash` which is:
//!
//! ```text
//! world_hash[N] = BLAKE3(world_hash[N-1] || event_hash[N])
//! event_hash[N] = BLAKE3(seq || type_tag || author || zone_id || ts || payload_bytes)
//! ```
//!
//! This makes the ledger **tamper-evident**: any mutation of a past event
//! breaks every subsequent hash in the chain.

use serde::{Deserialize, Serialize};

use bifrost_kernel::{RawEvent, SequencedInstant};

// ─── Author identity ────────────────────────────────────────────────────────

/// Who produced this event.
///
/// Serialises as a plain string tag so the ledger is human-readable:
/// `"player:uuid"`, `"npc:aldric"`, `"aigm"`, `"system"`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum AuthorId {
    Player(String),
    Npc(String),
    AiGm,
    System,
}

impl std::fmt::Display for AuthorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthorId::Player(id) => write!(f, "player:{id}"),
            AuthorId::Npc(id)    => write!(f, "npc:{id}"),
            AuthorId::AiGm       => write!(f, "aigm"),
            AuthorId::System     => write!(f, "system"),
        }
    }
}

// ─── Event type catalogue ────────────────────────────────────────────────────

/// All event types known to the NOVA engine.
///
/// New variants must be added here **and** to [`EventPayload`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    // ── Voxel world ─────────────────────────────────────────────────────────
    VoxelSet,
    VoxelBatch,
    VoxelExplosion,

    // ── Entity lifecycle ─────────────────────────────────────────────────────
    EntitySpawn,
    EntityDespawn,
    EntityMove,
    EntityTeleport,

    // ── Combat ───────────────────────────────────────────────────────────────
    CombatAttack,
    CombatDamage,
    CombatDeath,
    CombatResurrect,
    CombatStatusApply,
    CombatStatusRemove,

    // ── AI Game Master ────────────────────────────────────────────────────────
    AigmQuestCreate,
    AigmQuestUpdate,
    AigmQuestComplete,
    AigmQuestFail,
    AigmEventWorld,     // weather, disasters, invasions, …
    AigmNpcSpeak,
    AigmNpcGoalSet,
    AigmStoryBeat,

    // ── Player ───────────────────────────────────────────────────────────────
    PlayerJoin,
    PlayerLeave,
    PlayerInput,
    PlayerInventoryChange,
    PlayerLevelUp,
    PlayerReputationChange,
    PlayerSpeak,

    // ── Economy ──────────────────────────────────────────────────────────────
    EconomyTrade,
    EconomyLootDrop,
    EconomyLootPickup,

    // ── Zone ─────────────────────────────────────────────────────────────────
    ZoneLoad,
    ZoneUnload,
    ZoneAuthorityTransfer,
}

// ─── Typed event payloads ────────────────────────────────────────────────────

/// Payload variants — one per [`EventType`].
///
/// Uses an untagged enum so JSON payloads are compact (no extra wrapper key).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EventPayload {
    VoxelSet(VoxelSetPayload),
    VoxelBatch(VoxelBatchPayload),
    VoxelExplosion(VoxelExplosionPayload),
    EntitySpawn(EntitySpawnPayload),
    EntityDespawn(EntityDespawnPayload),
    EntityMove(EntityMovePayload),
    EntityTeleport(EntityTeleportPayload),
    CombatAttack(CombatAttackPayload),
    CombatDamage(CombatDamagePayload),
    CombatDeath(CombatDeathPayload),
    CombatStatusApply(CombatStatusPayload),
    CombatStatusRemove(CombatStatusPayload),
    AigmQuestCreate(QuestCreatePayload),
    AigmQuestUpdate(QuestUpdatePayload),
    AigmQuestComplete(QuestOutcomePayload),
    AigmQuestFail(QuestOutcomePayload),
    AigmEventWorld(WorldEventPayload),
    AigmNpcSpeak(NpcSpeakPayload),
    AigmNpcGoalSet(NpcGoalPayload),
    AigmStoryBeat(StoryBeatPayload),
    PlayerJoin(PlayerJoinPayload),
    PlayerLeave(PlayerLeavePayload),
    PlayerLevelUp(PlayerLevelUpPayload),
    PlayerReputationChange(ReputationChangePayload),
    PlayerSpeak(PlayerSpeakPayload),
    EconomyTrade(TradePayload),
    EconomyLootDrop(LootPayload),
    EconomyLootPickup(LootPayload),
    ZoneLoad(ZonePayload),
    ZoneUnload(ZonePayload),
    ZoneAuthorityTransfer(ZoneAuthorityPayload),
    /// Generic raw payload for events that carry no structured data (e.g.
    /// `PlayerInput`, `EntityMove` when only seq tracking is needed).
    Raw(serde_json::Value),
}

// ─── Individual payload structs ───────────────────────────────────────────────

/// A single voxel mutation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoxelSetPayload {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    /// Material palette index (0 = air).
    pub material: u16,
    /// Previous material — required for deterministic rollback.
    pub prev: u16,
}

/// Bulk terrain mutation (e.g. terrain generation, cave-in).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoxelBatchPayload {
    pub zone_id: String,
    /// Packed list of (x, y, z, material, prev) tuples.
    pub mutations: Vec<VoxelSetPayload>,
}

/// Radial voxel destruction centred on a point.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoxelExplosionPayload {
    pub cx: i32,
    pub cy: i32,
    pub cz: i32,
    /// Radius in voxels.
    pub radius: u8,
    /// Damage applied to each voxel within radius.
    pub damage: u16,
}

// R1 — One concept, one crate: Vec3 is defined once in nova-core.
// Using f32 throughout (WebGPU / lockstep compatibility).
pub type Vec3Payload = nova_core::Vec3;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntitySpawnPayload {
    pub entity_id: String,
    pub entity_type: String,   // "player" | "npc" | "mob" | "item"
    pub position: Vec3Payload,
    pub zone_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityDespawnPayload {
    pub entity_id: String,
    pub reason: String,        // "death" | "logout" | "zone_unload"
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityMovePayload {
    pub entity_id: String,
    pub from: Vec3Payload,
    pub to: Vec3Payload,
    pub velocity: Vec3Payload,
    pub tick: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityTeleportPayload {
    pub entity_id: String,
    pub to: Vec3Payload,
    pub to_zone_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CombatAttackPayload {
    pub attacker_id: String,
    pub target_id: String,
    pub weapon: String,
    pub tick: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CombatDamagePayload {
    pub target_id: String,
    pub source_id: String,
    pub amount: u32,
    pub damage_type: String,   // "physical" | "fire" | "poison" | …
    pub remaining_hp: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CombatDeathPayload {
    pub entity_id: String,
    pub killer_id: Option<String>,
    pub position: Vec3Payload,
    pub loot_event_seq: Option<u64>,  // seq of the accompanying EconomyLootDrop
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CombatStatusPayload {
    pub target_id: String,
    pub status: String,        // "burning" | "poisoned" | "stunned" | …
    pub duration_ticks: u32,
    pub source_id: String,
}

/// AI GM — new quest created.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuestCreatePayload {
    pub quest_id: String,
    pub title: String,
    pub description: String,
    pub giver_npc_id: String,
    pub target_ids: Vec<String>,
    pub objectives: Vec<QuestObjectivePayload>,
    pub reward: QuestRewardPayload,
    /// Unix-ms deadline; `None` = no expiry.
    pub expires_at: Option<u64>,
    /// AI reasoning trace (debug / audit only, never shown to players).
    pub ai_context: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuestObjectivePayload {
    pub objective_id: String,
    pub kind: String,          // "kill" | "collect" | "explore" | "speak" | "build"
    pub description: String,
    pub target_id: Option<String>,
    pub required_count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuestRewardPayload {
    pub xp: u32,
    pub gold: u32,
    pub items: Vec<String>,
    pub reputation: Vec<ReputationChangePayload>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuestUpdatePayload {
    pub quest_id: String,
    pub objective_id: String,
    pub player_id: String,
    pub progress: u32,
    pub required: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuestOutcomePayload {
    pub quest_id: String,
    pub player_id: String,
    pub reason: Option<String>,
}

/// AI GM — world event (weather, invasion, disaster, festival …).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldEventPayload {
    pub event_name: String,
    pub description: String,
    pub affected_zones: Vec<String>,
    /// Voxel mutations this world event causes (terrain changes, structure
    /// spawns, …).  May be empty; large mutations use `VoxelBatch` instead.
    pub voxel_consequences: Vec<VoxelSetPayload>,
    pub duration_ticks: Option<u32>,
    pub ai_context: String,
}

/// NPC dialogue line written to the ledger.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NpcSpeakPayload {
    pub npc_id: String,
    pub dialogue: String,
    pub emotion: String,       // "neutral" | "anxious" | "angry" | "joyful" | …
    /// Player this line is addressed to, if any.
    pub addressed_to: Option<String>,
    /// LLM provider used ("ollama/llama3-8b", "openrouter/mistral-7b", …).
    pub model_used: Option<String>,
    /// BLAKE3 of the prompt used — allows deduplication / cache hits.
    pub prompt_hash: Option<String>,
}

/// NPC goal update from the AI GM.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NpcGoalPayload {
    pub npc_id: String,
    pub previous_goal: String,
    pub new_goal: String,
    pub reason: String,
}

/// Narrative beat emitted by the story engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoryBeatPayload {
    pub beat_id: String,
    pub arc_id: String,
    pub title: String,
    pub description: String,
    pub affected_zones: Vec<String>,
    /// World consequences (may trigger follow-up events).
    pub consequences: Vec<StoryConsequence>,
    pub ai_context: String,
}

/// A single causal consequence of a story beat.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StoryConsequence {
    SpawnNpc  { npc_template_id: String, zone_id: String, position: Vec3Payload },
    DespawnNpc { npc_id: String },
    CreateQuest { quest_template_id: String },
    MutateVoxels { description: String },
    ChangeWorldMood { new_mood: String },
    UnlockZone { zone_id: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerJoinPayload {
    pub player_id: String,
    pub player_name: String,
    pub zone_id: String,
    pub position: Vec3Payload,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerLeavePayload {
    pub player_id: String,
    pub reason: String,        // "logout" | "disconnect" | "kicked"
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerLevelUpPayload {
    pub player_id: String,
    pub new_level: u32,
    pub class: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReputationChangePayload {
    pub faction_id: String,
    pub delta: i32,
    pub reason: String,
}

/// A player chat message — feeds into NPC AI context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerSpeakPayload {
    pub player_id: String,
    pub text: String,
    /// NPC the player is speaking to, if targeted.
    pub target_npc_id: Option<String>,
    pub zone_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradePayload {
    pub buyer_id: String,
    pub seller_id: String,
    pub item_id: String,
    pub quantity: u32,
    pub gold: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LootPayload {
    pub item_id: String,
    pub quantity: u32,
    pub position: Vec3Payload,
    pub zone_id: String,
    pub owner_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZonePayload {
    pub zone_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZoneAuthorityPayload {
    pub zone_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub at_seq: u64,
}

// ─── WorldEvent ───────────────────────────────────────────────────────────────

/// The universal NOVA event ledger entry.
///
/// Append-only. Never mutated after creation.
///
/// ## R5 — No SystemTime
///
/// `ts_ms` is a wall-clock audit field only — it is **never** included in
/// `event_hash()` or any hash input.  `instant` is the authoritative ordering
/// reference; it is assigned by the zone [`EventPipeline`] via [`RawEvent`].
///
/// [`EventPipeline`]: bifrost_kernel::EventPipeline
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldEvent {
    /// Pipeline-assigned logical position (R5: replaces raw seq/timestamp).
    pub instant: SequencedInstant,
    /// Discriminant — used for routing and budget accounting.
    pub event_type: EventType,
    /// Typed payload.
    pub payload: EventPayload,
    /// Who produced this event.
    pub author: AuthorId,
    /// BLAKE3 integrity chain link:
    /// `world_hash[N] = BLAKE3(world_hash[N-1] || event_hash[N])`
    pub world_hash: [u8; 32],
    /// Zone this event belongs to.
    pub zone_id: String,
    /// Wall-clock timestamp (unix ms) — **informational / audit only**.
    /// Never included in hash computations (R5).
    pub ts_ms: u64,
}

impl WorldEvent {
    /// Compute this event's own hash:
    ///
    /// ```text
    /// BLAKE3(tick_le8 || seq_le8 || type_tag || author_str || zone_id || payload_json)
    /// ```
    ///
    /// `ts_ms` is intentionally excluded — it is wall-clock time and would
    /// make the chain non-deterministic (R5 violation).
    pub fn event_hash(&self) -> [u8; 32] {
        let mut h = blake3::Hasher::new();
        h.update(&self.instant.tick.to_le_bytes());
        h.update(&self.instant.seq.to_le_bytes());
        h.update(self.event_type_tag().as_bytes());
        h.update(self.author.to_string().as_bytes());
        h.update(self.zone_id.as_bytes());
        // Payload as canonical JSON — deterministic for the same value.
        let payload_bytes = serde_json::to_vec(&self.payload)
            .unwrap_or_default();
        h.update(&payload_bytes);
        *h.finalize().as_bytes()
    }

    /// Advance the BLAKE3 integrity chain given the previous hash.
    ///
    /// ```text
    /// new_world_hash = BLAKE3(prev_world_hash || event_hash)
    /// ```
    pub fn compute_world_hash(prev: &[u8; 32], event_hash: &[u8; 32]) -> [u8; 32] {
        let mut h = blake3::Hasher::new();
        h.update(prev);
        h.update(event_hash);
        *h.finalize().as_bytes()
    }

    /// Build a new event and advance the chain in one call.
    ///
    /// `instant` is the `SequencedInstant` assigned by the caller's
    /// [`EventPipeline`]; `ts_ms` is a wall-clock audit field only and is
    /// **not** included in the hash (R5).
    ///
    /// [`EventPipeline`]: bifrost_kernel::EventPipeline
    pub fn new(
        instant: SequencedInstant,
        event_type: EventType,
        payload: EventPayload,
        author: AuthorId,
        prev_world_hash: &[u8; 32],
        zone_id: impl Into<String>,
        ts_ms: u64,
    ) -> Self {
        let zone_id = zone_id.into();
        // Compute event hash — ts_ms intentionally excluded (R5).
        let mut h = blake3::Hasher::new();
        h.update(&instant.tick.to_le_bytes());
        h.update(&instant.seq.to_le_bytes());
        let type_tag = Self::type_tag_for(event_type);
        h.update(type_tag.as_bytes());
        h.update(author.to_string().as_bytes());
        h.update(zone_id.as_bytes());
        let payload_bytes = serde_json::to_vec(&payload).unwrap_or_default();
        h.update(&payload_bytes);
        let event_hash = *h.finalize().as_bytes();

        let world_hash = Self::compute_world_hash(prev_world_hash, &event_hash);

        WorldEvent { instant, event_type, payload, author, world_hash, zone_id, ts_ms }
    }

    /// Verify this event's `world_hash` against a known previous hash.
    pub fn verify_chain(&self, prev_world_hash: &[u8; 32]) -> bool {
        let eh = self.event_hash();
        let expected = Self::compute_world_hash(prev_world_hash, &eh);
        expected == self.world_hash
    }

    fn event_type_tag(&self) -> String {
        Self::type_tag_for(self.event_type)
    }

    fn type_tag_for(t: EventType) -> String {
        // Use the serde rename (snake_case) as the canonical string tag.
        serde_json::to_value(t)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_else(|| format!("{t:?}"))
    }
}

// ─── RawEvent (bifrost-kernel R3 integration) ────────────────────────────────

/// R3 — EventPipeline required.
///
/// Implementing [`RawEvent`] lets [`EventPipeline`] stamp `WorldEvent` with
/// a monotonic [`SequencedInstant`] and advance the BLAKE3 chain.
///
/// [`EventPipeline`]: bifrost_kernel::EventPipeline
impl RawEvent for WorldEvent {
    fn zone_id(&self) -> &str { &self.zone_id }
    fn content_hash(&self) -> [u8; 32] { self.event_hash() }
    fn set_instant(&mut self, instant: SequencedInstant) { self.instant = instant; }
    fn set_world_hash(&mut self, hash: [u8; 32]) { self.world_hash = hash; }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn genesis_hash() -> [u8; 32] {
        [0u8; 32]
    }

    fn make_voxel_event(seq: u64, prev: &[u8; 32]) -> WorldEvent {
        WorldEvent::new(
            SequencedInstant::new(0, seq),
            EventType::VoxelSet,
            EventPayload::VoxelSet(VoxelSetPayload { x: 1, y: 2, z: 3, material: 5, prev: 0 }),
            AuthorId::Player("player-1".into()),
            prev,
            "zone-overworld",
            1_700_000_000_000,
        )
    }

    #[test]
    fn chain_verifies() {
        let genesis = genesis_hash();
        let e = make_voxel_event(0, &genesis);
        assert!(e.verify_chain(&genesis));
    }

    #[test]
    fn chain_breaks_on_tamper() {
        let genesis = genesis_hash();
        let mut e = make_voxel_event(0, &genesis);
        // Tamper with the payload after construction
        e.payload = EventPayload::VoxelSet(VoxelSetPayload { x: 99, y: 0, z: 0, material: 1, prev: 0 });
        // Hash check should now fail
        assert!(!e.verify_chain(&genesis));
    }

    #[test]
    fn chain_links_correctly() {
        let genesis = genesis_hash();
        let e0 = make_voxel_event(0, &genesis);
        let e1 = make_voxel_event(1, &e0.world_hash);
        assert!(e1.verify_chain(&e0.world_hash));
        // e1 does NOT verify against genesis
        assert!(!e1.verify_chain(&genesis));
    }

    #[test]
    fn author_display() {
        assert_eq!(AuthorId::Player("abc".into()).to_string(), "player:abc");
        assert_eq!(AuthorId::Npc("aldric".into()).to_string(),  "npc:aldric");
        assert_eq!(AuthorId::AiGm.to_string(),                  "aigm");
        assert_eq!(AuthorId::System.to_string(),                 "system");
    }

    #[test]
    fn serialise_round_trip() {
        let genesis = genesis_hash();
        let e = make_voxel_event(42, &genesis);
        let json = serde_json::to_string(&e).unwrap();
        let back: WorldEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(e, back);
    }
}
