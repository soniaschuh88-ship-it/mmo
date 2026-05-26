//! ChunkAuthority — the current authority assignment for a chunk epoch.

use serde::{Deserialize, Serialize};

use crate::coord::ChunkId;
use crate::peer::PeerId;

/// Describes which peer holds authority over a chunk during a given epoch.
///
/// Authority is epoch-fenced: it expires at `epoch_start_tick + epoch_duration_ticks`.
/// The holding peer is responsible for:
/// - Aggregating `VoxelInstruction`s from all peers in the chunk
/// - Producing the reference tick state hash for witness verification
/// - Signing the `EpochBoundary` at rotation time
///
/// Two witness peers independently verify the authority's output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkAuthority {
    pub chunk_id: ChunkId,

    /// The peer currently responsible for this chunk's tick execution.
    pub authority: PeerId,

    /// Two independent peers that verify the authority's tick hash.
    pub witnesses: [PeerId; 2],

    /// Sequential epoch counter, incremented on each authority rotation.
    pub epoch: u64,

    /// World tick at which this epoch started.
    pub epoch_start_tick: u64,

    /// How many ticks this epoch lasts before rotation.
    pub epoch_duration_ticks: u64,

    /// BLAKE3 hash of the chunk's voxel state at epoch start.
    /// Used as the replay anchor for new authorities joining the chunk.
    #[serde(with = "hex_bytes")]
    pub state_hash: [u8; 32],
}

impl ChunkAuthority {
    /// Construct a new epoch assignment.
    pub fn new(
        chunk_id: ChunkId,
        authority: PeerId,
        witnesses: [PeerId; 2],
        epoch: u64,
        epoch_start_tick: u64,
        epoch_duration_ticks: u64,
        state_hash: [u8; 32],
    ) -> Self {
        Self {
            chunk_id, authority, witnesses,
            epoch, epoch_start_tick, epoch_duration_ticks, state_hash,
        }
    }

    /// True if `peer` is the current authority.
    pub fn is_authority(&self, peer: &PeerId) -> bool {
        &self.authority == peer
    }

    /// True if `peer` is one of the two witnesses.
    pub fn is_witness(&self, peer: &PeerId) -> bool {
        self.witnesses.contains(peer)
    }

    /// True if `peer` has any privileged role (authority or witness).
    pub fn has_role(&self, peer: &PeerId) -> bool {
        self.is_authority(peer) || self.is_witness(peer)
    }

    /// The tick at which this epoch expires.
    pub fn epoch_end_tick(&self) -> u64 {
        self.epoch_start_tick.saturating_add(self.epoch_duration_ticks)
    }

    /// True if the current tick has passed the epoch boundary.
    pub fn is_expired(&self, current_tick: u64) -> bool {
        current_tick >= self.epoch_end_tick()
    }

    /// Remaining ticks in this epoch, saturating at 0 if expired.
    pub fn ticks_remaining(&self, current_tick: u64) -> u64 {
        self.epoch_end_tick().saturating_sub(current_tick)
    }
}

// ─── serde helper ─────────────────────────────────────────────────────────────

mod hex_bytes {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &[u8; 32], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&hex::encode(bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 32], D::Error> {
        let s = String::deserialize(d)?;
        let v = hex::decode(&s).map_err(serde::de::Error::custom)?;
        v.try_into()
            .map_err(|_| serde::de::Error::custom("expected 32-byte hex string"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coord::ChunkCoord;
    use crate::peer::test_peer;

    fn make_authority(start: u64, duration: u64) -> ChunkAuthority {
        ChunkAuthority::new(
            ChunkId::full(ChunkCoord::default()),
            test_peer(1),
            [test_peer(2), test_peer(3)],
            0, start, duration, [0u8; 32],
        )
    }

    #[test]
    fn expiry() {
        let a = make_authority(0, 1000);
        assert!(!a.is_expired(999));
        assert!(a.is_expired(1000));
        assert!(a.is_expired(2000));
    }

    #[test]
    fn ticks_remaining() {
        let a = make_authority(500, 1000);
        assert_eq!(a.ticks_remaining(500), 1000);
        assert_eq!(a.ticks_remaining(900), 600);
        assert_eq!(a.ticks_remaining(1500), 0);
    }

    #[test]
    fn roles() {
        let a = make_authority(0, 1000);
        assert!(a.is_authority(&test_peer(1)));
        assert!(a.is_witness(&test_peer(2)));
        assert!(a.is_witness(&test_peer(3)));
        assert!(!a.has_role(&test_peer(9)));
    }
}
