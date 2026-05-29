//! TileMap compiler — text → [`TileMapIR`].
//!
//! Generates a deterministic 2-D tile layout from a natural-language spec.
//! The output is directly consumable by the `game.html` tile renderer.
//!
//! ## Tile palette
//!
//! Uses [`crate::biomes::BIOME_IDS`] / [`crate::biomes::BiomeKey`] as the
//! canonical tile palette.  Indices are stable and shared between Rust and
//! the JS `BIOME` constant.  See `bifrost/wac/src/biomes.rs` for the full
//! list.

use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use crate::biomes::{BiomeKey, BIOME_IDS};
use crate::types::{AssetBlueprint, TileMapIR};
use crate::validate::WacError;
use super::make_id;

const DEFAULT_W: u32 = 16;
const DEFAULT_H: u32 = 16;

/// Compile a blueprint into a [`TileMapIR`].
pub fn compile(bp: &AssetBlueprint) -> Result<TileMapIR, WacError> {
    let spec     = bp.natural_language_spec.to_lowercase();
    let mut rng  = StdRng::seed_from_u64(bp.seed);
    let (w, h)   = parse_size_constraints(&bp.constraints, DEFAULT_W, DEFAULT_H);
    let dominant = dominant_tile(&spec);
    let accent   = accent_tile(&spec, dominant);

    let mut tiles = Vec::with_capacity((w * h) as usize);
    for y in 0..h {
        for x in 0..w {
            tiles.push(generate_tile(&mut rng, x, y, w, h, dominant, accent));
        }
    }

    // Solid perimeter border.
    let border = if dominant == BiomeKey::Dungeon { BiomeKey::Dungeon } else { BiomeKey::Rock };
    let bi = border as u32;
    for x in 0..w {
        tiles[(0 * w + x) as usize] = bi;
        tiles[((h-1) * w + x) as usize] = bi;
    }
    for y in 0..h {
        tiles[(y * w + 0) as usize] = bi;
        tiles[(y * w + w-1) as usize] = bi;
    }

    Ok(TileMapIR {
        id:           make_id(&spec),
        size:         (w, h),
        seed:         bp.seed,
        tile_palette: BIOME_IDS.iter().map(|s| s.to_string()).collect(),
        tiles,
    })
}

fn dominant_tile(spec: &str) -> BiomeKey {
    if spec.contains("dungeon")  || spec.contains("cave")                    { return BiomeKey::Dungeon; }
    if spec.contains("crimson")  || spec.contains("crystal")                 { return BiomeKey::CrimsonForest; }
    if spec.contains("snow")     || spec.contains("ice") || spec.contains("tundra") { return BiomeKey::Snow; }
    if spec.contains("swamp")    || spec.contains("sumpf") || spec.contains("marsh") { return BiomeKey::Swamp; }
    if spec.contains("volcanic") || spec.contains("lava") || spec.contains("magma") { return BiomeKey::Volcanic; }
    if spec.contains("forest")   || spec.contains("jungle") || spec.contains("woodland") { return BiomeKey::DarkForest; }
    if spec.contains("village")  || spec.contains("town") || spec.contains("settlement") { return BiomeKey::Village; }
    if spec.contains("building") || spec.contains("castle") || spec.contains("fortress") { return BiomeKey::Building; }
    if spec.contains("mountain") || spec.contains("peak") || spec.contains("cliff")  { return BiomeKey::Mountain; }
    if spec.contains("rock")     || spec.contains("stone") || spec.contains("plateau") { return BiomeKey::Rock; }
    if spec.contains("desert")   || spec.contains("sand") || spec.contains("dune")  { return BiomeKey::Sand; }
    if spec.contains("water")    || spec.contains("ocean") || spec.contains("sea")  { return BiomeKey::Water; }
    BiomeKey::Grass
}

fn accent_tile(spec: &str, dominant: BiomeKey) -> BiomeKey {
    let candidates: &[(BiomeKey, &str)] = &[
        (BiomeKey::Water,      "water"),
        (BiomeKey::Grass,      "grass"),
        (BiomeKey::Rock,       "stone"),
        (BiomeKey::DarkForest, "tree"),
        (BiomeKey::Mountain,   "rock"),
        (BiomeKey::Sand,       "sand"),
    ];
    for &(key, kw) in candidates {
        if key != dominant && spec.contains(kw) { return key; }
    }
    if dominant != BiomeKey::Grass { BiomeKey::Grass } else { BiomeKey::Rock }
}

fn generate_tile(
    rng: &mut StdRng, x: u32, y: u32,
    w: u32, h: u32, dominant: BiomeKey, accent: BiomeKey,
) -> u32 {
    let (cx, cy) = (w/2, h/2);
    if (x == cx && y > 2 && y < h-3) || (y == cy && x > 2 && x < w-3) {
        return dominant as u32;
    }
    let roll: f32 = rng.gen();
    if roll < 0.65 { dominant as u32 }
    else if roll < 0.85 { accent as u32 }
    else { 0 } // empty
}

fn parse_size_constraints(constraints: &[String], dw: u32, dh: u32) -> (u32, u32) {
    let (mut w, mut h) = (dw, dh);
    for c in constraints {
        let c = c.to_lowercase();
        if let Some(rest) = c.strip_prefix("size") {
            let rest = rest.trim_matches(|ch: char| ch == ' ' || ch == '=');
            if let Some((lhs, rhs)) = rest.split_once('x') {
                if let Ok(pw) = lhs.trim().parse::<u32>() { w = pw.clamp(4, 128); }
                if let Ok(ph) = rhs.trim().parse::<u32>() { h = ph.clamp(4, 128); }
            }
        }
    }
    (w, h)
}

// `make_id` is imported from `compile/mod.rs`.
