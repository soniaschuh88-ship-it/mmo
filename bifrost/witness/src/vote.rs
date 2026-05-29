//! WitnessVote — a peer's signed attestation of a tick state hash.

use serde::{Deserialize, Serialize};

use bifrost_chunk::PeerId;
use bifrost_lockstep::LockstepTick;

use crate::role::PeerRole;
use crate::tick_hash::TickHash;

/// A peer's vote on the world state hash after executing tick N.
///
/// # Vote payload hash
///
/// The `payload_hash()` covers all fields except `signature`:
/// ```text
/// BLAKE3(peer_id.0 || tick_le8 || tick_hash.0 || role_byte)
/// ```
///
/// The peer signs `payload_hash()` with their Ed25519 key.
/// The signature is `[0u8; 64]` for unsigned test votes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WitnessVote {
    pub peer_id:   PeerId,
    pub tick:      LockstepTick,
    pub tick_hash: TickHash,
    pub role:      PeerRole,
    /// Ed25519 signature of `payload_hash()`. All-zeros = unsigned (test only).
    #[serde(with = "hex_bytes_64")]
    pub signature: [u8; 64],
}

impl WitnessVote {
    /// Create a vote without a signature (for testing / authority-only flows).
    pub fn unsigned(
        peer_id: PeerId,
        tick: LockstepTick,
        tick_hash: TickHash,
        role: PeerRole,
    ) -> Self {
        Self { peer_id, tick, tick_hash, role, signature: [0u8; 64] }
    }

    /// Compute the BLAKE3 payload hash that should be signed.
    pub fn payload_hash(&self) -> [u8; 32] {
        let role_byte: u8 = match self.role {
            PeerRole::Authority => 0,
            PeerRole::Witness   => 1,
            PeerRole::Advisory  => 2,
        };
        let mut hasher = blake3::Hasher::new();
        hasher.update(self.peer_id.as_bytes());
        // Use canonical_bytes() to include zone_id + local_seq + epoch
        hasher.update(&self.tick.canonical_bytes());
        hasher.update(self.tick_hash.as_bytes());
        hasher.update(&[role_byte]);
        *hasher.finalize().as_bytes()
    }

    /// True if this vote has a non-zero signature (was actually signed).
    pub fn is_signed(&self) -> bool {
        self.signature != [0u8; 64]
    }
}

// ─── serde helper ─────────────────────────────────────────────────────────────

mod hex_bytes_64 {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(b: &[u8; 64], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&hex::encode(b.as_slice()))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 64], D::Error> {
        let v = hex::decode(String::deserialize(d)?).map_err(serde::de::Error::custom)?;
        v.try_into().map_err(|_| serde::de::Error::custom("expected 64 bytes"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peer(seed: u8) -> PeerId { PeerId([seed; 32]) }

    #[test]
    fn payload_hash_deterministic() {
        let v = WitnessVote::unsigned(
            peer(1),
            LockstepTick::from_legacy(5),
            TickHash::from_bytes([0xAAu8; 32]),
            PeerRole::Authority,
        );
        assert_eq!(v.payload_hash(), v.payload_hash());
        assert_ne!(v.payload_hash(), [0u8; 32]);
    }

    #[test]
    fn different_role_different_hash() {
        let t = LockstepTick::from_legacy(1);
        let v_auth = WitnessVote::unsigned(peer(1), t, TickHash::from_bytes([1u8; 32]), PeerRole::Authority);
        let v_wit  = WitnessVote::unsigned(peer(1), t, TickHash::from_bytes([1u8; 32]), PeerRole::Witness);
        assert_ne!(v_auth.payload_hash(), v_wit.payload_hash());
    }

    #[test]
    fn different_zone_different_hash() {
        use bifrost_lockstep::ShardId;
        let h = TickHash::from_bytes([1u8; 32]);
        let t1 = LockstepTick::at(ShardId::new(1), 5, 0);
        let t2 = LockstepTick::at(ShardId::new(2), 5, 0);
        let v1 = WitnessVote::unsigned(peer(1), t1, h, PeerRole::Witness);
        let v2 = WitnessVote::unsigned(peer(1), t2, h, PeerRole::Witness);
        // Same local_seq, same hash, but different zone → different payload hash
        assert_ne!(v1.payload_hash(), v2.payload_hash());
    }

    #[test]
    fn unsigned_flag() {
        let v = WitnessVote::unsigned(peer(1), LockstepTick::zero(), TickHash::default(), PeerRole::Advisory);
        assert!(!v.is_signed());
    }
}
