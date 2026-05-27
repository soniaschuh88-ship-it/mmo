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

// ─── Run System ───────────────────────────────────────────────────────────────

/// POST /run — start a new world run.
#[derive(Debug, Deserialize)]
pub struct StartRunReq {
    /// Win condition variant (serialised `EndCondition`).
    pub end_condition:   serde_json::Value,
    pub player_factions: Vec<String>,
    pub ai_factions:     Vec<String>,
    /// World seed forwarded to the WAC pipeline for world generation.
    pub world_seed:      u64,
    pub label:           String,
}

/// POST /run/tick — evaluate win conditions for the current tick.
#[derive(Debug, Deserialize)]
pub struct RunTickReq {
    pub current_tick:      u64,
    /// faction_id → zones controlled.
    pub zones_controlled:  std::collections::BTreeMap<String, u32>,
    /// faction_id → tech level.
    pub tech_levels:       std::collections::BTreeMap<String, u32>,
    /// faction_id → economy fraction (0.0–1.0).
    pub economy_fractions: std::collections::BTreeMap<String, f32>,
}

/// POST /run/end — force-end the active run.
#[derive(Debug, Deserialize)]
pub struct EndRunReq {
    pub winner_faction_id: Option<String>,
    pub reason:            String,
}

// ─── Synthesis AI ─────────────────────────────────────────────────────────────

/// POST /synthesis/init — create (or reset) the Synthesis AI faction.
#[derive(Debug, Deserialize)]
pub struct SynthesisInitReq {
    pub faction_id:   String,
    pub display_name: String,
}

/// POST /synthesis/tick — run one Synthesis AI tick.
///
/// The caller provides a lightweight world snapshot; the AI emits intents.
#[derive(Debug, Deserialize)]
pub struct SynthesisTickReq {
    pub current_tick:   u64,
    /// Zones currently owned by the Synthesis faction.
    pub owned_zones:    Vec<String>,
    /// Current threat level from human factions (0.0–1.0).
    pub threat_level:   f32,
    /// Available resource budget for this tick (reserved for future tick logic).
    #[allow(dead_code)]
    pub resource_budget: u32,
}

// ─── Safe City / Economy ──────────────────────────────────────────────────────

/// POST /safe-city/auction/list — post a new auction listing.
#[derive(Debug, Deserialize)]
pub struct PostListingReq {
    pub seller_id:  String,
    pub item_id:    String,
    pub item_name:  String,
    pub quantity:   u32,
    pub unit_price: u32,
    pub current_tick: u64,
}

/// POST /safe-city/auction/buy — purchase a listing at the fixed price.
///
/// `budget` is the buyer's available gold. The auction house validates
/// it covers the full price + tax before completing the sale.
#[derive(Debug, Deserialize)]
pub struct BuyListingReq {
    pub listing_id: String,   // UUID hex
    pub buyer_id:   String,
    pub budget:     u32,
}

/// POST /safe-city/zones/:id/influence — update faction influence on a zone.
#[derive(Debug, Deserialize)]
pub struct ZoneInfluenceReq {
    pub faction_id: String,
    /// Delta to add (positive = gaining influence, negative = losing).
    pub delta:      f32,
    /// World tick at time of influence update (logged for audit).
    #[allow(dead_code)]
    pub current_tick: u64,
}
