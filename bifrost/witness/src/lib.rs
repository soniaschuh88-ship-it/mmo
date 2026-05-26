//! # bifrost-witness — Witness Quorum Executor
//!
//! Implements the deterministic witness verification protocol:
//!
//! ```text
//! 1 Authority Peer
//! + 2 Witness Peers
//! + N Advisory Peers
//! ```
//!
//! All core peers independently execute the same tick and hash the resulting
//! world state. Agreement → `Accepted`. Mismatch → `Contested` + replay.
//!
//! This provides quasi-serverless verification: no giant server farm,
//! just a quorum of peers producing BLAKE3 state hashes.
//!
//! ## Key types
//!
//! - [`PeerRole`] — Authority | Witness | Advisory
//! - [`TickHash`] — BLAKE3 hash of world state after a tick
//! - [`WitnessVote`] — a peer's signed attestation on a tick hash
//! - [`ConsensusResult`] — Accepted | Contested | Pending
//! - [`WitnessExecutor`] — manages votes and evaluates quorum consensus

pub mod executor;
pub mod quorum;
pub mod role;
pub mod tick_hash;
pub mod vote;

pub use executor::{VoteError, WitnessExecutor};
pub use quorum::ConsensusResult;
pub use role::PeerRole;
pub use tick_hash::TickHash;
pub use vote::WitnessVote;
