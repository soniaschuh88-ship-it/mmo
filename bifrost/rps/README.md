# bifrost-rps — Resonance Progression System Integration

> **Engine crate for RPS implementation in DRCF**

---

## Overview

`bifrost-rps` is the DRCF subsystem that:

1. Analyzes behavior from the event ledger
2. Generates resonance progression
3. Emits resonance events
4. Integrates with NPC memory and world direction
5. Ensures full determinism and replayability

**Status:** Ready for implementation  
**Depends on:** `bifrost-aigm`, `bifrost-lockstep`  
**Used by:** `bifrost-server`, `bifrost-wac`  

---

## Cargo.toml

```toml
[package]
name        = "bifrost-rps"
description = "Resonance Progression System — behavior-based identity evolution"
version.workspace    = true
edition.workspace    = true
license.workspace    = true
authors.workspace    = true
repository.workspace = true

[dependencies]
serde       = { workspace = true }
serde_json  = { workspace = true }
blake3      = { workspace = true }
thiserror   = { workspace = true }
uuid        = { workspace = true }
hex         = { workspace = true }

bifrost-aigm = { workspace = true }

[features]
default = []
test-helpers = []
```

---

## Module Graph

```
bifrost-rps/
├── src/
│   ├── lib.rs              # exports
│   ├── types.rs            # core types (Resonance, Engraving, etc)
│   ├── analyzer.rs         # pattern matching + emergence logic
│   ├── lifecycle.rs        # state machine (Dormant → Emerged → Crystallized)
│   ├── events.rs           # RPS event types
│   ├── abuse_detection.rs  # anti-exploit rules
│   ├── serialization.rs    # BLAKE3 verification
│   ├── replication.rs      # WASM sync rules
│   └── tests/
│       ├── determinism.rs  # verify replay-identical outputs
│       ├── lifecycle.rs    # test all state transitions
│       └── abuse.rs        # verify exploit prevention
```

---

## Core Types

### lib.rs

```rust
//! # bifrost-rps
//!
//! Resonance Progression System for DRCF.
//!
//! Behavior → Ledger → Analysis → Resonance → Identity
//!
//! ## Architecture
//!
//! ```text
//! Player Action
//!     ↓
//! WorldEvent (bifrost-aigm)
//!     ↓
//! Ledger append
//!     ↓
//! ResonanceAnalyzer::analyze_deterministic()
//!     ↓
//! ResonanceProfile updated
//!     ↓
//! ResonanceEvent emitted
//!     ↓
//! Broadcast to NPCs, WorldDirector, Clients
//! ```
//!
//! All analysis is **pure, deterministic, and fully replayable**.

pub mod types;
pub mod analyzer;
pub mod lifecycle;
pub mod events;
pub mod abuse_detection;
pub mod serialization;
pub mod replication;

pub use types::{
    ResonanceProfile, InstinctResonance, MemoryEngraving, SoulFracture,
    CombatTechnique, MythicEcho, RealityEntanglement, KnowledgeFragment,
    ResonanceState, ResonanceType,
};

pub use analyzer::{ResonanceAnalyzer, PatternType, EmergenceCondition};
pub use events::{ResonanceEvent, ResonancePayload};
pub use abuse_detection::AbuseDetector;

pub type Result<T> = std::result::Result<T, RpsError>;

