//! WAC deterministic compilers.
//!
//! Each compiler transforms an [`AssetBlueprint`] into a typed [`AssetIR`]
//! using only the blueprint's `natural_language_spec`, `constraints`, and
//! `seed`.  Identical inputs always produce identical outputs.

pub mod animation;
pub mod biome;
pub mod entity;
pub mod loot;

use crate::types::{AssetBlueprint, AssetIR, AssetIntent, CompiledAsset};
use crate::validate::{validate, WacError};

/// Validate then deterministically compile a blueprint to `AssetIR`.
pub fn compile(bp: &AssetBlueprint) -> Result<AssetIR, WacError> {
    validate(bp)?;

    let semantic_hash = crate::cache::semantic_hash(bp);

    let asset = match bp.asset_type {
        AssetIntent::BiomeDefinition => CompiledAsset::BiomeDefinition(biome::compile(bp)?),
        AssetIntent::LootTable       => CompiledAsset::LootTable(loot::compile(bp)?),
        AssetIntent::AnimationGraph  => CompiledAsset::AnimationGraph(animation::compile(bp)?),
        AssetIntent::EntityPrefab    => CompiledAsset::EntityPrefab(entity::compile(bp)?),
        AssetIntent::VoxelStructure  => {
            // Voxel generation is physics-level; return a minimal stub IR.
            CompiledAsset::VoxelChunk(crate::types::VoxelChunkIR {
                id:               bp.id.to_string(),
                size:             (16, 16, 16),
                seed:             bp.seed,
                material_palette: vec!["air".into(), "stone".into()],
                blocks:           vec![],
            })
        }
    };

    Ok(AssetIR {
        blueprint_id: bp.id,
        ir_version:   1,
        semantic_hash,
        asset,
    })
}
