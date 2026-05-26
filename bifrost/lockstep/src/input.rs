//! InputBuffer — per-tick, per-peer VoxelProgram storage.
//!
//! Before a tick barrier releases, each peer submits their `VoxelProgram`
//! for that tick. The buffer accumulates these submissions until the tick is
//! ready to execute. Old ticks are evicted to bound memory usage.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use bifrost_chunk::PeerId;
use bifrost_vis::VoxelProgram;

use crate::tick::LockstepTick;

/// Stores `VoxelProgram` submissions from peers, indexed by tick then peer.
///
/// Uses nested `BTreeMap` for deterministic key ordering.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct InputBuffer {
    buffer: BTreeMap<LockstepTick, BTreeMap<PeerId, VoxelProgram>>,
}

impl InputBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Submit a peer's `VoxelProgram` for a tick.
    ///
    /// If the peer already submitted for this tick, the new submission
    /// replaces the old one.
    pub fn submit(&mut self, tick: LockstepTick, peer: PeerId, program: VoxelProgram) {
        self.buffer
            .entry(tick)
            .or_default()
            .insert(peer, program);
    }

    /// Get all peer submissions for a tick.
    pub fn get_tick_inputs(&self, tick: LockstepTick) -> Option<&BTreeMap<PeerId, VoxelProgram>> {
        self.buffer.get(&tick)
    }

    /// True if all `required_peers` have submitted for `tick`.
    pub fn is_complete(&self, tick: LockstepTick, required_peers: &[PeerId]) -> bool {
        match self.buffer.get(&tick) {
            Some(map) => required_peers.iter().all(|p| map.contains_key(p)),
            None      => required_peers.is_empty(),
        }
    }

    /// Number of peers that have submitted for `tick`.
    pub fn submission_count(&self, tick: LockstepTick) -> usize {
        self.buffer.get(&tick).map(|m| m.len()).unwrap_or(0)
    }

    /// Remove all entries for ticks strictly before `cutoff_tick`.
    /// Call after a tick has been fully executed to bound memory.
    pub fn evict_before(&mut self, cutoff_tick: LockstepTick) {
        self.buffer.retain(|tick, _| *tick >= cutoff_tick);
    }

    /// Number of ticks currently in the buffer.
    pub fn buffered_tick_count(&self) -> usize {
        self.buffer.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bifrost_vis::{InstructionPayload, SetVoxelPayload, VoxelCoord};

    fn test_peer(seed: u8) -> PeerId {
        PeerId([seed; 32])
    }

    fn make_program() -> VoxelProgram {
        let mut p = VoxelProgram::new();
        p.push(1, InstructionPayload::SetVoxel(SetVoxelPayload {
            position: VoxelCoord::new(0, 0, 0),
            material: 1,
        })).unwrap();
        p
    }

    #[test]
    fn submit_and_retrieve() {
        let mut buf = InputBuffer::new();
        let tick = LockstepTick(5);
        let peer = test_peer(1);
        buf.submit(tick, peer, make_program());
        let map = buf.get_tick_inputs(tick).unwrap();
        assert!(map.contains_key(&peer));
    }

    #[test]
    fn completeness_check() {
        let mut buf = InputBuffer::new();
        let tick = LockstepTick(3);
        let peers = [test_peer(1), test_peer(2)];
        buf.submit(tick, peers[0], make_program());
        assert!(!buf.is_complete(tick, &peers));
        buf.submit(tick, peers[1], make_program());
        assert!(buf.is_complete(tick, &peers));
    }

    #[test]
    fn evict_before() {
        let mut buf = InputBuffer::new();
        for i in 0u64..5 {
            buf.submit(LockstepTick(i), test_peer(1), make_program());
        }
        assert_eq!(buf.buffered_tick_count(), 5);
        buf.evict_before(LockstepTick(3));
        assert_eq!(buf.buffered_tick_count(), 2); // ticks 3 and 4 remain
    }
}
