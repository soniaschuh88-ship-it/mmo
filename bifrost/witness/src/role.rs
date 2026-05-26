//! PeerRole — the role a peer plays in the witness quorum.

use serde::{Deserialize, Serialize};

/// The role a peer holds in a given chunk's witness quorum.
///
/// # Quorum structure
///
/// ```text
/// 1 Authority + 2 Witnesses + N Advisory
/// ```
///
/// - **Authority**: executes the tick and produces the reference state hash.
/// - **Witness**: independently re-executes the same tick and votes on the hash.
/// - **Advisory**: soft vote, contributes to `trustScore` but not consensus.
///
/// Consensus requires agreement among Authority + both Witnesses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PeerRole {
    Authority,
    Witness,
    Advisory,
}

impl PeerRole {
    /// True if this role participates in binding consensus (Authority or Witness).
    pub fn is_core(self) -> bool {
        matches!(self, Self::Authority | Self::Witness)
    }

    /// True if this is the authority role.
    pub fn is_authority(self) -> bool {
        matches!(self, Self::Authority)
    }
}

impl std::fmt::Display for PeerRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Authority => write!(f, "authority"),
            Self::Witness   => write!(f, "witness"),
            Self::Advisory  => write!(f, "advisory"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_roles() {
        assert!(PeerRole::Authority.is_core());
        assert!(PeerRole::Witness.is_core());
        assert!(!PeerRole::Advisory.is_core());
    }
}
