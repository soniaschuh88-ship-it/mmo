//! ConsensusResult — the outcome of evaluating a witness quorum.

use serde::{Deserialize, Serialize};

use bifrost_chunk::PeerId;
use bifrost_lockstep::LockstepTick;

use crate::tick_hash::TickHash;

/// The outcome of evaluating the witness quorum for a specific tick.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsensusResult {
    /// All 3 core peers (authority + 2 witnesses) produced identical tick hashes.
    /// The tick is committed and cannot be contested.
    Accepted {
        tick:      LockstepTick,
        tick_hash: TickHash,
    },

    /// At least one core peer produced a different hash.
    /// The system must replay from `replay_from_tick`.
    Contested {
        tick:              LockstepTick,
        /// The authority's claimed hash (used as reference).
        authority_hash:    TickHash,
        /// Peers whose hash differed from the authority's.
        mismatched_peers:  Vec<PeerId>,
        /// The last known-good tick to replay from.
        replay_from_tick:  LockstepTick,
    },

    /// Not enough votes collected yet. Waiting for more peers.
    Pending {
        tick:            LockstepTick,
        votes_received:  usize,
        /// Minimum required: 3 (authority + 2 witnesses).
        votes_required:  usize,
    },
}

impl ConsensusResult {
    /// True if the tick was accepted.
    pub fn is_accepted(&self) -> bool {
        matches!(self, Self::Accepted { .. })
    }

    /// True if there is a hash conflict requiring replay.
    pub fn is_contested(&self) -> bool {
        matches!(self, Self::Contested { .. })
    }

    /// True if we are still waiting for votes.
    pub fn is_pending(&self) -> bool {
        matches!(self, Self::Pending { .. })
    }
}

impl std::fmt::Display for ConsensusResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Accepted { tick, tick_hash } =>
                write!(f, "ACCEPTED tick={} hash={}", tick.local_seq(), tick_hash.short_hex()),
            Self::Contested { tick, mismatched_peers, .. } =>
                write!(f, "CONTESTED tick={} mismatches={}", tick.local_seq(), mismatched_peers.len()),
            Self::Pending { tick, votes_received, votes_required } =>
                write!(f, "PENDING tick={} {}/{}", tick.local_seq(), votes_received, votes_required),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepted_predicate() {
        let r = ConsensusResult::Accepted {
            tick: LockstepTick::from_legacy(1),
            tick_hash: TickHash::from_bytes([1u8; 32]),
        };
        assert!(r.is_accepted());
        assert!(!r.is_contested());
        assert!(!r.is_pending());
    }

    #[test]
    fn display_pending() {
        let r = ConsensusResult::Pending {
            tick: LockstepTick::from_legacy(5),
            votes_received: 1,
            votes_required: 3,
        };
        assert!(r.to_string().contains("1/3"));
    }
}
