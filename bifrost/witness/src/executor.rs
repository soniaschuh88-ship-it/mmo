//! WitnessExecutor — the main quorum manager for a chunk's witness protocol.
//!
//! Collects `WitnessVote`s from all peers for each tick, then evaluates
//! consensus when the quorum is complete:
//!
//! - Authority + both Witnesses agree → `Accepted`
//! - Any mismatch among core peers → `Contested` + replay signal
//! - Missing votes → `Pending`

use std::collections::BTreeMap;

use thiserror::Error;

use bifrost_chunk::PeerId;
use bifrost_lockstep::LockstepTick;

use crate::quorum::ConsensusResult;
use crate::vote::WitnessVote;

/// Required core votes: authority (1) + witnesses (2).
const CORE_QUORUM_SIZE: usize = 3;

#[derive(Debug, Error)]
pub enum VoteError {
    #[error("peer {0:?} is not registered in this executor")]
    UnknownPeer(PeerId),
    #[error("tick {0} is in the past and already committed")]
    TickInPast(u64),
    #[error("duplicate vote from peer {0:?} for tick {1}")]
    DuplicateVote(PeerId, u64),
}

/// Manages the witness quorum for a single chunk.
///
/// # Roles
///
/// - `authority` — the one peer executing and producing the reference hash
/// - `witnesses[2]` — two independent verifiers
/// - `advisory` — any number of soft-vote peers (informational only)
///
/// # State storage
///
/// Votes are stored in `BTreeMap<LockstepTick, BTreeMap<PeerId, WitnessVote>>`
/// — deterministic ordering, no `HashMap` nondeterminism.
///
/// # Last accepted tick
///
/// Tracks `last_accepted_tick` to provide the `replay_from_tick` value
/// in `Contested` results.
#[derive(Debug)]
pub struct WitnessExecutor {
    pub authority: PeerId,
    pub witnesses: [PeerId; 2],
    pub advisory:  Vec<PeerId>,

    /// Collected votes per tick.
    votes: BTreeMap<LockstepTick, BTreeMap<PeerId, WitnessVote>>,

    /// The last tick that reached `Accepted` consensus.
    last_accepted_tick: Option<LockstepTick>,
}

impl WitnessExecutor {
    pub fn new(authority: PeerId, witnesses: [PeerId; 2], advisory: Vec<PeerId>) -> Self {
        Self {
            authority,
            witnesses,
            advisory,
            votes: BTreeMap::new(),
            last_accepted_tick: None,
        }
    }

    /// Update the authority and witnesses (called on epoch rotation).
    pub fn set_quorum(&mut self, authority: PeerId, witnesses: [PeerId; 2]) {
        self.authority = authority;
        self.witnesses = witnesses;
    }

    /// Add an advisory peer.
    pub fn add_advisory(&mut self, peer: PeerId) {
        if !self.advisory.contains(&peer) {
            self.advisory.push(peer);
            self.advisory.sort(); // keep deterministic
        }
    }

    /// Submit a vote from a peer.
    ///
    /// Returns `Err` if the peer is not in the quorum, if the tick is too old,
    /// or if the peer already voted for this tick.
    pub fn submit_vote(&mut self, vote: WitnessVote) -> Result<(), VoteError> {
        // Verify the peer is known
        if !self.is_known_peer(&vote.peer_id) {
            return Err(VoteError::UnknownPeer(vote.peer_id));
        }
        // Reject past committed ticks
        if let Some(last) = self.last_accepted_tick {
            if vote.tick <= last {
                return Err(VoteError::TickInPast(vote.tick.local_seq()));
            }
        }
        // Reject duplicates
        let tick_votes = self.votes.entry(vote.tick).or_default();
        if tick_votes.contains_key(&vote.peer_id) {
            return Err(VoteError::DuplicateVote(vote.peer_id, vote.tick.local_seq()));
        }
        tick_votes.insert(vote.peer_id, vote);
        Ok(())
    }