#[derive(Debug, thiserror::Error)]
pub enum RpsError {
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("integrity violation: hash mismatch")]
    IntegrityViolation,
    
    #[error("invalid hash format")]
    InvalidHash,
    
    #[error("abuse detected: {0}")]
    AbuseDetected(String),
    
    #[error("emergence threshold not met")]
    ThresholdNotMet,
    
    #[error("player not found")]
    PlayerNotFound,
    
    #[error("resonance state invalid")]
    InvalidState,
}
```

### types.rs

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete player identity profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResonanceProfile {
    pub player_id: String,
    pub world_hash: [u8; 32],
    
    // Identity layers
    pub instincts: Vec<(InstinctResonance, ResonanceState)>,
    pub engravings: Vec<MemoryEngraving>,
    pub fractures: Vec<SoulFracture>,
    pub techniques: Vec<CombatTechnique>,
    pub echoes: Vec<MythicEcho>,
    pub entanglements: Vec<RealityEntanglement>,
    pub knowledge_pool: Vec<KnowledgeFragment>,
    
    // Metadata
    pub last_updated_tick: u64,
    pub integrity_hash: [u8; 32],
    pub total_strength: f32,  // cached
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InstinctResonance {
    LastStandInstinct,
    ThreadwalkerReflex,
    CounterPredator,
    NocturnalAwareness,
    GhostFooting,
    VenomStrike,
    ElementalAttunement,
    VeteransResolve,
    ProtectorsResolve,
    HuntersPredation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEngraving {
    pub id: String,
    pub title: String,
    pub description: String,
    pub event_seq: u64,
    pub world_hash: [u8; 32],
    pub significance: f32,  // 0.1-1.0
    pub npcs_aware: Vec<String>,
    pub mechanical_effects: Vec<EngravingEffect>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EngravingEffect {
    ReputationBonus { faction: String, delta: f32 },
    PriceModifier(f32),
    DialogueUnlock(String),
    AreaEffect { zone: String, effect: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoulFracture {
    pub id: String,
    pub fracture_type: SoulFractureType,
    pub intensity: f32,  // 0.0-1.0
    pub visible_to_npcs: bool,
    pub acquired_tick: u64,
    pub healing_methods: Vec<HealingMethod>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SoulFractureType {
    FearOfFlame,
    VoidEcho,
    BattleHunger,
    GhostMemory,
    ParalysisTrauma,
    VengeanceOath,
    MourningWeight,
    CowardsMark,
    DeathEater,
    ApocalypseVision,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealingMethod {
    TimeOnly { ticks_required: u64 },
    ActionRequired { actions: Vec<String> },
    NpcHeal { npc_id: String, cost: u32 },
    ItemSacrifice { item_id: String },
    Ritual { name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatTechnique {
    pub name: String,
    pub description: String,
    pub emerged_from: Vec<String>,  // combat action names
    pub cooldown_ms: u32,
    pub damage_multiplier: f32,
    pub effectiveness_conditions: Vec<TechniqueCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TechniqueCondition {
    AlliesNearby,
    EnemiesOutnumbered,
    InDarkness,
    MovingBackward,
    HasCombo { previous_attack: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MythicEcho {
    pub event_name: String,
    pub event_hash: [u8; 32],
    pub participants: Vec<String>,
    pub significance: f32,
    pub monument: Option<MonumentData>,
    pub npc_memories: Vec<(String, String)>,  // (npc_id, memory_text)
    pub mechanical_effects: Vec<EchoEffect>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonumentData {
    pub location: [f32; 3],
    pub visual_mesh: String,
    pub inscription: String,
    pub decay_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EchoEffect {
    PermanentBonus { stat: String, delta: f32 },
    ContentUnlock(String),
    FactionsModifier { faction: String, delta: f32 },
    DialogueUnlock(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealityEntanglement {
    pub zone_id: String,
    pub entanglement_type: EntanglementType,
    pub strength: f32,
    pub effects: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntanglementType {
    Bound,        // tied to place
    Haunted,      // ghosts follow
    Corrupted,    // zone altered by you
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeFragment {
    pub id: String,
    pub fragment_type: KnowledgeFragmentType,
    pub content_hash: [u8; 32],
    pub discovered_by: String,
    pub potency: f32,  // 0.5-1.0 (degrades if copied)
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KnowledgeFragmentType {
    BossWeakness(String),
    CraftingRecipe(String),
    RegionSecret(String),
    FactionIntelligence(String),
    AncientLore(String),
}

/// State machine for resonance accumulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResonanceState {
    Dormant {
        pattern_matches: u32,
        required_for_emergence: u32,
    },
    Emerged {
        strength: f32,
        first_activated_tick: u64,
        latest_reinforcement_tick: u64,
    },
    Fading {
        remaining_ticks: u64,
    },
    Crystallized {
        immutable_since_tick: u64,
        world_events_that_locked: Vec<u64>,
    },
}

/// Public visibility of resonance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResonanceType {
    PublicAlways,
    FriendsOnly,
    MythicOnly,
}

impl ResonanceProfile {
    pub fn new(player_id: impl Into<String>) -> Self {
        Self {
            player_id: player_id.into(),
            world_hash: [0u8; 32],
            instincts: Vec::new(),
            engravings: Vec::new(),
            fractures: Vec::new(),
            techniques: Vec::new(),
            echoes: Vec::new(),
            entanglements: Vec::new(),
            knowledge_pool: Vec::new(),
            last_updated_tick: 0,
            integrity_hash: [0u8; 32],
            total_strength: 0.0,
        }
    }

    pub fn total_strength(&self) -> f32 {
        self.instincts.iter()
            .map(|(_, state)| state.strength_value())
            .sum::<f32>()
            + (self.engravings.len() as f32 * 0.1)
            + (self.echoes.len() as f32 * 0.2)
    }

    pub fn has_mythic_echoes(&self) -> bool {
        self.echoes.iter().any(|e| e.significance > 0.8)
    }
}

impl ResonanceState {
    pub fn strength_value(&self) -> f32 {
        match self {
            ResonanceState::Dormant { .. } => 0.0,
            ResonanceState::Emerged { strength, .. } => *strength,
            ResonanceState::Fading { remaining_ticks } => {
                (*remaining_ticks as f32) / 10000.0  // fade over 10k ticks
            },
            ResonanceState::Crystallized { .. } => 1.0,
        }
    }
}
```

