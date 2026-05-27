//! Shared data types matching `app/world-data.json`.
//!
//! Every struct derives Serialize + Deserialize so it can be
//! round-tripped through the `/admin-api/` REST endpoints.

use serde::{Deserialize, Serialize};

// ─── World ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldSettings {
    pub name: String,
    pub size: u32,
    pub description: String,
}

impl Default for WorldSettings {
    fn default() -> Self {
        Self {
            name:        "NOVA World".into(),
            size:        60,
            description: "A voxel RPG world powered by Bifrost".into(),
        }
    }
}

// ─── Biome ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Biome {
    pub id:            String,
    pub name:          String,
    pub color:         String,
    #[serde(rename = "tileType")]
    pub tile_type:     String,
    pub zone:          String,
    pub description:   String,
    #[serde(rename = "encounterRate")]
    pub encounter_rate: f32,
}

impl Default for Biome {
    fn default() -> Self {
        Self {
            id:             String::new(),
            name:           String::new(),
            color:          "#4a8a4a".into(),
            tile_type:      "grass".into(),
            zone:           "C".into(),
            description:    String::new(),
            encounter_rate: 0.0,
        }
    }
}

// ─── Story ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoryBeat {
    pub id:              String,
    pub title:           String,
    pub description:     String,
    #[serde(rename = "minTick")]
    pub min_tick:        u64,
    #[serde(rename = "requiredQuests", default)]
    pub required_quests: Vec<String>,
    #[serde(default)]
    pub consequences:    Vec<String>,
}

impl Default for StoryBeat {
    fn default() -> Self {
        Self {
            id:              String::new(),
            title:           String::new(),
            description:     String::new(),
            min_tick:        0,
            required_quests: Vec::new(),
            consequences:    Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoryArc {
    pub id:              String,
    pub title:           String,
    pub synopsis:        String,
    pub status:          String,   // "active" | "completed" | "abandoned"
    #[serde(rename = "affectedZones", default)]
    pub affected_zones:  Vec<String>,
    #[serde(default)]
    pub beats:           Vec<StoryBeat>,
}

impl Default for StoryArc {
    fn default() -> Self {
        Self {
            id:             String::new(),
            title:          String::new(),
            synopsis:       String::new(),
            status:         "active".into(),
            affected_zones: Vec::new(),
            beats:          Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoryData {
    #[serde(rename = "worldMood")]
    pub world_mood: String,
    pub arcs:       Vec<StoryArc>,
}

// ─── NPC ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Npc {
    pub id:           String,
    pub name:         String,
    pub icon:         String,
    pub x:            i32,
    pub y:            i32,
    pub color:        String,
    pub faction:      String,    // "neutral" | "friendly" | "hostile"
    #[serde(rename = "questId")]
    pub quest_id:     Option<String>,
    #[serde(default)]
    pub lines:        Vec<String>,
    pub model:        String,
    #[serde(rename = "systemPrompt")]
    pub system_prompt: String,
    #[serde(rename = "currentGoal")]
    pub current_goal: String,
    #[serde(rename = "cooldownMs")]
    pub cooldown_ms:  u64,
}

impl Default for Npc {
    fn default() -> Self {
        Self {
            id:            String::new(),
            name:          String::new(),
            icon:          "🧍".into(),
            x:             30,
            y:             30,
            color:         "#a0a0a0".into(),
            faction:       "neutral".into(),
            quest_id:      None,
            lines:         Vec::new(),
            model:         "ollama/llama3-8b".into(),
            system_prompt: String::new(),
            current_goal:  String::new(),
            cooldown_ms:   5000,
        }
    }
}

// ─── Quest ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Quest {
    pub id:         String,
    pub title:      String,
    pub icon:       String,
    pub target:     String,   // monster id to kill / item id to collect
    pub count:      u32,
    pub gold:       u32,
    pub xp:         u32,
    #[serde(rename = "giverName")]
    pub giver_name: String,
    pub desc:       String,
}

impl Default for Quest {
    fn default() -> Self {
        Self {
            id:         String::new(),
            title:      String::new(),
            icon:       "📜".into(),
            target:     String::new(),
            count:      1,
            gold:       10,
            xp:         20,
            giver_name: String::new(),
            desc:       String::new(),
        }
    }
}

// ─── WAC — World Asset Compiler ──────────────────────────────────────────────

/// Request body for `POST /api/wac/compile`.
/// Matches `bifrost_wac::AssetBlueprint`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WacRequest {
    pub id:                    String,  // UUID v4 string
    pub asset_type:            String,  // snake_case AssetIntent
    pub natural_language_spec: String,
    pub constraints:           Vec<String>,
    pub seed:                  u64,
}

/// One zone entry for the Director tick form.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ZonePressureInput {
    pub zone_id:        String,
    pub player_density: f32,
    pub kill_rate:      f32,
    pub loot_flow:      f32,
    pub quest_rate:     f32,
    pub contention:     f32,
}

/// Global pressure inputs for the Director tick form.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GlobalPressureInput {
    pub total_players:      u32,
    pub economy_delta:      f32,
    pub player_trend:       f32,
    pub narrative_momentum: f32,
    pub quest_throughput:   f32,
}

/// Full pressure graph for `POST /api/wac/director/tick`.
/// Matches `bifrost_wac::PressureGraph`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PressureGraphRequest {
    pub zones:   std::collections::BTreeMap<String, ZonePressureInput>,
    pub global:  GlobalPressureInput,
    pub at_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DropEntry {
    #[serde(rename = "itemId")]
    pub item_id:  String,
    pub chance:   f32,
    #[serde(rename = "minQty")]
    pub min_qty:  u32,
    #[serde(rename = "maxQty")]
    pub max_qty:  u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Monster {
    pub id:    String,
    pub name:  String,
    pub hp:    u32,
    pub atk:   u32,
    pub def:   u32,
    pub xp:    u32,
    pub gold:  u32,
    pub color: String,
    pub icon:  String,
    pub zone:  String,
    #[serde(default)]
    pub drops: Vec<DropEntry>,
}

impl Default for Monster {
    fn default() -> Self {
        Self {
            id:    String::new(),
            name:  String::new(),
            hp:    20,
            atk:   5,
            def:   2,
            xp:    10,
            gold:  2,
            color: "#808080".into(),
            icon:  "👾".into(),
            zone:  "forest".into(),
            drops: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LootItem {
    pub id:          String,
    pub name:        String,
    pub icon:        String,
    pub value:       u32,
    #[serde(rename = "type")]
    pub item_type:   String,  // "material" | "currency" | "equipment" | "consumable"
    pub description: String,
}

impl Default for LootItem {
    fn default() -> Self {
        Self {
            id:          String::new(),
            name:        String::new(),
            icon:        "📦".into(),
            value:       1,
            item_type:   "material".into(),
            description: String::new(),
        }
    }
}
