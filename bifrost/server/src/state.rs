//! Shared simulation state behind an async mutex.
//!
//! ## Core Principles compliance
//!
//! | Rule | Implementation |
//! |---|---|
//! | R2 — single mutation path | [`SimState`] implements [`ApplyTransition`] |
//! | R3 — EventPipeline required | `pipelines: BTreeMap<ZoneId, EventPipeline>` |
//! | R4 — replay-safe | `ledgers: BTreeMap<ZoneId, Ledger<WorldEvent>>` |

use std::sync::Arc;

use bifrost_chunk::{ChunkRegistry, PeerId};
use bifrost_kernel::{ApplyTransition, EventPipeline, Ledger};
use bifrost_lockstep::LockstepScheduler;
use bifrost_physics::PhysicsWorld;
use bifrost_witness::WitnessExecutor;
use bifrost_wac::{AssetCache, WorldDirector, canonicalize_biome_id};
use bifrost_run::WorldRunDirector;
use bifrost_synthesis::AiFaction;
use bifrost_safe_city::{SafeCity, Zone, ZoneId, ZoneState, ResourceMap};
use bifrost_aigm::{NpcRegistry, QuestRegistry, WorldEvent};
use bifrost_aigm::npc::context::AiContext;
use bifrost_aigm::npc::behavior::{BehaviorConfig, NpcFaction};
use bifrost_aigm::npc::registry::NpcState;
use bifrost_kernel::bridge::RuntimeAdapter;
use std::collections::BTreeMap;
use tokio::sync::Mutex;

/// The live simulation state shared across all HTTP handlers.
pub struct SimState {
    pub scheduler:      LockstepScheduler,
    pub world:          PhysicsWorld,
    /// Spatial chunk authority registry (Phase 2 — not yet wired to handlers).
    #[allow(dead_code)]
    pub chunk_registry: ChunkRegistry,
    /// Witness executor — None until at least 3 peers are registered
    /// and `/witness/setup` has been called.
    pub witness:        Option<WitnessExecutor>,
    /// Ordered list of registered peers (for display / authority rotation).
    pub peers:          Vec<PeerId>,

    // ── WAC + World Director ──────────────────────────────────────────────────
    /// BLAKE3-keyed compiled asset cache (per server instance).
    pub asset_cache:    AssetCache,
    /// World Director — reads pressure graph, emits AssetBlueprints.
    pub director:       WorldDirector,

    // ── Nexus Voxel Kernel ────────────────────────────────────────────────────
    /// Nexus WAC runtime: LLM/AI → VoxelChunk pipeline.
    pub nexus_rt:       RuntimeAdapter,

    // ── Run System (bifrost-run) ──────────────────────────────────────────────
    /// Run director: tracks win conditions and emits world-mutation blueprints.
    /// Active run is available via `run_director.runs` / `active_run()`.
    pub run_director:   WorldRunDirector,

    // ── Synthesis AI (bifrost-synthesis) ─────────────────────────────────────
    /// The active Synthesis AI faction (one per server for now).
    pub synthesis:      Option<AiFaction>,

    // ── Safe City + Economy (bifrost-safe-city) ───────────────────────────────
    /// The safe city anchor zone (contains the embedded AuctionHouse market).
    pub safe_city:      SafeCity,
    /// All active world zones keyed by zone ID.
    pub zones:          BTreeMap<ZoneId, Zone>,

    // ── R3 — EventPipeline per zone (required for all state changes) ──────────
    /// One [`EventPipeline`] per zone.
    ///
    /// Every world-state-changing event MUST be stamped by the pipeline
    /// for that zone before being appended to its ledger.
    pub pipelines:      BTreeMap<String, EventPipeline>,

    // ── R4 — Replay-safe ledger per zone ──────────────────────────────────────
    /// Append-only event log.  World state = fold(ledger.events, ∅, reducer).
    pub ledgers:        BTreeMap<String, Ledger<WorldEvent>>,

    // ── AI Game Master registries (bifrost-aigm) ──────────────────────────────
    /// Quest state, rebuilt deterministically from ledger on startup.
    pub quest_registry: QuestRegistry,
    /// NPC state and 3-layer AI behaviour.
    pub npc_registry:   NpcRegistry,
}

