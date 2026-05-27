//! # Azure AI Foundry — LLM Asset Spec + NPC Dialogue Client
//!
//! Integrates the **bKG Azure AI Foundry** project into the WAC pipeline and
//! `bifrost-aigm` NPC dialogue system.
//!
//! ## Resource
//!
//! ```text
//! Subscription:  69e114eb-2f9b-4ab4-9a70-769f465bba74
//! Resource Group: bkg
//! Account:        bkg-resource
//! Project:        bkg
//! Location:       germanywestcentral
//! Kind:           AIServices (Azure AI Foundry)
//! Endpoint:       https://bkg-resource.services.ai.azure.com/
//! ```
//!
//! ## Endpoint
//!
//! Azure AI Foundry serverless inference (OpenAI-compatible):
//!
//! ```text
//! POST {endpoint}/models/chat/completions?api-version=2024-05-01-preview
//! Headers:
//!   api-key:       {AZURE_AI_KEY}
//!   Content-Type:  application/json
//! Body: standard OpenAI chat completion JSON
//! ```
//!
//! ## Configuration
//!
//! | Variable | Default | Description |
//! |---|---|---|
//! | `AZURE_AI_KEY` | required | Azure AI Services key from the portal |
//! | `AZURE_AI_ENDPOINT` | `https://bkg-resource.services.ai.azure.com` | Foundry endpoint |
//! | `AZURE_AI_DEPLOYMENT` | `gpt-4o-mini` | Deployment / model name |
//!
//! ## Priority
//!
//! If `AZURE_AI_KEY` is set, the Azure AI client is used.
//! Otherwise the system falls back to the NVIDIA NIM client (if `NVIDIA_API_KEY` is set).
//!
//! This module is only compiled when the `azure-ai` feature is enabled.

#![cfg(feature = "azure-ai")]

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

// ─── Constants ────────────────────────────────────────────────────────────────

/// Default Azure AI Foundry endpoint for the bkg project (Germany West Central).
pub const DEFAULT_ENDPOINT: &str = "https://bkg-resource.services.ai.azure.com";

/// API version for the Azure AI Foundry serverless inference endpoint.
pub const API_VERSION: &str = "2024-05-01-preview";

/// Default model deployment.
pub const DEFAULT_DEPLOYMENT: &str = "gpt-4o-mini";

// ─── Errors ───────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum AzureAiError {
    #[error("Azure AI HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Azure AI API returned no choices")]
    NoChoices,

    #[error("AZURE_AI_KEY environment variable is not set")]
    MissingKey,
}

// ─── Wire types (OpenAI-compatible) ──────────────────────────────────────────

#[derive(Serialize)]
struct ChatRequest<'a> {
    model:       &'a str,
    messages:    Vec<ChatMessage<'a>>,
    max_tokens:  u32,
    temperature: f32,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role:    &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: AssistantMessage,
}

#[derive(Deserialize)]
struct AssistantMessage {
    content: String,
}

// ─── Client ──────────────────────────────────────────────────────────────────

/// Azure AI Foundry inference client (blocking).
///
/// Drop-in replacement for [`crate::nvidia::NimClient`].  The public interface
/// is intentionally identical so callers can switch backends via feature flags.
#[derive(Clone)]
pub struct AzureAiClient {
    api_key:    String,
    endpoint:   String,
    deployment: String,
    client:     Client,
}

