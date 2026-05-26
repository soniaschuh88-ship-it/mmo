//! Shared simulation state behind an async mutex.

use std::sync::Arc;

use bifrost_chunk::{ChunkRegistry, PeerId};
use bifrost_lockstep::LockstepScheduler;
use bifrost_physics::PhysicsWorld;
use bifrost_witness::WitnessExecutor;
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
}

impl SimState {
    pub fn new() -> Self {
        Self {
            scheduler:      LockstepScheduler::new(50),
            world:          PhysicsWorld::new(),
            chunk_registry: ChunkRegistry::with_default_epoch_duration(),
            witness:        None,
            peers:          Vec::new(),
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
