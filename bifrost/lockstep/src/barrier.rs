//! TickBarrier — tracks peer acknowledgments and enforces the lockstep advance rule.
//!
//! The core invariant:
//! ```text
//! Tick N+1 starts ONLY when ∀ peer ∈ registered_peers: peer.last_ack >= N
//! ```

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use bifrost_chunk::PeerId;

use crate::tick::LockstepTick;

/// Tracks which peers have completed each tick.
///
/// Uses `BTreeMap` and `BTreeSet` for deterministic iteration.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TickBarrier {
    /// All peers currently participating in lockstep.
    registered: BTreeSet<PeerId>,

    /// Last acknowledged tick per peer.
    acks: BTreeMap<PeerId, LockstepTick>,
}

impl TickBarrier {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a peer to the barrier. All subsequent ticks require this peer's ack.
    ///
    /// The peer has no ack on join — it must explicitly call `ack()` before
    /// the barrier can clear. This prevents a spurious first-tick advance.
    pub fn register(&mut self, peer: PeerId) {
        self.registered.insert(peer);
        // Do NOT set an initial ack; peer must explicitly ack tick 0.
    }

    /// Remove a peer (e.g. on disconnect). Unblocks the barrier if it was stalled.
    pub fn evict(&mut self, peer: &PeerId) {
        self.registered.remove(peer);
        self.acks.remove(peer);
    }

    /// Record that `peer` has finished `tick`.
    ///
    /// Only advances the stored ack if `tick` is later than the current record.
    pub fn ack(&mut self, peer: PeerId, tick: LockstepTick) {
        let entry = self.acks.entry(peer).or_insert(LockstepTick::zero());
        if tick > *entry {
            *entry = tick;
        }
    }

    /// The minimum tick that all registered peers have acked.
    ///
    /// Returns `None` if no peers are registered, or if any registered peer
    /// has not yet submitted any ack.
    pub fn min_acked_tick(&self) -> Option<LockstepTick> {
        if self.registered.is_empty() {
            return None;
        }
        // If any peer has no ack on record, the barrier cannot clear.
        if self.registered.iter().any(|p| !self.acks.contains_key(p)) {
            return None;
        }
        self.registered
            .iter()
            .filter_map(|p| self.acks.get(p).copied())
            .min()
    }

    /// True if all peers have acked `current_tick`, allowing advance to `current_tick + 1`.
    pub fn can_advance(&self, current_tick: LockstepTick) -> bool {
        match self.min_acked_tick() {
            Some(min) => min >= current_tick,
            None      => false,
        }
    }

    /// Peers that have not yet acked `current_tick` (lagging peers).
    pub fn lagging_peers(&self, current_tick: LockstepTick) -> Vec<PeerId> {
        self.registered
            .iter()
            .filter(|p| {
                match self.acks.get(p).copied() {
                    None    => true,  // never acked anything
                    Some(t) => t < current_tick,
                }
            })
            .copied()
            .collect()
    }

    /// Number of registered peers.
    pub fn peer_count(&self) -> usize {
        self.registered.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_peer(seed: u8) -> PeerId {
        PeerId([seed; 32])
    }

    #[test]
    fn single_peer_advance() {
        let mut b = TickBarrier::new();
        let p = test_peer(1);
        b.register(p);
        assert!(!b.can_advance(LockstepTick(1)));
        b.ack(p, LockstepTick(1));
        assert!(b.can_advance(LockstepTick(1)));
    }

    #[test]
    fn all_peers_must_ack() {
        let mut b = TickBarrier::new();
        let p1 = test_peer(1);
        let p2 = test_peer(2);
        b.register(p1);
        b.register(p2);
        b.ack(p1, LockstepTick(5));
        assert!(!b.can_advance(LockstepTick(5))); // p2 hasn't acked
        b.ack(p2, LockstepTick(5));
        assert!(b.can_advance(LockstepTick(5)));
    }

    #[test]
    fn lagging_peers() {
        let mut b = TickBarrier::new();
        let p1 = test_peer(1);
        let p2 = test_peer(2);
        b.register(p1);
        b.register(p2);
        b.ack(p1, LockstepTick(10));
        // p2 is at tick 0
        let lagging = b.lagging_peers(LockstepTick(5));
        assert_eq!(lagging.len(), 1);
        assert_eq!(lagging[0], p2);
    }

    #[test]
    fn evict_unblocks() {
        let mut b = TickBarrier::new();
        let p1 = test_peer(1);
        let p2 = test_peer(2);
        b.register(p1);
        b.register(p2);
        b.ack(p1, LockstepTick(3));
        // p2 is stalled, evict it
        b.evict(&p2);
        assert!(b.can_advance(LockstepTick(3)));
    }

    #[test]
    fn ack_monotone() {
        let mut b = TickBarrier::new();
        let p = test_peer(1);
        b.register(p);
        b.ack(p, LockstepTick(10));
        b.ack(p, LockstepTick(5)); // older ack — must not regress
        assert_eq!(b.min_acked_tick(), Some(LockstepTick(10)));
    }
}