    /// Evaluate consensus for `tick`.
    ///
    /// Checks authority and both witnesses. Advisory votes are not required
    /// for consensus but their presence is noted (future: trust score updates).
    pub fn evaluate_consensus(&self, tick: LockstepTick) -> ConsensusResult {
        let tick_votes = match self.votes.get(&tick) {
            Some(v) => v,
            None    => return ConsensusResult::Pending {
                tick,
                votes_received: 0,
                votes_required: CORE_QUORUM_SIZE,
            },
        };

        // Collect core votes
        let auth_vote   = tick_votes.get(&self.authority);
        let wit0_vote   = tick_votes.get(&self.witnesses[0]);
        let wit1_vote   = tick_votes.get(&self.witnesses[1]);

        let core_count = [auth_vote, wit0_vote, wit1_vote]
            .iter()
            .filter(|v| v.is_some())
            .count();

        if core_count < CORE_QUORUM_SIZE {
            return ConsensusResult::Pending {
                tick,
                votes_received: core_count,
                votes_required: CORE_QUORUM_SIZE,
            };
        }

        // All core votes present — unwrap is safe
        let auth_hash = auth_vote.unwrap().tick_hash;
        let wit0_hash = wit0_vote.unwrap().tick_hash;
        let wit1_hash = wit1_vote.unwrap().tick_hash;

        // Find mismatching peers
        let mut mismatched: Vec<PeerId> = Vec::new();
        if wit0_hash != auth_hash {
            mismatched.push(self.witnesses[0]);
        }
        if wit1_hash != auth_hash {
            mismatched.push(self.witnesses[1]);
        }

        if mismatched.is_empty() {
            ConsensusResult::Accepted { tick, tick_hash: auth_hash }
        } else {
            let replay_from = self.last_accepted_tick
                .map(|t| t.next())
                .unwrap_or(LockstepTick::zero());
            ConsensusResult::Contested {
                tick,
                authority_hash: auth_hash,
                mismatched_peers: mismatched,
                replay_from_tick: replay_from,
            }
        }
    }

    /// Mark a tick as accepted and record it as the last known-good state.
    ///
    /// Call this after `evaluate_consensus` returns `Accepted` to allow
    /// future `Contested` results to compute `replay_from_tick` correctly.
    pub fn commit_tick(&mut self, tick: LockstepTick) {
        self.last_accepted_tick = Some(tick);
        // Evict votes for committed ticks to bound memory
        self.votes.retain(|t, _| *t > tick);
    }

    /// Remove all votes for `tick` (e.g. after replay).
    pub fn evict_tick(&mut self, tick: LockstepTick) {
        self.votes.remove(&tick);
    }

    /// True if `peer` has any role in this executor.
    fn is_known_peer(&self, peer: &PeerId) -> bool {
        peer == &self.authority
            || self.witnesses.contains(peer)
            || self.advisory.contains(peer)
    }

    /// True if the authority has voted for `tick`.
    pub fn authority_voted(&self, tick: LockstepTick) -> bool {
        self.votes.get(&tick)
            .map(|v| v.contains_key(&self.authority))
            .unwrap_or(false)
    }

