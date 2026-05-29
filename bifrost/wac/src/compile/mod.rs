//! WAC deterministic compilers.
//!
//! Each compiler transforms an [`AssetBlueprint`] into a typed [`AssetIR`]
//! using only the blueprint's `natural_language_spec`, `constraints`, and
//! `seed`.  Identical inputs always produce identical outputs.

// AnimIR → nova-anim FSM bridge: see nova_anim::wac_bridge (feature "wac").
pub mod animation;
pub mod biome;
pub mod entity;
pub mod loot;
pub mod tilemap;

use crate::types::{AssetBlueprint, AssetIR, AssetIntent, CompiledAsset};
use crate::validate::{validate, WacError};

// ─── Shared compile helpers ───────────────────────────────────────────────────
// Extracted from the individual compilers to eliminate duplication.
// All submodules import these via `use super::{has, make_id, title_case}`.

/// True if `spec` contains any of the given keywords.
pub(crate) fn has(spec: &str, keys: &[&str]) -> bool {
    keys.iter().any(|k| spec.contains(k))
}

/// Derive a stable, URL-safe ID from the first 3 meaningful words of `spec`.
///
/// Stop-words (articles, prepositions, conjunctions in DE/EN) are filtered out.
/// Output format: `"word1-word2-word3"` (hyphenated, lowercase, alphanumeric only).
pub(crate) fn make_id(spec: &str) -> String {
    const STOP_WORDS: &[&str] = &[
        "mit","und","die","der","das",
        "the","a","an","and","or","of","in","with","that","are","is","from","at","by",
    ];
    spec.split_whitespace()
        .filter(|w| !STOP_WORDS.contains(&w.as_ref()))
        .take(3)
        .map(|w| w.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Convert a `snake_case` or `space separated` string to `Title Case`.
///
/// Treats both `_` and ` ` as word separators.
pub(crate) fn title_case(s: &str) -> String {
    s.split(|c: char| c == '_' || c == ' ' || c == '-')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                None    => String::new(),
                Some(f) => f.to_uppercase().to_string() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Validate then deterministically compile a blueprint to `AssetIR`.
pub fn compile(bp: &AssetBlueprint) -> Result<AssetIR, WacError> {
    validate(bp)?;

    let semantic_hash = crate::cache::semantic_hash(bp);

    let asset = match bp.asset_type {
        AssetIntent::TileMap         => CompiledAsset::TileMap(tilemap::compile(bp)?),
        AssetIntent::BiomeDefinition => CompiledAsset::BiomeDefinition(biome::compile(bp)?),
        AssetIntent::LootTable       => CompiledAsset::LootTable(loot::compile(bp)?),
        AssetIntent::AnimationGraph  => CompiledAsset::AnimationGraph(animation::compile(bp)?),
        AssetIntent::EntityPrefab    => CompiledAsset::EntityPrefab(entity::compile(bp)?),
    };

    Ok(AssetIR {
        blueprint_id: bp.id,
        ir_version:   2,   // bumped: 3-D → 2-D tile IR
        semantic_hash,
        asset,
    })
}
