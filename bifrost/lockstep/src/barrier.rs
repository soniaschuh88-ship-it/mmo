//! TickBarrier — tracks peer acknowledgments and enforces the lockstep advance rule.
//!
//! The core invariant:
//! ```text
//! Tick N+1 starts ONLY when ∀ peer ∈ registered_peers: peer.last_ack >= N
//! ```
//!
//! Zone awareness: the barrier operates within a single zone. Ticks from
//! different zones are rejected by the caller (LockstepScheduler).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use bifrost_chunk::PeerId;

use crate::tick::LockstepTick;

/// Tracks which peers have completed each tick within a zone.
///
/// Uses `BTreeMap` and `BTreeSet` for deterministic iteration.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TickBarrier {
    registered: BTreeSet<PeerId>,
    acks:       BTreeMap<PeerId, LockstepTick>,
}

impl TickBarrier {
    pub fn new() -> Self { Self::default() }

    /// Add a peer to the barrier. The peer must explicitly ack before the
    /// barrier can clear — no implicit "acked tick 0" on registration.
    pub fn register(&mut self, peer: PeerId) {
        self.registered.insert(peer);
    }

    /// Remove a peer (e.g. on disconnect). Unblocks the barrier if stalled.
    pub fn evict(&mut self, peer: &PeerId) {
        self.registered.remove(peer);
        self.acks.remove(peer);
    }

    /// Record that `peer` has finished `tick`.
    /// Monotone: only advances if `tick` is later than the current record.
    pub fn ack(&mut self, peer: PeerId, tick: LockstepTick) {
        let entry = self.acks.entry(peer).or_insert(LockstepTick::zero());
        if tick > *entry {
            *entry = tick;
        }
    }

    /// The minimum acked tick across all registered peers.
    ///
    /// Returns `None` if any peer has not yet submitted an ack.
    pub fn min_acked_tick(&self) -> Option<LockstepTick> {
        if self.registered.is_empty() {
            return None;
        }
        if self.registered.iter().any(|p| !self.acks.contains_key(p)) {
            return None;
        }
        self.registered
            .iter()
            .filter_map(|p| self.acks.get(p).copied())
            .min()
    }

    /// True if all peers have acked `current_tick`.
    pub fn can_advance(&self, current_tick: LockstepTick) -> bool {
        match self.min_acked_tick() {
            Some(min) => min >= current_tick,
            None      => false,
        }
    }

    /// Peers that have not yet acked `current_tick`.
    pub fn lagging_peers(&self, current_tick: LockstepTick) -> Vec<PeerId> {
        self.registered
            .iter()
            .filter(|p| match self.acks.get(p).copied() {
                None    => true,
                Some(t) => t < current_tick,
            })
            .copied()
            .collect()
    }

    pub fn peer_count(&self) -> usize { self.registered.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tick::ShardId;

    fn t(seq: u64) -> LockstepTick { LockstepTick::from_legacy(seq) }
    fn p(seed: u8) -> PeerId       { PeerId([seed; 32]) }

    #[test]
    fn single_peer_advance() {
        let mut b = TickBarrier::new();
        b.register(p(1));
        assert!(!b.can_advance(t(1)));
        b.ack(p(1), t(1));
        assert!(b.can_advance(t(1)));
    }

    #[test]
    fn all_peers_must_ack() {
        let mut b = TickBarrier::new();
        b.register(p(1)); b.register(p(2));
        b.ack(p(1), t(5));
        assert!(!b.can_advance(t(5)));
        b.ack(p(2), t(5));
        assert!(b.can_advance(t(5)));
    }

    #[test]
    fn lagging_peers() {
        let mut b = TickBarrier::new();
        b.register(p(1)); b.register(p(2));
        b.ack(p(1), t(10));
        let lag = b.lagging_peers(t(5));
        assert_eq!(lag.len(), 1);
        assert_eq!(lag[0], p(2));
    }

    #[test]
    fn evict_unblocks() {
        let mut b = TickBarrier::new();
        b.register(p(1)); b.register(p(2));
        b.ack(p(1), t(3));
        b.evict(&p(2));
        assert!(b.can_advance(t(3)));
    }

    #[test]
    fn ack_monotone() {
        let mut b = TickBarrier::new();
        b.register(p(1));
        b.ack(p(1), t(10));
        b.ack(p(1), t(5)); // must not regress
        assert_eq!(b.min_acked_tick(), Some(t(10)));
    }

    #[test]
    fn zone_aware_advance() {
        let mut b = TickBarrier::new();
        let peer = p(1);
        b.register(peer);
        // Zone 3, seq 99
        let tick = LockstepTick::at(ShardId::new(3), 99, 0);
        b.ack(peer, tick);
        assert!(b.can_advance(tick));
    }
}
