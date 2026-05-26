//! TickHash — BLAKE3 hash of the world state after a tick.
//!
//! Every peer computes `TickHash` independently after executing a tick.
//! The witness quorum compares hashes to detect divergence.

use serde::{Deserialize, Serialize};

/// The canonical hash of world state after executing tick N.
///
/// ```text
/// TickHash = BLAKE3(serialized_world_state_after_tick_N)
/// ```
///
/// If two peers produce identical `TickHash` for the same tick, their
/// simulation is provably in sync (up to BLAKE3 collision resistance).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct TickHash(#[serde(with = "hex_bytes")] pub [u8; 32]);

impl TickHash {
    pub fn from_bytes(b: [u8; 32]) -> Self {
        Self(b)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// True if this is the zero/null hash (unset).
    pub fn is_zero(&self) -> bool {
        self.0 == [0u8; 32]
    }

    /// Short 8-char hex for display.
    pub fn short_hex(&self) -> String {
        hex::encode(&self.0[..4])
    }
}

impl std::fmt::Display for TickHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "hash({}…)", self.short_hex())
    }
}

// ─── serde helper ─────────────────────────────────────────────────────────────

mod hex_bytes {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(b: &[u8; 32], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&hex::encode(b))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 32], D::Error> {
        let v = hex::decode(String::deserialize(d)?).map_err(serde::de::Error::custom)?;
        v.try_into().map_err(|_| serde::de::Error::custom("expected 32 bytes"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_detection() {
        assert!(TickHash::default().is_zero());
        assert!(!TickHash::from_bytes([1u8; 32]).is_zero());
    }

    #[test]
    fn display() {
        let h = TickHash::from_bytes([0xABu8; 32]);
        assert!(h.to_string().contains("abababab"));
    }
}
