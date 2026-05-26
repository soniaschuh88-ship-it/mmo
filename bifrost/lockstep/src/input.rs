//! InputBuffer — per-tick, per-peer VoxelProgram storage.
//!
//! Before a tick barrier releases, each peer submits their `VoxelProgram`
//! for that tick. Ticks are keyed by `LockstepTick` (zone + local_seq),
//! so a single buffer can hold submissions for multiple zones without
//! ambiguity.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use bifrost_chunk::PeerId;
use bifrost_vis::VoxelProgram;

use crate::tick::LockstepTick;

/// Stores `VoxelProgram` submissions from peers, indexed by tick then peer.
///
/// Key ordering: `BTreeMap<LockstepTick, _>` groups by zone first, then
/// local_seq — deterministic across all platforms.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct InputBuffer {
    buffer: BTreeMap<LockstepTick, BTreeMap<PeerId, VoxelProgram>>,
}

impl InputBuffer {
    pub fn new() -> Self { Self::default() }

    /// Submit a peer's `VoxelProgram` for `tick`. Replaces any prior submission.
    pub fn submit(&mut self, tick: LockstepTick, peer: PeerId, program: VoxelProgram) {
        self.buffer.entry(tick).or_default().insert(peer, program);
    }

    /// Get all peer submissions for `tick`.
    pub fn get_tick_inputs(&self, tick: LockstepTick)
        -> Option<&BTreeMap<PeerId, VoxelProgram>>
    {
        self.buffer.get(&tick)
    }

    /// True if all `required_peers` have submitted for `tick`.
    pub fn is_complete(&self, tick: LockstepTick, required_peers: &[PeerId]) -> bool {
        match self.buffer.get(&tick) {
            Some(map) => required_peers.iter().all(|p| map.contains_key(p)),
            None      => required_peers.is_empty(),
        }
    }

    pub fn submission_count(&self, tick: LockstepTick) -> usize {
        self.buffer.get(&tick).map(|m| m.len()).unwrap_or(0)
    }

    /// Evict all ticks strictly before `cutoff_tick` (by `Ord` — zone + seq).
    pub fn evict_before(&mut self, cutoff_tick: LockstepTick) {
        self.buffer.retain(|tick, _| *tick >= cutoff_tick);
    }

    pub fn buffered_tick_count(&self) -> usize { self.buffer.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tick::ZoneId;
    use bifrost_vis::{InstructionPayload, SetVoxelPayload, VoxelCoord};

    fn t(seq: u64) -> LockstepTick { LockstepTick::from_legacy(seq) }
    fn p(seed: u8) -> PeerId       { PeerId([seed; 32]) }

    fn make_program() -> VoxelProgram {
        let mut prog = VoxelProgram::new();
        prog.push(1, InstructionPayload::SetVoxel(SetVoxelPayload {
            position: VoxelCoord::new(0, 0, 0), material: 1,
        })).unwrap();
        prog
    }

    #[test]
    fn submit_and_retrieve() {
        let mut buf = InputBuffer::new();
        buf.submit(t(5), p(1), make_program());
        assert!(buf.get_tick_inputs(t(5)).unwrap().contains_key(&p(1)));
    }

    #[test]
    fn completeness_check() {
        let mut buf = InputBuffer::new();
        let peers = [p(1), p(2)];
        buf.submit(t(3), peers[0], make_program());
        assert!(!buf.is_complete(t(3), &peers));
        buf.submit(t(3), peers[1], make_program());
        assert!(buf.is_complete(t(3), &peers));
    }

    #[test]
    fn evict_before() {
        let mut buf = InputBuffer::new();
        for i in 0u64..5 { buf.submit(t(i), p(1), make_program()); }
        assert_eq!(buf.buffered_tick_count(), 5);
        buf.evict_before(t(3));
        assert_eq!(buf.buffered_tick_count(), 2); // 3 and 4 remain
    }

    #[test]
    fn multi_zone_no_collision() {
        let mut buf = InputBuffer::new();
        let z1 = LockstepTick::at(ZoneId::new(1), 0, 0);
        let z2 = LockstepTick::at(ZoneId::new(2), 0, 0);
        buf.submit(z1, p(1), make_program());
        buf.submit(z2, p(1), make_program());
        // Both ticks stored — different keys despite same local_seq=0
        assert_eq!(buf.buffered_tick_count(), 2);
    }
}
