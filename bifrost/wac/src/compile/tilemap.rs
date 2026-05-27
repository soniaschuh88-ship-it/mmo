//! TileMap compiler — text → [`TileMapIR`].
//!
//! Generates a deterministic 2-D tile layout from a natural-language spec.
//! The output is directly consumable by the `game.html` tile renderer.
//!
//! ## Tile palette (fixed indices)
//!
//! | Index | Name        | `game.html` biome ID |
//! |-------|-------------|----------------------|
//! | 0     | empty       | —                    |
//! | 1     | grass       | B.GR                 |
//! | 2     | stone_floor | B.RK                 |
//! | 3     | water       | B.W                  |
//! | 4     | sand        | B.S                  |
//! | 5     | dungeon     | B.DG                 |
//! | 6     | snow        | B.SN                 |
//! | 7     | swamp       | B.SW                 |
//! | 8     | forest      | B.FO                 |
//! | 9     | village     | B.VI                 |
//! | 10    | building    | B.BL                 |
//! | 11    | mountain    | B.MN                 |

use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use crate::types::{AssetBlueprint, TileMapIR};
use crate::validate::WacError;

/// Standard 2-D tile palette shared with the game renderer.
pub const TILE_PALETTE: &[&str] = &[
    "empty",        // 0
    "grass",        // 1
    "stone_floor",  // 2
    "water",        // 3
    "sand",         // 4
    "dungeon",      // 5
    "snow",         // 6
    "swamp",        // 7
    "forest",       // 8
    "village",      // 9
    "building",     // 10
    "mountain",     // 11
];

// Default map dimensions.
const DEFAULT_W: u32 = 16;
const DEFAULT_H: u32 = 16;

