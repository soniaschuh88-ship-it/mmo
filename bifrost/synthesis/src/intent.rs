//! [`FactionIntent`] — world manipulation intent emitted by Synthesis agents.
//!
//! Synthesis intents use the **same format as human player intents**.
//! Every intent flows through WAC validation before world application —
//! no special faction access.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::faction::ZoneId;

// ─── IntentPriority ──────────────────────────────────────────────────────────

/// Priority tier for routing and scheduling intents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentPriority {
    Low,
    Normal,
    High,
    /// Emergency — faction survival at risk.
    Critical,
}

// ─── IntentType ──────────────────────────────────────────────────────────────

/// What kind of world manipulation this intent requests.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum IntentType {
    /// Compile a WAC blueprint and inject it into the world.
    CompileAsset {
        /// Natural language spec forwarded to WAC.
        spec:       String,
        /// Target zone for world injection.
        zone_id:    ZoneId,
        /// Asset type hint ("tile_map", "biome_definition", etc.).
        asset_type: String,
        /// Determinism seed.
        seed:       u64,
    },

    /// Attempt to capture a zone (triggers combat / influence contest).
    CaptureZone { zone_id: ZoneId },

    /// Inject agent into a zone for intelligence gathering.
    InfiltrateZone { zone_id: ZoneId },

    /// Observe auction house in Safe City — no world mutation.
    ObserveAuction,

    /// Modify biome parameters in a zone via WAC BiomeDefinition.
    AdaptBiome {
        zone_id:     ZoneId,
        temperature_delta: f32,
        humidity_delta:    f32,
    },

    /// Invest in tech level increase.
    ResearchTech { investment: f32 },
}

// ─── FactionIntent ───────────────────────────────────────────────────────────

/// A single world-manipulation intent emitted by a Synthesis agent.
///
/// The intent is validated by the WAC pipeline (IVL) before execution.
/// Rejected intents are recorded in faction memory for strategy adaptation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactionIntent {
    pub id:           Uuid,
    pub faction_id:   String,
    pub agent_id:     Uuid,
    pub tick_emitted: u64,
    pub priority:     IntentPriority,
    pub intent:       IntentType,
}

impl FactionIntent {
    pub fn new(
        faction_id:   impl Into<String>,
        agent_id:     Uuid,
        tick_emitted: u64,
        priority:     IntentPriority,
        intent:       IntentType,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            faction_id: faction_id.into(),
            agent_id,
            tick_emitted,
            priority,
            intent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intent_has_unique_id() {
        let a = FactionIntent::new(
            "synthesis", Uuid::new_v4(), 0, IntentPriority::Normal,
            IntentType::ObserveAuction,
        );
        let b = FactionIntent::new(
            "synthesis", Uuid::new_v4(), 0, IntentPriority::Normal,
            IntentType::ObserveAuction,
        );
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn capture_zone_intent_roundtrips_json() {
        let intent = FactionIntent::new(
            "synthesis", Uuid::new_v4(), 42, IntentPriority::High,
            IntentType::CaptureZone { zone_id: "zone-A3".into() },
        );
        let json = serde_json::to_string(&intent).unwrap();
        let back: FactionIntent = serde_json::from_str(&json).unwrap();
        assert_eq!(intent.id, back.id);
    }
}