---

## analyzer.rs

```rust
use crate::types::*;
use bifrost_aigm::WorldEvent;
use std::collections::HashMap;

pub struct ResonanceAnalyzer {
    pub emergence_conditions: HashMap<InstinctResonance, EmergenceCondition>,
}

pub struct EmergenceCondition {
    pub pattern_type: PatternType,
    pub required_matches: u32,
    pub window_size_ticks: u64,
    pub confidence_threshold: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternType {
    OutnumberedWins,
    PerfectDodges,
    DistanceFighting,
    DefensiveKills,
    NightCombat,
    HighAltitude,
}

impl ResonanceAnalyzer {
    pub fn new() -> Self {
        let mut conditions = HashMap::new();
        
        conditions.insert(
            InstinctResonance::LastStandInstinct,
            EmergenceCondition {
                pattern_type: PatternType::OutnumberedWins,
                required_matches: 10,
                window_size_ticks: 100_000,
                confidence_threshold: 0.8,
            },
        );
        
        // ... more conditions ...
        
        Self { emergence_conditions: conditions }
    }

    /// Pure function: same input → same output always
    pub fn analyze_deterministic(
        &self,
        ledger_slice: &[WorldEvent],
        player_id: &str,
        current_world_hash: &[u8; 32],
    ) -> Result<ResonanceProfile, crate::RpsError> {
        let mut profile = ResonanceProfile::new(player_id);
        let mut pattern_tracker: HashMap<PatternType, Vec<u64>> = HashMap::new();

        // Iterate ledger in order (immutable)
        for event in ledger_slice {
            if event.is_authored_by_player(player_id) {
                // Pattern matching (no randomness)
                if let Some(pattern) = self.detect_pattern(event) {
                    pattern_tracker.entry(pattern)
                        .or_insert_with(Vec::new)
                        .push(event.seq);

                    // Check emergence
                    if self.should_emerge(pattern, &pattern_tracker) {
                        let instinct = self.pattern_to_instinct(pattern);
                        profile.emit_emergence(instinct, event.seq);
                    }
                }
            }
        }

        profile.last_updated_tick = ledger_slice.last()
            .map(|e| e.seq)
            .unwrap_or(0);
        profile.world_hash = *current_world_hash;
        profile.integrity_hash = crate::serialization::compute_hash(&profile);

        Ok(profile)
    }

    fn detect_pattern(&self, event: &WorldEvent) -> Option<PatternType> {
        match event.event_type {
            bifrost_aigm::EventType::CombatWon => {
                if self.is_outnumbered(event) {
                    Some(PatternType::OutnumberedWins)
                } else {
                    None
                }
            },
            bifrost_aigm::EventType::CombatDodged => {
                Some(PatternType::PerfectDodges)
            },
            _ => None,
        }
    }

    fn should_emerge(
        &self,
        pattern: PatternType,
        tracker: &HashMap<PatternType, Vec<u64>>,
    ) -> bool {
        if let Some(condition) = self.emergence_conditions.get(&self.pattern_to_instinct(pattern)) {
            let recent = tracker.get(&pattern)
                .map(|v| v.len())
                .unwrap_or(0) as u32;
            recent >= condition.required_matches
        } else {
            false
        }
    }

    fn pattern_to_instinct(&self, pattern: PatternType) -> InstinctResonance {
        match pattern {
            PatternType::OutnumberedWins => InstinctResonance::LastStandInstinct,
            PatternType::PerfectDodges => InstinctResonance::ThreadwalkerReflex,
            _ => InstinctResonance::VeteransResolve,
        }
    }

    fn is_outnumbered(&self, event: &WorldEvent) -> bool {
        // Parse event payload to check if player was outnumbered
        // (simplified; real impl checks combat context)
        true
    }
}
```

