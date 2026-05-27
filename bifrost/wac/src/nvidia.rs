//! # NVIDIA NIM — LLM Asset Spec Generator
//!
//! Integrates NVIDIA's NIM inference API (OpenAI-compatible) into the WAC
//! pipeline.  Instead of hand-written keyword matching, Synthesis AI agents
//! (and optionally human designers) can generate a `natural_language_spec`
//! by prompting a large language model running on NVIDIA infrastructure.
//!
//! ## Usage in the WAC pipeline
//!
//! ```text
//! Synthesis Faction Intent  ─►  NimClient::generate_asset_spec()
//!                                         │
//!                                         ▼
//!                               natural_language_spec
//!                                         │
//!                                         ▼
//!                               validate() → compile() → AssetIR
//! ```
//!
//! ## Configuration
//!
//! Set the following environment variables (injected by Cloud Run):
//!
//! | Variable | Description |
//! |---|---|
//! | `NVIDIA_API_KEY` | Bearer token for `integrate.api.nvidia.com` |
//! | `NVIDIA_NIM_BASE_URL` | API base URL (default: `https://integrate.api.nvidia.com/v1`) |
//! | `NVIDIA_NIM_MODEL` | Model name (default: `meta/llama-3.3-70b-instruct`) |
//!
//! This module is only compiled when the `nvidia-nim` feature is enabled.

#![cfg(feature = "nvidia-nim")]

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

// ─── Error ───────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum NimError {
    #[error("NIM HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("NIM API returned no choices")]
    NoChoices,

    #[error("NVIDIA_API_KEY environment variable is not set")]
    MissingApiKey,
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

/// NVIDIA NIM inference client (blocking).
///
/// The client is cheap to clone — it wraps a `reqwest` connection pool.
#[derive(Clone)]
pub struct NimClient {
    api_key:  String,
    base_url: String,
    model:    String,
    client:   Client,
}

impl NimClient {
    /// Construct a client from explicit parameters.
    pub fn new(api_key: impl Into<String>, base_url: impl Into<String>, model: impl Into<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("failed to build reqwest client");
        Self {
            api_key:  api_key.into(),
            base_url: base_url.into(),
            model:    model.into(),
            client,
        }
    }

    /// Construct a client from environment variables.
    ///
    /// Reads:
    /// - `NVIDIA_API_KEY` (required)
    /// - `NVIDIA_NIM_BASE_URL` (optional, defaults to NVIDIA's public NIM endpoint)
    /// - `NVIDIA_NIM_MODEL` (optional, defaults to Llama 3.3 70B instruct)
    pub fn from_env() -> Result<Self, NimError> {
        let api_key = std::env::var("NVIDIA_API_KEY").map_err(|_| NimError::MissingApiKey)?;
        let base_url = std::env::var("NVIDIA_NIM_BASE_URL")
            .unwrap_or_else(|_| "https://integrate.api.nvidia.com/v1".into());
        let model = std::env::var("NVIDIA_NIM_MODEL")
            .unwrap_or_else(|_| "meta/llama-3.3-70b-instruct".into());
        Ok(Self::new(api_key, base_url, model))
    }

    /// Generate a `natural_language_spec` for an asset from a structured intent.
    ///
    /// `intent` is the Synthesis faction's high-level intent string, e.g.:
    /// `"create a hostile dungeon with skeleton guardians near zone A3"`.
    ///
    /// Returns a concise spec string suitable for feeding into [`crate::compile`].
    pub fn generate_asset_spec(&self, intent: &str, asset_type: &str) -> Result<String, NimError> {
        let system_prompt = format!(
            "You are a world asset compiler assistant for a 2D tile-based MMO. \
             You translate high-level design intents into concise natural-language \
             asset specifications.\n\
             Rules:\n\
             - Respond with ONE concise sentence (max 30 words).\n\
             - Include terrain keywords: dungeon, forest, plains, snow, swamp, etc.\n\
             - Include entity/faction keywords if relevant: hostile, friendly, boss, mob.\n\
             - For loot tables, include drop conditions: night, biome, kill-type.\n\
             - Asset type is: {asset_type}"
        );

        let user_prompt = format!(
            "Generate a WAC asset spec for this intent: {intent}"
        );

        let request = ChatRequest {
            model:       &self.model,
            messages:    vec![
                ChatMessage { role: "system", content: &system_prompt },
                ChatMessage { role: "user",   content: &user_prompt },
            ],
            max_tokens:  80,
            temperature: 0.4,
        };

        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));

        let response: ChatResponse = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&request)
            .send()?
            .error_for_status()?
            .json()?;

        response
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content.trim().to_string())
            .ok_or(NimError::NoChoices)
    }

    /// Generate an [`crate::types::AssetBlueprint`] spec + intent from a faction strategy.
    ///
    /// This is the main entry point for the Synthesis AI faction:
    ///
    /// ```text
    /// Synthesis agent decides: "destabilise zone A3 with hostile dungeon"
    ///   → generate_blueprint_spec("hostile dungeon zone A3", "tile_map", seed)
    ///   → AssetBlueprint { spec: "...", asset_type: TileMap, seed }
    ///   → validate() → compile() → world mutation
    /// ```
    pub fn generate_blueprint_spec(
        &self,
        faction_intent: &str,
        asset_type:     &str,
        seed:           u64,
    ) -> Result<crate::types::AssetBlueprint, NimError> {
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
}

// ─── Synthesis prompt helpers ─────────────────────────────────────────────────

/// Pre-built system prompt for Synthesis faction world manipulation.
///
/// Synthesis agents call this before feeding the result into [`NimClient`].
pub fn synthesis_world_prompt(zone_id: &str, strategy: &str) -> String {
    format!(
        "Synthesis AI faction is executing strategy '{strategy}' in zone {zone_id}. \
         Generate a world asset specification that advances this strategy while \
         obeying WAC rules (no direct voxel/tile manipulation, rules only)."
    )
}