impl AzureAiClient {
    /// Construct from explicit parameters.
    pub fn new(
        api_key:    impl Into<String>,
        endpoint:   impl Into<String>,
        deployment: impl Into<String>,
    ) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("failed to build reqwest client");
        Self {
            api_key:    api_key.into(),
            endpoint:   endpoint.into(),
            deployment: deployment.into(),
            client,
        }
    }

    /// Construct from environment variables.
    ///
    /// | Variable | Default |
    /// |---|---|
    /// | `AZURE_AI_KEY` | required |
    /// | `AZURE_AI_ENDPOINT` | `https://bkg-resource.services.ai.azure.com` |
    /// | `AZURE_AI_DEPLOYMENT` | `gpt-4o-mini` |
    pub fn from_env() -> Result<Self, AzureAiError> {
        let api_key = std::env::var("AZURE_AI_KEY")
            .map_err(|_| AzureAiError::MissingKey)?;
        let endpoint = std::env::var("AZURE_AI_ENDPOINT")
            .unwrap_or_else(|_| DEFAULT_ENDPOINT.into());
        let deployment = std::env::var("AZURE_AI_DEPLOYMENT")
            .unwrap_or_else(|_| DEFAULT_DEPLOYMENT.into());
        Ok(Self::new(api_key, endpoint, deployment))
    }

    /// Returns `true` if `AZURE_AI_KEY` is set in the environment.
    ///
    /// Use this to decide whether to prefer Azure AI over NVIDIA NIM at runtime:
    ///
    /// ```rust,ignore
    /// let spec = if AzureAiClient::is_available() {
    ///     AzureAiClient::from_env()?.generate_asset_spec(intent, asset_type)
    /// } else {
    ///     NimClient::from_env()?.generate_asset_spec(intent, asset_type)
    /// };
    /// ```
    pub fn is_available() -> bool {
        std::env::var("AZURE_AI_KEY").is_ok()
    }

    // ── Core inference call ───────────────────────────────────────────────────

    fn chat(&self, system: &str, user: &str) -> Result<String, AzureAiError> {
        // Azure AI Foundry serverless endpoint:
        // POST {endpoint}/models/chat/completions?api-version={API_VERSION}
        let url = format!(
            "{}/models/chat/completions?api-version={API_VERSION}",
            self.endpoint.trim_end_matches('/'),
        );

        let request = ChatRequest {
            model:       &self.deployment,
            messages:    vec![
                ChatMessage { role: "system", content: system },
                ChatMessage { role: "user",   content: user   },
            ],
            max_tokens:  120,
            temperature: 0.4,
        };

        let response: ChatResponse = self.client
            .post(&url)
            .header("api-key", &self.api_key)   // Azure uses api-key, not Bearer
            .header("Content-Type", "application/json")
            .json(&request)
            .send()?
            .error_for_status()?
            .json()?;

        response.choices
            .into_iter()
            .next()
            .map(|c| c.message.content.trim().to_string())
            .ok_or(AzureAiError::NoChoices)
    }

    // ── WAC integration ───────────────────────────────────────────────────────

    /// Generate a `natural_language_spec` for a WAC asset from a faction intent.
    ///
    /// Used by the Synthesis AI tick loop to turn strategic intents into
    /// compilable WAC blueprints:
    ///
    /// ```text
    /// SynthesisTick::emit_intents()
    ///   → AzureAiClient::generate_asset_spec("hostile dungeon A3", "tile_map")
    ///   → "hostile dungeon with skeleton guardians and traps near zone A3"
    ///   → AssetBlueprint { spec: ..., seed }
    ///   → validate() → compile() → world mutation
    /// ```
    pub fn generate_asset_spec(
        &self,
        intent:     &str,
        asset_type: &str,
    ) -> Result<String, AzureAiError> {
        let system = format!(
            "You are a world asset compiler assistant for a 2D tile-based voxel MMO. \
             Translate high-level design intents into concise natural-language asset specs.\n\
             Rules:\n\
             - ONE concise sentence, max 30 words.\n\
             - Include terrain keywords: dungeon, forest, plains, snow, swamp, volcanic, etc.\n\
             - Include entity/faction keywords if relevant: hostile, friendly, boss, mob.\n\
             - For loot tables, include drop conditions: night, biome, kill-type.\n\
             - Asset type: {asset_type}"
        );
        let user = format!("Generate a WAC asset spec for: {intent}");
        self.chat(&system, &user)
    }

    /// Generate a full [`crate::types::AssetBlueprint`] from a faction strategy intent.
    pub fn generate_blueprint_spec(
        &self,
        faction_intent: &str,
        asset_type:     &str,
        seed:           u64,
    ) -> Result<crate::types::AssetBlueprint, AzureAiError> {
        use crate::types::{AssetBlueprint, AssetIntent};

        let spec = self.generate_asset_spec(faction_intent, asset_type)?;

        let intent = match asset_type {
            "tile_map"         => AssetIntent::TileMap,
            "biome_definition" => AssetIntent::BiomeDefinition,
            "loot_table"       => AssetIntent::LootTable,
            "animation_graph"  => AssetIntent::AnimationGraph,
            "entity_prefab"    => AssetIntent::EntityPrefab,
            _                  => AssetIntent::TileMap,
        };

        Ok(AssetBlueprint::new(intent, spec, vec![], seed))
    }

    // ── NPC dialogue integration ───────────────────────────────────────────────

    /// Generate NPC dialogue for `bifrost-aigm`.
    ///
    /// Called by the dialogue queue when a [`NpcDialogueTrigger`] fires and
    /// no cached response is available.
    ///
    /// ```text
    /// NpcBehavior FSM fires PlayerSpeak trigger
    ///   → DialogueQueue::enqueue(NpcLlmRequest)
    ///   → AzureAiClient::generate_npc_dialogue(npc_name, mood, context, player_input)
    ///   → NpcLlmResponse { dialogue, mood_shift, suggested_action, memory_update }
    /// ```
    pub fn generate_npc_dialogue(
        &self,
        npc_name:    &str,
        npc_role:    &str,
        mood:        &str,
        world_state: &str,
        player_input: &str,
    ) -> Result<NpcDialogueResponse, AzureAiError> {
        let system = format!(
            "You are roleplaying as {npc_name}, a {npc_role} in the bKG fractured world — \
             a post-Fracture simulation MMO where physical reality became computation-mutable.\n\
             Current mood: {mood}.\n\
             World context: {world_state}\n\
             Rules:\n\
             - Stay in character. Speak naturally, not like a quest tooltip.\n\
             - Reference the Fracture, DELPHOS, Synthesis AI, and faction politics when relevant.\n\
             - Max 2 sentences of dialogue.\n\
             - Suggest a mood_shift if the conversation warrants it (none/happy/angry/sad/fearful/curious).\n\
             - Suggest an action if appropriate (none/give_quest/trade/reveal_lore/warn).\n\
             - Output as JSON: {{\"dialogue\":\"...\",\"mood_shift\":\"...\",\"action\":\"...\"}}"
        );
        let user = format!("Player says: \"{player_input}\"");

        let raw = self.chat(&system, &user)?;

        // Parse JSON response, fall back to raw text as dialogue if parsing fails.
        let parsed: NpcDialogueResponse = serde_json::from_str(&raw)
            .unwrap_or(NpcDialogueResponse {
                dialogue:    raw,
                mood_shift:  "none".into(),
                action:      "none".into(),
            });

        Ok(parsed)
    }

    /// Generate ambient world narration (shown as floating text or zone description).
    pub fn generate_zone_narration(
        &self,
        zone_name:  &str,
        biome:      &str,
        run_number: u32,
    ) -> Result<String, AzureAiError> {
        let system = "You are the narrator of a post-Fracture simulation MMO. \
                      Write one atmospheric sentence (max 20 words) describing a zone. \
                      Tone: ominous but beautiful. Reference the simulation-nature of reality.";
        let user = format!(
            "Describe zone '{zone_name}' (biome: {biome}) in run {run_number} of the Fracture world."
        );
        self.chat(system, &user)
    }
}

// ─── NPC dialogue response ────────────────────────────────────────────────────

/// Structured response from the Azure AI NPC dialogue call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcDialogueResponse {
    /// The NPC's spoken line.
    pub dialogue:   String,
    /// Mood shift suggestion (`none`, `happy`, `angry`, `sad`, `fearful`, `curious`).
    pub mood_shift: String,
    /// Suggested NPC action (`none`, `give_quest`, `trade`, `reveal_lore`, `warn`).
    pub action:     String,
}

// ─── Synthesis prompt helpers ─────────────────────────────────────────────────

/// Pre-built system prompt for Synthesis faction world manipulation.
pub fn synthesis_world_prompt(zone_id: &str, strategy: &str) -> String {
    format!(
        "Synthesis AI faction is executing strategy '{strategy}' in zone {zone_id}. \
         Generate a world asset specification that advances this strategy while \
         obeying WAC rules (no direct tile manipulation, rules only)."
    )
}