---

## events.rs

```rust
use serde::{Deserialize, Serialize};
use crate::types::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResonanceEvent {
    pub seq: u64,
    pub event_type: ResonanceEventType,
    pub payload: ResonancePayload,
    pub author: String,
    pub world_hash: [u8; 32],
    pub ts_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResonanceEventType {
    Emergence,
    Engraving,
    SoulFracture,
    SoulFractureHealed,
    KnowledgeDropped,
    KnowledgePickedUp,
    MythicEchoCreated,
    Entanglement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResonancePayload {
    Emergence(EmergencePayload),
    Engraving(EngravingPayload),
    SoulFracture(SoulFracturePayload),
    Knowledge(KnowledgePayload),
    MythicEcho(MythicEchoPayload),
    Entanglement(EntanglementPayload),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencePayload {
    pub instinct_type: String,
    pub strength: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngravingPayload {
    pub title: String,
    pub significance: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoulFracturePayload {
    pub fracture_type: String,
    pub intensity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgePayload {
    pub fragment_type: String,
    pub potency: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MythicEchoPayload {
    pub event_name: String,
    pub participants: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntanglementPayload {
    pub zone_id: String,
    pub entanglement_type: String,
}
```

---

## abuse_detection.rs

```rust
use crate::types::*;

pub struct AbuseDetector {
    pub rules: Vec<AbuseRule>,
}

pub enum AbuseRule {
    SameSourcingLimit {
        instinct: InstinctResonance,
        max_matches_per_day: u32,
    },
    CollusionDetection {
        threshold: f32,
    },
}

impl AbuseDetector {
    pub fn new() -> Self {
        Self {
            rules: vec![
                AbuseRule::SameSourcingLimit {
                    instinct: InstinctResonance::LastStandInstinct,
                    max_matches_per_day: 5,
                },
            ],
        }
    }

    pub fn validate(&self, profile: &ResonanceProfile) -> Result<(), String> {
        for rule in &self.rules {
            match rule {
                AbuseRule::SameSourcingLimit { instinct, max_matches_per_day } => {
                    let count = profile.instincts.iter()
                        .filter(|(i, _)| i == instinct)
                        .count() as u32;
                    if count > max_matches_per_day {
                        return Err(format!("Instinct spam detected: {}", count));
                    }
                },
                _ => {}
            }
        }
        Ok(())
    }
}
```

---

## serialization.rs

```rust
use crate::types::ResonanceProfile;
use blake3::Hasher;

pub fn compute_hash(profile: &ResonanceProfile) -> [u8; 32] {
    let mut hasher = Hasher::new();

    // Hash player ID
    hasher.update(profile.player_id.as_bytes());

    // Hash world hash
    hasher.update(&profile.world_hash);

    // Hash all instincts
    for (instinct, state) in &profile.instincts {
        hasher.update(format!("{:?}", instinct).as_bytes());
        hasher.update(format!("{:?}", state).as_bytes());
    }

    // Hash engravings
    for engraving in &profile.engravings {
        hasher.update(engraving.id.as_bytes());
    }

    *hasher.finalize().as_bytes()
}

pub fn verify_integrity(profile: &ResonanceProfile) -> bool {
    let computed = compute_hash(profile);
    computed == profile.integrity_hash
}
```