impl SimState {
    pub fn new() -> Self {
        // Seed a few starting zones so the API has data immediately.
        let mut zones: BTreeMap<ZoneId, Zone> = BTreeMap::new();

        // Seed starting zones — biome IDs canonicalized via bifrost-wac::BiomeKey.
        zones.insert("safe-city".into(), Zone::safe("safe-city",
            canonicalize_biome_id("safe-city"))); // → "village"

        for (id, legacy_biome, risk) in [
            ("outer-east", "forest",  1u8),  // → "dark_forest"
            ("outer-west", "desert",  1),    // → "sand"
            ("deep-north", "dungeon", 2),    // already canonical
        ] {
            let biome = canonicalize_biome_id(legacy_biome);
            zones.insert(id.into(), Zone {
                id:        id.into(),
                state:     ZoneState::Contested { leader: None, contest_strength: 0.0 },
                biome_id:  biome.into(),
                resources: ResourceMap::new(),
                influence: BTreeMap::new(),
                risk_tier: risk,
            });
        }

        // R3: one EventPipeline per zone, starting from tick 0.
        let zone_ids = ["safe-city", "outer-east", "outer-west", "deep-north"];
        let genesis  = [0u8; 32];
        let mut pipelines = BTreeMap::new();
        let mut ledgers   = BTreeMap::new();
        for zone_id in zone_ids {
            pipelines.insert(zone_id.into(), EventPipeline::new(zone_id, 0, genesis));
            ledgers.insert(zone_id.into(), Ledger::new(zone_id, genesis));
        }

        // Seed the canonical village NPCs that game.html displays.
        // Positions are zone-relative voxel coords (world center = [32,0,32]).
        // system_prompt encodes "DisplayName|goal" so the API can return both.
        let mut npc_registry = NpcRegistry::new();
        let friendly_cfg = BehaviorConfig { faction: NpcFaction::Friendly, ..Default::default() };
        for (id, name, goal, pos) in [
            ("guard",   "Guard Captain Aldric", "protect the village gates",       [30.5_f32, 0.0, 29.5]),
            ("innkeep", "Innkeeper Bram",        "run the inn and serve travelers",  [33.5, 0.0, 30.5]),
            ("elder",   "Elder Mirova",           "safeguard village lore",          [32.5, 0.0, 35.5]),
            ("wizard",  "Wizard Seraphon",        "study the dungeon's power",       [34.5, 0.0, 34.5]),
            ("smith",   "Blacksmith Helga",       "forge weapons for adventurers",   [29.5, 0.0, 34.5]),
            ("healer",  "Healer Lyris",            "heal the wounded",               [32.5, 0.0, 28.5]),
        ] {
            // Store "Name|goal" in system_prompt so the API can surface both.
            let prompt = format!("{name}|{goal}");
            let ctx = AiContext::new(id, "llama3", prompt, goal);
            let npc = NpcState::new(id, ctx, friendly_cfg.clone(), 200, pos, "safe-city");
            npc_registry.insert(npc);
        }

        Self {
            scheduler:      LockstepScheduler::new(50),
            world:          PhysicsWorld::new(),
            chunk_registry: ChunkRegistry::with_default_epoch_duration(),
            witness:        None,
            peers:          Vec::new(),
            asset_cache:    AssetCache::new(),
            director:       WorldDirector::default(),
            nexus_rt:       RuntimeAdapter::new(),
            run_director:   WorldRunDirector::new(),
            synthesis:      None,
            safe_city:      SafeCity::new(String::from("safe-city")),
            zones,
            pipelines,
            ledgers,
            quest_registry: QuestRegistry::new(),
            npc_registry,
        }
    }

    /// Get the pipeline for a zone, or the safe-city pipeline as fallback.
    pub fn pipeline_for(&mut self, zone_id: &str) -> &mut EventPipeline {
        if !self.pipelines.contains_key(zone_id) {
            let genesis = [0u8; 32];
            self.pipelines.insert(zone_id.into(), EventPipeline::new(zone_id, 0, genesis));
            self.ledgers.insert(zone_id.into(), Ledger::new(zone_id, genesis));
        }
        self.pipelines.get_mut(zone_id).unwrap()
    }

    /// Process a WorldEvent through the zone pipeline and append to the ledger.
    ///
    /// R3: all state changes must go through EventPipeline.
    /// R4: all events are appended to the append-only ledger.
    pub fn emit(&mut self, mut event: WorldEvent) -> Result<(), String> {
        use bifrost_kernel::RawEvent;
        let zone_id = event.zone_id().to_string();
        let pipeline = self.pipeline_for(&zone_id);
        event = pipeline.process(event).map_err(|e| e.to_string())?;
        // Apply event to projections.
        let _ = self.quest_registry.apply_event(&event);
        // Append to ledger (R4).
        use bifrost_kernel::ledger::LedgerEntry;  // internal module path
        let entry = LedgerEntry {
            instant:    event.instant,
            world_hash: event.world_hash,
            event:      event.clone(),
        };
        if let Some(ledger) = self.ledgers.get_mut(&zone_id) {
            let _ = ledger.append(entry);
        }
        Ok(())
    }

    /// Parse a 64-character hex string into a PeerId.
    pub fn parse_peer_id(hex: &str) -> Result<PeerId, String> {
        let bytes = hex::decode(hex)
            .map_err(|_| format!("invalid hex: {hex}"))?;
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| "peer_id must be 32 bytes (64 hex chars)".to_string())?;
        Ok(PeerId(arr))
    }
}

/// R2 — Single mutation path.
///
/// All state changes to [`SimState`] MUST use `state.apply(transition_fn)`.
/// Direct field mutation outside constructors is prohibited.
impl ApplyTransition for SimState {}

impl Default for SimState {
    fn default() -> Self {
        Self::new()
    }
}

pub type SharedState = Arc<Mutex<SimState>>;

pub fn new_shared() -> SharedState {
    Arc::new(Mutex::new(SimState::new()))
}