    /// Last tick that reached accepted consensus.
    pub fn last_accepted_tick(&self) -> Option<LockstepTick> {
        self.last_accepted_tick
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::role::PeerRole;
    use crate::tick_hash::TickHash;

    fn peer(seed: u8) -> PeerId { PeerId([seed; 32]) }

    fn vote(peer_id: PeerId, tick: u64, hash: u8, role: PeerRole) -> WitnessVote {
        WitnessVote::unsigned(
            peer_id,
            LockstepTick::from_legacy(tick),
            TickHash::from_bytes([hash; 32]),
            role,
        )
    }

    fn make_executor() -> WitnessExecutor {
        WitnessExecutor::new(peer(1), [peer(2), peer(3)], vec![peer(4)])
    }

    #[test]
    fn pending_before_all_votes() {
        let exec = make_executor();
        let r = exec.evaluate_consensus(LockstepTick::from_legacy(0));
        assert!(r.is_pending());
    }

    #[test]
    fn accepted_on_full_agreement() {
        let mut exec = make_executor();
        exec.submit_vote(vote(peer(1), 0, 0xAA, PeerRole::Authority)).unwrap();
        exec.submit_vote(vote(peer(2), 0, 0xAA, PeerRole::Witness)).unwrap();
        exec.submit_vote(vote(peer(3), 0, 0xAA, PeerRole::Witness)).unwrap();
        let r = exec.evaluate_consensus(LockstepTick::from_legacy(0));
        assert!(r.is_accepted());
        if let ConsensusResult::Accepted { tick_hash, .. } = r {
            assert_eq!(tick_hash, TickHash::from_bytes([0xAAu8; 32]));
        }
    }

    #[test]
    fn contested_on_witness_mismatch() {
        let mut exec = make_executor();
        exec.submit_vote(vote(peer(1), 0, 0xAA, PeerRole::Authority)).unwrap();
        exec.submit_vote(vote(peer(2), 0, 0xBB, PeerRole::Witness)).unwrap(); // different!
        exec.submit_vote(vote(peer(3), 0, 0xAA, PeerRole::Witness)).unwrap();
        let r = exec.evaluate_consensus(LockstepTick::from_legacy(0));
        assert!(r.is_contested());
        if let ConsensusResult::Contested { mismatched_peers, replay_from_tick, .. } = r {
            assert_eq!(mismatched_peers.len(), 1);
            assert_eq!(mismatched_peers[0], peer(2));
            // No prior accepted tick → replay from 0
            assert_eq!(replay_from_tick, LockstepTick::from_legacy(0));
        }
    }

    #[test]
    fn replay_from_last_accepted() {
        let mut exec = make_executor();
        // Commit tick 5 as accepted
        exec.last_accepted_tick = Some(LockstepTick::from_legacy(5));
        exec.submit_vote(vote(peer(1), 6, 0xAA, PeerRole::Authority)).unwrap();
        exec.submit_vote(vote(peer(2), 6, 0xBB, PeerRole::Witness)).unwrap(); // mismatch
        exec.submit_vote(vote(peer(3), 6, 0xAA, PeerRole::Witness)).unwrap();
        let r = exec.evaluate_consensus(LockstepTick::from_legacy(6));
        if let ConsensusResult::Contested { replay_from_tick, .. } = r {
            // last_accepted = 5 → replay_from = 5.next() → local_seq = 6
            // epoch comes from the prior tick's .next(), not from_legacy(6)
            assert_eq!(replay_from_tick.local_seq(), 6);
            assert_eq!(replay_from_tick.zone_id(), LockstepTick::from_legacy(5).zone_id());
        }
    }

    #[test]
    fn duplicate_vote_rejected() {
        let mut exec = make_executor();
        exec.submit_vote(vote(peer(1), 0, 0x01, PeerRole::Authority)).unwrap();
        assert!(exec.submit_vote(vote(peer(1), 0, 0x01, PeerRole::Authority)).is_err());
    }

    #[test]
    fn unknown_peer_rejected() {
        let mut exec = make_executor();
        let result = exec.submit_vote(vote(peer(99), 0, 0x01, PeerRole::Advisory));
        assert!(matches!(result, Err(VoteError::UnknownPeer(_))));
    }

    #[test]
    fn commit_evicts_old_votes() {
        let mut exec = make_executor();
        exec.submit_vote(vote(peer(1), 0, 0xAA, PeerRole::Authority)).unwrap();
        exec.submit_vote(vote(peer(2), 0, 0xAA, PeerRole::Witness)).unwrap();
        exec.submit_vote(vote(peer(3), 0, 0xAA, PeerRole::Witness)).unwrap();
        exec.commit_tick(LockstepTick::from_legacy(0));
        // After commit, tick 0 votes should be evicted
        assert!(exec.votes.get(&LockstepTick::from_legacy(0)).is_none());
        assert_eq!(exec.last_accepted_tick(), Some(LockstepTick::from_legacy(0)));
    }

    #[test]
    fn advisory_vote_accepted() {
        let mut exec = make_executor();
        // Advisory peer (4) can vote without error
        exec.submit_vote(vote(peer(4), 0, 0xAA, PeerRole::Advisory)).unwrap();
        // But quorum is still pending without core votes
        assert!(exec.evaluate_consensus(LockstepTick::from_legacy(0)).is_pending());
    }
}
