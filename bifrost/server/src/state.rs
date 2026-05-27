//! Shared simulation state behind an async mutex.

use std::sync::Arc;

use bifrost_chunk::{ChunkRegistry, PeerId};
use bifrost_lockstep::LockstepScheduler;
use bifrost_physics::PhysicsWorld;
use bifrost_witness::WitnessExecutor;
use bifrost_wac::{AssetCache, WorldDirector, canonicalize_biome_id};
use bifrost_run::WorldRunDirector;
use bifrost_synthesis::AiFaction;
use bifrost_safe_city::{SafeCity, Zone, ZoneId, ZoneState, ResourceMap};
use nexus_voxel_kernel::bridge::RuntimeAdapter;
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
        }
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

impl Default for SimState {
    fn default() -> Self {
        Self::new()
    }
}

pub type SharedState = Arc<Mutex<SimState>>;

pub fn new_shared() -> SharedState {
    Arc::new(Mutex::new(SimState::new()))
}
