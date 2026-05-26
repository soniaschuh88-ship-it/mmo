//! ChunkRegistry — manages all chunk authority assignments.
//!
//! The registry tracks which peer holds authority over each chunk and drives
//! epoch rotation when an epoch expires. Authority rotation uses a deterministic
//! round-robin over the provided peer pool.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::authority::ChunkAuthority;
use crate::coord::ChunkId;
use crate::epoch::EpochBoundary;
use crate::peer::PeerId;

const DEFAULT_EPOCH_DURATION: u64 = 1_000;

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("chunk {0} not found in registry")]
    ChunkNotFound(String),
    #[error("peer pool must have at least 3 peers (authority + 2 witnesses)")]
    InsufficientPeers,
    #[error("chunk {0} epoch is not expired at tick {1}")]
    EpochNotExpired(String, u64),
}

/// Central registry for all chunk authority assignments.
///
/// Internally uses `BTreeMap` for deterministic iteration order — never `HashMap`.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ChunkRegistry {
    /// Current authority assignment per chunk.
    chunks: BTreeMap<ChunkId, ChunkAuthority>,

    /// Completed epoch boundaries, used as replay anchors.
    boundaries: Vec<EpochBoundary>,

    /// Default epoch duration in ticks for new assignments.
    pub epoch_duration_ticks: u64,
}

impl ChunkRegistry {
    pub fn new(epoch_duration_ticks: u64) -> Self {
        Self {
            chunks: BTreeMap::new(),
            boundaries: Vec::new(),
            epoch_duration_ticks,
        }
    }

    pub fn with_default_epoch_duration() -> Self {
        Self::new(DEFAULT_EPOCH_DURATION)
    }

    /// Assign initial authority for a chunk.
    ///
    /// Requires at least 3 peers: 1 authority + 2 witnesses.
    /// Authority = peers[0], Witnesses = [peers[1], peers[2]].
    pub fn assign(
        &mut self,
        chunk_id: ChunkId,
        peers: &[PeerId],
        start_tick: u64,
        state_hash: [u8; 32],
    ) -> Result<&ChunkAuthority, RegistryError> {
        if peers.len() < 3 {
            return Err(RegistryError::InsufficientPeers);
        }
        let authority = ChunkAuthority::new(
            chunk_id,
            peers[0],
            [peers[1], peers[2]],
            0,
            start_tick,
            self.epoch_duration_ticks,
            state_hash,
        );
        self.chunks.insert(chunk_id, authority);
        Ok(self.chunks.get(&chunk_id).unwrap())
    }

    /// Rotate the authority for a chunk to the next epoch.
    ///
    /// Uses deterministic round-robin: new_authority = peers[epoch % peers.len()].
    /// Witnesses are the next two peers after the authority.
    ///
    /// Returns the `EpochBoundary` (unsigned — caller must sign with outgoing key).
    pub fn rotate_epoch(
        &mut self,
        chunk_id: &ChunkId,
        peers: &[PeerId],
        current_tick: u64,
        final_state_hash: [u8; 32],
    ) -> Result<EpochBoundary, RegistryError> {
        if peers.len() < 3 {
            return Err(RegistryError::InsufficientPeers);
        }
        let current = self.chunks.get(chunk_id)
            .ok_or_else(|| RegistryError::ChunkNotFound(chunk_id.to_string()))?;

        if !current.is_expired(current_tick) {
            return Err(RegistryError::EpochNotExpired(chunk_id.to_string(), current_tick));
        }

        let old_epoch       = current.epoch;
        let outgoing        = current.authority;
        let new_epoch       = old_epoch + 1;

        // Deterministic round-robin rotation
        let n       = peers.len();
        let auth_i  = (new_epoch as usize) % n;
        let wit0_i  = (auth_i + 1) % n;
        let wit1_i  = (auth_i + 2) % n;

        let incoming = peers[auth_i];
        let boundary = EpochBoundary::unsigned(
            *chunk_id,
            old_epoch,
            outgoing,
            incoming,
            final_state_hash,
            current_tick,
        );

        // Advance the registry state
        let new_authority = ChunkAuthority::new(
            *chunk_id,
            peers[auth_i],
            [peers[wit0_i], peers[wit1_i]],
            new_epoch,
            current_tick,
            self.epoch_duration_ticks,
            final_state_hash,
        );
        self.chunks.insert(*chunk_id, new_authority);
        self.boundaries.push(boundary.clone());

        Ok(boundary)
    }

    /// Get the current authority for a chunk.
    pub fn get(&self, chunk_id: &ChunkId) -> Option<&ChunkAuthority> {
        self.chunks.get(chunk_id)
    }

