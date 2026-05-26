//! PeerId — 32-byte public key identity for mesh peers.

use serde::{Deserialize, Serialize};

/// A peer's public key identity.
///
/// In the witness system, every peer's votes and authority claims are tied to
/// their `PeerId`. The 32 bytes correspond to an Ed25519 public key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PeerId(#[serde(with = "hex_bytes")] pub [u8; 32]);

impl PeerId {
    pub fn from_bytes(b: [u8; 32]) -> Self {
        Self(b)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Hex-encoded display representation.
    pub fn short_hex(&self) -> String {
        hex::encode(&self.0[..4]) // first 4 bytes = 8 hex chars
    }
}

impl std::fmt::Display for PeerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "peer({}…)", self.short_hex())
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

/// Create a deterministic test `PeerId` from a single byte (for unit tests).
#[cfg(any(test, feature = "test-helpers"))]
pub fn test_peer(seed: u8) -> PeerId {
    PeerId([seed; 32])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering_deterministic() {
        let p0 = test_peer(0);
        let p1 = test_peer(1);
        assert!(p0 < p1);
    }

    #[test]
    fn display() {
        let p = test_peer(0xAB);
        assert!(p.to_string().contains("abababab"));
    }
}
