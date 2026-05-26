//! EpochBoundary — signed checkpoint for authority rotation.
//!
//! When an epoch ends, the outgoing authority produces an `EpochBoundary`
//! containing the final chunk state hash and their signature. New peers
//! can use this as a replay anchor — instead of replaying the entire
//! chunk history, they start from the boundary state hash.

use serde::{Deserialize, Serialize};

use crate::coord::ChunkId;
use crate::peer::PeerId;

/// Signed checkpoint produced at every authority rotation.
///
/// # Payload hash
///
/// The `payload_hash` covers all fields except `signature`:
/// ```text
/// BLAKE3(chunk_id.to_bytes() || epoch_le8 || outgoing.0 || incoming.0 ||
///        final_state_hash || final_tick_le8)
/// ```
///
/// The outgoing authority signs `payload_hash` with their Ed25519 key.
/// Witnesses verify this before accepting the `EpochBoundary`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpochBoundary {
    pub chunk_id:            ChunkId,
    pub epoch_number:        u64,
    pub outgoing_authority:  PeerId,
    pub incoming_authority:  PeerId,
    /// BLAKE3 of the chunk's voxel state at the last tick of this epoch.
    #[serde(with = "hex_bytes_32")]
    pub final_state_hash:    [u8; 32],
    /// Tick number at which the epoch ended.
    pub final_tick:          u64,
    /// Ed25519 signature of `payload_hash()` by `outgoing_authority`.
    #[serde(with = "hex_bytes_64")]
    pub signature:           [u8; 64],
}

impl EpochBoundary {
    /// Compute the BLAKE3 hash of all fields except `signature`.
    ///
    /// This is what the outgoing authority signs and witnesses verify.
    pub fn payload_hash(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.chunk_id.to_bytes());
        hasher.update(&self.epoch_number.to_le_bytes());
        hasher.update(self.outgoing_authority.as_bytes());
        hasher.update(self.incoming_authority.as_bytes());
        hasher.update(&self.final_state_hash);
        hasher.update(&self.final_tick.to_le_bytes());
        *hasher.finalize().as_bytes()
    }

    /// Unsigned boundary (signature = all-zeros).
    /// Used before the outgoing authority signs.
    pub fn unsigned(
        chunk_id: ChunkId,
        epoch_number: u64,
        outgoing_authority: PeerId,
        incoming_authority: PeerId,
        final_state_hash: [u8; 32],
        final_tick: u64,
    ) -> Self {
        Self {
            chunk_id, epoch_number,
            outgoing_authority, incoming_authority,
            final_state_hash, final_tick,
            signature: [0u8; 64],
        }
    }
}

// ─── serde helpers ─────────────────────────────────────────────────────────────

mod hex_bytes_32 {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(b: &[u8; 32], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&hex::encode(b))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 32], D::Error> {
        let v = hex::decode(String::deserialize(d)?).map_err(serde::de::Error::custom)?;
        v.try_into().map_err(|_| serde::de::Error::custom("expected 32 bytes"))
    }
}

mod hex_bytes_64 {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(b: &[u8; 64], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&hex::encode(b))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 64], D::Error> {
        let v = hex::decode(String::deserialize(d)?).map_err(serde::de::Error::custom)?;
        v.try_into().map_err(|_| serde::de::Error::custom("expected 64 bytes"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coord::ChunkCoord;
    use crate::peer::test_peer;

    #[test]
    fn payload_hash_deterministic() {
        let b = EpochBoundary::unsigned(
            ChunkId::full(ChunkCoord::new(1, 2, 3)),
            5,
            test_peer(1),
            test_peer(2),
            [0xABu8; 32],
            9999,
        );
        let h1 = b.payload_hash();
        let h2 = b.payload_hash();
        assert_eq!(h1, h2);
        assert_ne!(h1, [0u8; 32]); // non-trivial hash
    }

    #[test]
    fn different_incoming_different_hash() {
        let b1 = EpochBoundary::unsigned(
            ChunkId::full(ChunkCoord::default()), 1, test_peer(1), test_peer(2), [0u8; 32], 100,
        );
        let b2 = EpochBoundary::unsigned(
            ChunkId::full(ChunkCoord::default()), 1, test_peer(1), test_peer(9), [0u8; 32], 100,
        );
        assert_ne!(b1.payload_hash(), b2.payload_hash());
    }
}