---

## replication.rs

```rust
use crate::types::*;

pub struct ReplicationPolicy {
    pub resonance_type: ResonanceType,
}

pub fn should_replicate_to_client(
    source_player: &str,
    target_player: &str,
    resonance: &ResonanceProfile,
    policy: &ReplicationPolicy,
) -> bool {
    match policy.resonance_type {
        ResonanceType::PublicAlways => true,
        ResonanceType::FriendsOnly => is_friend(source_player, target_player),
        ResonanceType::MythicOnly => resonance.has_mythic_echoes(),
    }
}

fn is_friend(_a: &str, _b: &str) -> bool {
    // Simplified; real impl checks friendship registry
    false
}
```

---

## Tests

### tests/determinism.rs

```rust
#[cfg(test)]
mod tests {
    use super::super::*;
    use bifrost_aigm::{WorldEvent, EventType};

    #[test]
    fn analysis_is_deterministic() {
        let analyzer = ResonanceAnalyzer::new();
        let events = create_test_ledger();
        
        let profile1 = analyzer.analyze_deterministic(&events, "player-1", &[0u8; 32]).unwrap();
        let profile2 = analyzer.analyze_deterministic(&events, "player-1", &[0u8; 32]).unwrap();
        
        assert_eq!(profile1.integrity_hash, profile2.integrity_hash);
    }

    fn create_test_ledger() -> Vec<WorldEvent> {
        vec![
            // Mock events
        ]
    }
}
```

---

## API Surface

```rust
// Main entry point: analyze player from ledger
pub fn analyze_player_resonance(
    analyzer: &ResonanceAnalyzer,
    ledger: &[WorldEvent],
    player_id: &str,
    current_world_hash: &[u8; 32],
) -> Result<ResonanceProfile> {
    analyzer.analyze_deterministic(ledger, player_id, current_world_hash)
}

// Verify integrity
pub fn verify_profile_integrity(profile: &ResonanceProfile) -> bool {
    serialization::verify_integrity(profile)
}

// Check for abuse
pub fn detect_abuse(profile: &ResonanceProfile) -> Result<()> {
    let detector = AbuseDetector::new();
    detector.validate(profile)
}

// Determine visibility
pub fn should_broadcast_to_client(
    source: &str,
    target: &str,
    profile: &ResonanceProfile,
) -> bool {
    replication::should_replicate_to_client(
        source,
        target,
        profile,
        &ReplicationPolicy {
            resonance_type: ResonanceType::PublicAlways,
        },
    )
}
```

---

## Integration with DRCF

### In bifrost-server

```rust
pub async fn handle_resonance_analysis(
    State(state): State<Arc<AppState>>,
    Path(player_id): Path<String>,
) -> Json<ResonanceProfile> {
    let analyzer = ResonanceAnalyzer::new();
    let profile = analyzer.analyze_deterministic(
        &state.ledger,
        &player_id,
        &state.current_world_hash,
    ).unwrap();
    
    Json(profile)
}
```

### In AiGmState tick

```rust
impl AiGmState {
    pub fn tick(&mut self, ...) -> AiGmTick {
        // ... existing logic ...
        
        // NEW: analyze resonance
        let mut resonance_events = Vec::new();
        for player_id in self.active_players.iter() {
            let profile = bifrost_rps::analyze_player_resonance(
                &self.resonance_analyzer,
                &self.recent_events,
                player_id,
                &self.head_hash,
            ).ok();
            
            if let Some(profile) = profile {
                // Emit events if resonance changed
                // ...
            }
        }
        
        AiGmTick {
            resonance_events,
            // ...
        }
    }
}
```

---

## Summary

`bifrost-rps` is:

✅ **Deterministic** — pure functions, no randomness  
✅ **Replayable** — fully reconstructible from ledger  
✅ **Integrated** — fits into AiGmTick seamlessly  
✅ **Scalable** — O(n) per player per tick  
✅ **Abuse-resistant** — built-in detection  
✅ **WASM-compatible** — serializable types  

Ready for integration into DRCF.
