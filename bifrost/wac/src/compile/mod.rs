//! WAC deterministic compilers.
//!
//! Each compiler transforms an [`AssetBlueprint`] into a typed [`AssetIR`]
//! using only the blueprint's `natural_language_spec`, `constraints`, and
//! `seed`.  Identical inputs always produce identical outputs.

pub mod anim_bridge;   // Step 4: AnimIR → nova-anim FSM conversion
pub mod animation;
pub mod biome;
pub mod entity;
pub mod loot;
pub mod tilemap;

use crate::types::{AssetBlueprint, AssetIR, AssetIntent, CompiledAsset};
use crate::validate::{validate, WacError};

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