/// Compile a blueprint into a [`TileMapIR`].
///
/// The spec text is used to pick a dominant tile theme.
/// The seed ensures identical (spec, constraints, seed) → identical output.
pub fn compile(bp: &AssetBlueprint) -> Result<TileMapIR, WacError> {
    let spec  = bp.natural_language_spec.to_lowercase();
    let mut rng = StdRng::seed_from_u64(bp.seed);

    // Derive map size from constraint hints (default 16×16).
    let (width, height) = parse_size_constraints(&bp.constraints, DEFAULT_W, DEFAULT_H);

    // Pick dominant + accent tile indices from the spec text.
    let dominant = dominant_tile(&spec);
    let accent   = accent_tile(&spec, dominant);

    // Build a simple noise-based 2-D layout.
    let total = (width * height) as usize;
    let mut tiles = Vec::with_capacity(total);

    for y in 0..height {
        for x in 0..width {
            let t = generate_tile(&mut rng, x, y, width, height, dominant, accent);
            tiles.push(t);
        }
    }

    // Override border row with walls/perimeter tile (stone or matching terrain).
    let perimeter = if matches!(dominant, 5) { 5u32 } else { 2 }; // dungeon → dungeon, else stone
    for x in 0..width {
        tiles[(0 * width + x) as usize] = perimeter;
        tiles[((height - 1) * width + x) as usize] = perimeter;
    }
    for y in 0..height {
        tiles[(y * width + 0) as usize] = perimeter;
        tiles[(y * width + width - 1) as usize] = perimeter;
    }

    let id = make_id(&spec);

    Ok(TileMapIR {
        id,
        size: (width, height),
        seed: bp.seed,
        tile_palette: TILE_PALETTE.iter().map(|s| s.to_string()).collect(),
        tiles,
    })
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Map spec keywords to a dominant tile palette index.
fn dominant_tile(spec: &str) -> u32 {
    if spec.contains("dungeon") || spec.contains("cave")          { return 5; }
    if spec.contains("snow")   || spec.contains("ice")            { return 6; }
    if spec.contains("swamp")  || spec.contains("sumpf")          { return 7; }
    if spec.contains("forest") || spec.contains("jungle")         { return 8; }
    if spec.contains("village")|| spec.contains("town")           { return 9; }
    if spec.contains("building")|| spec.contains("castle")        { return 10; }
    if spec.contains("mountain")|| spec.contains("cliff")         { return 11; }
    if spec.contains("desert") || spec.contains("sand")           { return 4; }
    if spec.contains("water")  || spec.contains("ocean")          { return 3; }
    1 // default: grass
}

/// Return a secondary accent tile index (must differ from dominant).
fn accent_tile(spec: &str, dominant: u32) -> u32 {
    let candidates: &[(u32, &str)] = &[
        (3,  "water"),
        (1,  "grass"),
        (2,  "stone"),
        (8,  "tree"),
        (11, "rock"),
    ];
    for &(idx, kw) in candidates {
        if idx != dominant && spec.contains(kw) { return idx; }
    }
    // Fallback: pick a different index
    if dominant != 1 { 1 } else { 2 }
}

/// Deterministic per-tile selector.
fn generate_tile(
    rng:      &mut StdRng,
    x: u32, y: u32,
    width: u32, height: u32,
    dominant: u32,
    accent:   u32,
) -> u32 {
    // Leave a clear path through the centre (passable corridor).
    let cx = width  / 2;
    let cy = height / 2;
    if (x == cx && y > 2 && y < height - 3) || (y == cy && x > 2 && x < width - 3) {
        return dominant; // centre cross corridor always passable
    }

    let roll: f32 = rng.gen();
    if roll < 0.65 { dominant }
    else if roll < 0.85 { accent }
    else { 0 } // empty / passthrough
}

/// Parse `"size = WxH"` or `"width <= W"` constraints.
fn parse_size_constraints(constraints: &[String], default_w: u32, default_h: u32) -> (u32, u32) {
    let mut w = default_w;
    let mut h = default_h;
    for c in constraints {
        let c = c.to_lowercase();
        // "size = 32x32" or "size=32x32"
        if let Some(rest) = c.strip_prefix("size") {
            let rest = rest.trim_matches(|ch: char| ch == ' ' || ch == '=');
            if let Some((ws, hs)) = rest.split_once('x') {
                if let (Ok(pw), Ok(ph)) = (ws.trim().parse::<u32>(), hs.trim().parse::<u32>()) {
                    w = pw.clamp(8, 128);
                    h = ph.clamp(8, 128);
                }
            }
        }
    }
    (w, h)
}

/// Build a url-safe id from the first 3 meaningful words of the spec.
fn make_id(spec: &str) -> String {
    let stop = ["mit","und","die","der","das","the","a","an","and","or","of",
                "in","with","that","are","is","from","at","by","tile","map"];
    spec.split_whitespace()
        .filter(|w| !stop.contains(&w.as_ref()))
        .take(3)
        .map(|w| w.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AssetBlueprint, AssetIntent};

    fn tile_bp(spec: &str) -> AssetBlueprint {
        AssetBlueprint::new(AssetIntent::TileMap, spec, vec![], 42)
    }

    #[test]
    fn dungeon_map_has_dungeon_tiles() {
        let ir = compile(&tile_bp("dark dungeon cave with stone floors")).unwrap();
        let dungeon_count = ir.tiles.iter().filter(|&&t| t == 5).count();
        assert!(dungeon_count > 0, "should contain dungeon tiles");
    }

    #[test]
    fn default_size_is_16x16() {
        let ir = compile(&tile_bp("forest map")).unwrap();
        assert_eq!(ir.size, (16, 16));
        assert_eq!(ir.tiles.len(), 16 * 16);
    }

    #[test]
    fn custom_size_constraint() {
        let bp = AssetBlueprint::new(
            AssetIntent::TileMap, "dungeon map",
            vec!["size = 32x24".into()], 7,
        );
        let ir = compile(&bp).unwrap();
        assert_eq!(ir.size, (32, 24));
        assert_eq!(ir.tiles.len(), 32 * 24);
    }

    #[test]
    fn border_is_perimeter_tile() {
        let ir = compile(&tile_bp("grass plains map")).unwrap();
        let (w, h) = ir.size;
        // All four border edges must be stone_floor (index 2) for non-dungeon.
        for x in 0..w {
            assert_eq!(ir.tiles[(0 * w + x) as usize], 2, "top border");
            assert_eq!(ir.tiles[((h-1) * w + x) as usize], 2, "bottom border");
        }
    }

    #[test]
    fn deterministic_same_seed() {
        let bp  = tile_bp("snow mountain fortress");
        let ir1 = compile(&bp).unwrap();
        let ir2 = compile(&bp).unwrap();
        assert_eq!(ir1, ir2);
    }

    #[test]
    fn tile_palette_includes_standard_entries() {
        let ir = compile(&tile_bp("forest map")).unwrap();
        assert_eq!(ir.tile_palette[0], "empty");
        assert_eq!(ir.tile_palette[1], "grass");
        assert_eq!(ir.tile_palette[5], "dungeon");
    }
}
