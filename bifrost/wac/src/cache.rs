//! BLAKE3-keyed [`AssetCache`] — same semantic input → same compiled output.
//!
//! ## Key derivation
//!
//! ```text
//! key = BLAKE3(asset_type_tag || "\0" || natural_language_spec || "\0" || constraints_sorted)
//! ```
//!
//! The seed is intentionally **not** included in the key: two blueprints with
//! the same spec and different seeds produce different IR, and both should be
//! cached separately under their own semantic hash.  To cache by seed too,
//! call [`semantic_hash_with_seed`].

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::types::{AssetBlueprint, AssetIR};

/// Maximum entries in the in-memory cache.
pub const CACHE_CAPACITY: usize = 1024;

// ─── Hash functions ───────────────────────────────────────────────────────────

/// Compute the semantic BLAKE3 cache key for a blueprint.
///
/// Covers: asset type, spec, constraints (sorted).
/// Does **not** cover: seed, id.
pub fn semantic_hash(bp: &AssetBlueprint) -> [u8; 32] {
    let mut sorted_constraints = bp.constraints.clone();
    sorted_constraints.sort();

    let mut h = blake3::Hasher::new();
    h.update(format!("{:?}", bp.asset_type).as_bytes());
    h.update(b"\x00");
    h.update(bp.natural_language_spec.as_bytes());
    h.update(b"\x00");
    for c in &sorted_constraints {
        h.update(c.as_bytes());
        h.update(b"\x01");
    }
    *h.finalize().as_bytes()
}

/// Like [`semantic_hash`] but also incorporates the seed.
///
/// Use this when the same spec + different seeds should map to different
/// cache entries (e.g. multi-variant world generation).
pub fn semantic_hash_with_seed(bp: &AssetBlueprint) -> [u8; 32] {
    let base = semantic_hash(bp);
    let mut h = blake3::Hasher::new();
    h.update(&base);
    h.update(&bp.seed.to_le_bytes());
    *h.finalize().as_bytes()
}

// ─── Cache entry ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub key:           [u8; 32],
    pub compiled_asset: AssetIR,
}

// ─── AssetCache ───────────────────────────────────────────────────────────────

/// In-memory FIFO ring-buffer cache of compiled `AssetIR` values.
///
/// Uses BLAKE3 semantic hashes as keys.  Eviction is oldest-first (FIFO)
/// matching the [`crate::npc::memory::PromptCache`] design pattern.
#[derive(Debug, Clone, Default)]
pub struct AssetCache {
    keys:   Vec<[u8; 32]>,
    values: Vec<AssetIR>,
}

impl AssetCache {
    pub fn new() -> Self { Self::default() }

    /// Check if a compiled asset for this blueprint is already cached.
    ///
    /// Uses the **seed-inclusive** hash so different seeds get separate entries.
    pub fn get(&self, bp: &AssetBlueprint) -> Option<&AssetIR> {
        let key = semantic_hash_with_seed(bp);
        self.keys.iter().position(|k| k == &key)
            .map(|i| &self.values[i])
    }

    /// Look up by raw 32-byte key.
    pub fn get_by_key(&self, key: &[u8; 32]) -> Option<&AssetIR> {
        self.keys.iter().position(|k| k == key)
            .map(|i| &self.values[i])
    }

    /// Insert a compiled asset.  Evicts the oldest entry if at capacity.
    pub fn insert(&mut self, bp: &AssetBlueprint, ir: AssetIR) {
        let key = semantic_hash_with_seed(bp);
        // Update in-place if key exists.
        if let Some(i) = self.keys.iter().position(|k| k == &key) {
            self.values[i] = ir;
            return;
        }
        if self.keys.len() >= CACHE_CAPACITY {
            self.keys.remove(0);
            self.values.remove(0);
        }
        self.keys.push(key);
        self.values.push(ir);
    }

    /// Number of entries.
    pub fn len(&self) -> usize { self.keys.len() }

    /// True if empty.
    pub fn is_empty(&self) -> bool { self.keys.is_empty() }

    /// Clear all entries.
    pub fn clear(&mut self) { self.keys.clear(); self.values.clear(); }

    /// Snapshot all entries as (hex_key, ir) pairs — for persistence.
    pub fn snapshot(&self) -> BTreeMap<String, &AssetIR> {
        self.keys.iter().zip(self.values.iter())
            .map(|(k, v)| (hex::encode(k), v))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AssetBlueprint, AssetIntent};

    fn bp(spec: &str, seed: u64) -> AssetBlueprint {
        AssetBlueprint::new(AssetIntent::BiomeDefinition, spec, vec![], seed)
    }

    #[test]
    fn same_spec_same_semantic_hash() {
        let b1 = bp("crystal forest", 1);
        let b2 = bp("crystal forest", 1);
        assert_eq!(semantic_hash(&b1), semantic_hash(&b2));
    }

    #[test]
    fn different_seed_different_seeded_hash() {
        let b1 = bp("crystal forest", 1);
        let b2 = bp("crystal forest", 2);
        assert_ne!(semantic_hash_with_seed(&b1), semantic_hash_with_seed(&b2));
    }

    #[test]
    fn constraints_order_does_not_affect_hash() {
        let b1 = AssetBlueprint::new(
            AssetIntent::BiomeDefinition, "forest",
            vec!["no floating voxels".into(), "max_drop_rate <= 0.1".into()], 1,
        );
        let b2 = AssetBlueprint::new(
            AssetIntent::BiomeDefinition, "forest",
            vec!["max_drop_rate <= 0.1".into(), "no floating voxels".into()], 1,
        );
        assert_eq!(semantic_hash(&b1), semantic_hash(&b2));
    }

    #[test]
    fn cache_hit_after_insert() {
        let mut cache = AssetCache::new();
        let b = bp("crystal forest", 42);
        let dummy_ir = crate::types::AssetIR {
            blueprint_id: b.id,
            ir_version:   1,
            semantic_hash: [0u8; 32],
            asset: crate::types::CompiledAsset::VoxelChunk(crate::types::VoxelChunkIR {
                id: "x".into(), size: (1,1,1), seed: 1,
                material_palette: vec![], blocks: vec![],
            }),
        };
        cache.insert(&b, dummy_ir.clone());
        assert!(cache.get(&b).is_some());
    }

    #[test]
    fn evicts_at_capacity() {
        let mut cache = AssetCache::new();
        let make_ir = |b: &AssetBlueprint| crate::types::AssetIR {
            blueprint_id: b.id,
            ir_version: 1, semantic_hash: [0u8;32],
            asset: crate::types::CompiledAsset::VoxelChunk(crate::types::VoxelChunkIR {
                id:"x".into(), size:(1,1,1), seed:1, material_palette:vec![], blocks:vec![],
            }),
        };
        for i in 0..CACHE_CAPACITY {
            let b = bp(&format!("forest {i}"), i as u64 + 1);
            cache.insert(&b, make_ir(&b));
        }
        assert_eq!(cache.len(), CACHE_CAPACITY);
        let overflow = bp("forest overflow", CACHE_CAPACITY as u64 + 1);
        cache.insert(&overflow, make_ir(&overflow));
        assert_eq!(cache.len(), CACHE_CAPACITY);
        // First entry should be gone
        let first = bp("forest 0", 1);
        assert!(cache.get(&first).is_none());
    }
}
