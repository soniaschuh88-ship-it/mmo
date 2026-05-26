//! JSON request and response types.

use serde::{Deserialize, Serialize};

// ─── Requests ─────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RegisterPeerReq {
    /// 64-character hex-encoded 32-byte peer ID.
    pub peer_id: String,
}

#[derive(Debug, Deserialize)]
pub struct AckReq {
    pub peer_id: String,
    pub tick:    u64,
}

/// A single instruction within a tick input.
#[derive(Debug, Deserialize)]
pub struct RawInstruction {
    pub epoch:   u64,
    /// JSON-encoded InstructionPayload (must include `"op"` discriminant).
    pub payload: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct SubmitInputReq {
    pub peer_id:      String,
    pub tick:         u64,
    pub instructions: Vec<RawInstruction>,
}

#[derive(Debug, Deserialize)]
pub struct ExecuteInstructionReq {
    pub epoch:   u64,
    pub payload: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct SetupWitnessReq {
    /// Authority peer_id (hex).
    pub authority: String,
    /// Exactly 2 witness peer_ids (hex).
    pub witnesses: [String; 2],
}

#[derive(Debug, Deserialize)]
pub struct WitnessVoteReq {
    pub peer_id:   String,
    pub tick:      u64,
    /// 64-char hex BLAKE3 state hash.
    pub tick_hash: String,
    /// "authority" | "witness" | "advisory"
    pub role:      String,
}

// ─── Responses ────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ApiInfo {
    pub name:      &'static str,
    pub version:   &'static str,
    pub endpoints: Vec<&'static str>,
}

#[derive(Serialize)]
pub struct HealthResp {
    pub status:   &'static str,
    pub tick:     u64,
    pub voxels:   usize,
    pub peers:    usize,
}

#[derive(Serialize)]
pub struct StateResp {
    pub tick:        u64,
    pub state_hash:  String,
    pub voxel_count: usize,
    pub peer_count:  usize,
}

#[derive(Serialize)]
pub struct PeerResp {
    pub peer_id:  String,
    pub action:   &'static str,
    pub peer_count: usize,
}

#[derive(Serialize)]
pub struct TickResp {
    pub current_tick:   u64,
    pub lagging_peers:  Vec<String>,
}

#[derive(Serialize)]
pub struct AdvanceResp {
    pub advanced:       bool,
    pub current_tick:   u64,
    pub completed_tick: Option<u64>,
    pub state_hash:     String,
    pub instructions_executed: usize,
}

#[derive(Serialize)]
pub struct InputResp {
    pub accepted:     bool,
    pub peer_id:      String,
    pub tick:         u64,
    pub program_hash: String,
    pub instr_count:  usize,
}

#[derive(Serialize)]
pub struct WorldResp {
    pub tick:        u64,
    pub state_hash:  String,
    pub voxel_count: usize,
}

#[derive(Serialize)]
pub struct WitnessSetupResp {
    pub authority: String,
    pub witnesses: [String; 2],
}

#[derive(Serialize)]
pub struct ConsensusResp {
    pub tick:     u64,
    pub result:   &'static str,       // "accepted" | "contested" | "pending"
    pub hash:     Option<String>,
    pub details:  serde_json::Value,
}

/// Full demo pipeline result.
#[derive(Serialize)]
pub struct DemoResult {
    pub peers:             usize,
    pub instructions:      usize,
    pub voxels_before:     usize,
    pub voxels_after:      usize,
    pub state_hash:        String,
    pub consensus:         &'static str,
    pub tick_advanced:     bool,
    pub new_tick:          u64,
    pub steps:             Vec<String>,
}

#[derive(Serialize)]
pub struct ErrorResp {
    pub error: String,
}

impl ErrorResp {
    pub fn new(msg: impl Into<String>) -> Self {
        Self { error: msg.into() }
    }
}