    /// True if the chunk's current epoch has expired.
    pub fn is_expired(&self, chunk_id: &ChunkId, current_tick: u64) -> bool {
        self.chunks.get(chunk_id)
            .map(|a| a.is_expired(current_tick))
            .unwrap_or(false)
    }

    /// All chunks with expired epochs at `current_tick`.
    pub fn expired_chunks(&self, current_tick: u64) -> Vec<ChunkId> {
        self.chunks
            .iter()
            .filter(|(_, a)| a.is_expired(current_tick))
            .map(|(id, _)| *id)
            .collect()
    }

    /// The most recent `EpochBoundary` for a chunk, if any.
    pub fn last_boundary(&self, chunk_id: &ChunkId) -> Option<&EpochBoundary> {
        self.boundaries.iter().rev()
            .find(|b| &b.chunk_id == chunk_id)
    }

    /// Total number of tracked chunks.
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Total epoch boundaries recorded.
    pub fn boundary_count(&self) -> usize {
        self.boundaries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coord::ChunkCoord;
    use crate::peer::test_peer;

    fn peers(n: u8) -> Vec<PeerId> {
        (0..n).map(test_peer).collect()
    }

    fn chunk_zero() -> ChunkId {
        ChunkId::full(ChunkCoord::new(0, 0, 0))
    }

    #[test]
    fn assign_and_get() {
        let mut reg = ChunkRegistry::with_default_epoch_duration();
        reg.assign(chunk_zero(), &peers(4), 0, [0u8; 32]).unwrap();
        let auth = reg.get(&chunk_zero()).unwrap();
        assert_eq!(auth.epoch, 0);
        assert_eq!(auth.authority, test_peer(0));
    }

    #[test]
    fn rotate_epoch_deterministic() {
        let mut reg = ChunkRegistry::new(100);
        reg.assign(chunk_zero(), &peers(5), 0, [0u8; 32]).unwrap();

        // Epoch 0 expires at tick 100
        let boundary = reg.rotate_epoch(&chunk_zero(), &peers(5), 100, [1u8; 32]).unwrap();
        assert_eq!(boundary.epoch_number, 0);
        assert_eq!(boundary.outgoing_authority, test_peer(0));

        let auth = reg.get(&chunk_zero()).unwrap();
        assert_eq!(auth.epoch, 1);
        // Epoch 1: authority = peers[1 % 5] = peer(1)
        assert_eq!(auth.authority, test_peer(1));
    }

    #[test]
    fn rotate_not_expired_fails() {
        let mut reg = ChunkRegistry::new(1000);
        reg.assign(chunk_zero(), &peers(3), 0, [0u8; 32]).unwrap();
        assert!(reg.rotate_epoch(&chunk_zero(), &peers(3), 500, [0u8; 32]).is_err());
    }

    #[test]
    fn expired_chunks_list() {
        let mut reg = ChunkRegistry::new(100);
        let c0 = ChunkId::full(ChunkCoord::new(0, 0, 0));
        let c1 = ChunkId::full(ChunkCoord::new(1, 0, 0));
        reg.assign(c0, &peers(3), 0, [0u8; 32]).unwrap();
        reg.assign(c1, &peers(3), 50, [0u8; 32]).unwrap();
        // c0 expires at tick 100, c1 expires at tick 150
        let expired = reg.expired_chunks(100);
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0], c0);
    }

    #[test]
    fn insufficient_peers_error() {
        let mut reg = ChunkRegistry::with_default_epoch_duration();
        assert!(reg.assign(chunk_zero(), &peers(2), 0, [0u8; 32]).is_err());
    }

    #[test]
    fn round_robin_wraps() {
        let mut reg = ChunkRegistry::new(10);
        reg.assign(chunk_zero(), &peers(3), 0, [0u8; 32]).unwrap();
        // 3 peers: epoch 0→peer0, 1→peer1, 2→peer2, 3→peer0 again
        reg.rotate_epoch(&chunk_zero(), &peers(3), 10, [0u8; 32]).unwrap(); // ep 0→1 auth=peer1
        reg.rotate_epoch(&chunk_zero(), &peers(3), 20, [0u8; 32]).unwrap(); // ep 1→2 auth=peer2
        reg.rotate_epoch(&chunk_zero(), &peers(3), 30, [0u8; 32]).unwrap(); // ep 2→3 auth=peer0
        let auth = reg.get(&chunk_zero()).unwrap();
        assert_eq!(auth.authority, test_peer(0)); // wrapped
    }
}
