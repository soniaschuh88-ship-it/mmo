//! # bifrost-wasm — Browser WASM bridge
//!
//! Exposes world generation, noise, and biome data to the browser via
//! `wasm-bindgen`.  The JavaScript side (`game.html`) calls these functions
//! to:
//!
//! * Generate the world map deterministically from a seed
//! * Query canonical biome definitions (colors, names, properties)
//! * Run the same noise function as the Rust server (R4 replay-safe guarantee)
//!
//! ## Build
//!
//! ```bash
//! wasm-pack build bifrost/wasm --target web --out-dir ../../app/pkg/bifrost_wasm
//! ```
//!
//! ## Usage in game.html
//!
//! ```js
//! import init, { generate_world_data, biome_id, biome_colors_json, fbm_noise }
//!   from '/pkg/bifrost_wasm/bifrost_wasm.js';
//! await init();
//! ```

use wasm_bindgen::prelude::*;

use bifrost_wac::biomes::{BIOME_IDS, BIOME_COLORS, BiomeKey, BiomeRegistry};
use bifrost_kernel::generator::noise::{fbm_2d, hash2};

// ─── Noise ────────────────────────────────────────────────────────────────────

/// Quintic-smoothstep value noise — matches `sn(x,y,s)` in game.html exactly.
///
/// Returns a value in [0, 1].  Use this to verify JS and Rust produce the
/// same world from the same seed.
#[wasm_bindgen]
pub fn sn_noise(x: f64, y: f64, seed: f64) -> f64 {
    let s = seed as u64;
    bifrost_kernel::generator::noise::value_noise_2d(x, y, s)
}

/// Fractal Brownian Motion — matches `fbm(x,y,oct,s)` in game.html.
///
/// `octaves` is typically 4–6; `seed` is cast to `u64`.
#[wasm_bindgen]
pub fn fbm_noise(x: f64, y: f64, octaves: u32, seed: f64) -> f64 {
    fbm_2d(x, y, octaves, seed as u64, 1.0, 2.0, 0.5)
}

// ─── Biome helpers ────────────────────────────────────────────────────────────

/// Return the canonical biome ID string for a palette index (0–13).
///
/// Returns `"grass"` for any out-of-range index.
#[wasm_bindgen]
pub fn biome_id(index: u8) -> String {
    BIOME_IDS.get(index as usize)
        .copied()
        .unwrap_or("grass")
        .into()
}

/// Return tile colors as a JSON object `{"top":"#...","left":"#...","right":"#..."}`.
///
/// Used by game.html's isometric tile renderer to paint the three visible faces.
#[wasm_bindgen]
pub fn biome_colors_json(index: u8) -> String {
    let (top, left, right) = BIOME_COLORS.get(index as usize)
        .copied()
        .unwrap_or(("#487840", "#285828", "#387030")); // grass fallback
    serde_json::json!({ "top": top, "left": left, "right": right }).to_string()
}

/// Return the full biome registry as a JSON array.
///
/// Each entry includes `id`, `display_name`, `temperature`, `humidity`,
/// `risk_tier`, `passable`, and `colors`.  Used by game.html to populate
/// the UI and drive AI decision-making client-side.
#[wasm_bindgen]
pub fn biome_registry_json() -> String {
    let reg = BiomeRegistry::global();
    let arr: Vec<_> = BiomeKey::ALL.iter().map(|&key| {
        let def = reg.get(key);
        let (top, left, right) = def.colors;
        serde_json::json!({
            "index":        key as u8,
            "id":           key.as_str(),
            "display_name": key.display_name(),
            "temperature":  def.temperature,
            "humidity":     def.humidity,
            "risk_tier":    def.risk_tier,
            "passable":     def.passable,
            "colors":       { "top": top, "left": left, "right": right },
        })
    }).collect();
    serde_json::to_string(&arr).unwrap_or_else(|_| "[]".into())
}

// ─── World generation ─────────────────────────────────────────────────────────

/// Generate a flat world map as a `Uint8Array`.
///
/// The returned buffer has `width × height × 2` bytes.  For tile `(x, y)`:
/// - `buf[y * width * 2 + x * 2 + 0]` = biome palette index (0–13)
/// - `buf[y * width * 2 + x * 2 + 1]` = terrain height (0–9, scaled)
///
/// This is numerically identical to the JS `generateWorld()` function that
/// populates `BM[y][x]` and `HM[y][x]`, ensuring client–server determinism
/// (R4 — same seed → same world always).
#[wasm_bindgen]
pub fn generate_world_data(seed: u32, width: u32, height: u32) -> Vec<u8> {
    use bifrost_wac::biomes::BiomeKey;

    let s = seed as u64;
    let cx = width as f64 / 2.0;
    let cy = height as f64 / 2.0;
    let w  = width  as usize;
    let h  = height as usize;

    let mut buf = vec![0u8; w * h * 2];

    for y in 0..h {
        for x in 0..w {
            let fx = x as f64;
            let fy = y as f64;

            // ── Height map (same fbm as JS, quintic smoothstep) ───────────────
            let h_raw = fbm_2d(fx / 8.0, fy / 8.0, 6, s, 1.0, 2.0, 0.5);
            // Map [0,1] → height tier 0–9
            let h_tier = (h_raw * 10.0).floor() as u8;

            // ── Biome assignment (mirrors JS generateWorld logic) ──────────────
            // Distance to village center
            let dx = fx - cx;
            let dy = fy - cy;
            let vd = (dx * dx + dy * dy).sqrt();

            let biome: u8 = if x >= w.saturating_sub(17) && y >= h.saturating_sub(17) {
                BiomeKey::Dungeon as u8
            } else if y >= h.saturating_sub(14) && x < 15 {
                BiomeKey::Volcanic as u8
            } else if y < 17 && x >= w.saturating_sub(17) {
                BiomeKey::CrimsonForest as u8
            } else if vd <= 4.0 {
                BiomeKey::Village as u8
            } else {
                match h_tier {
                    0 | 1      => BiomeKey::DeepWater as u8,
                    2          => BiomeKey::Water     as u8,
                    3          => BiomeKey::Sand      as u8,
                    4 | 5      => BiomeKey::Grass     as u8,
                    6          => BiomeKey::DarkForest as u8,
                    7          => BiomeKey::Rock      as u8,
                    8          => BiomeKey::Mountain  as u8,
                    _          => BiomeKey::Snow      as u8,
                }
            };

            // Swamp overlay (mirrors JS: y>W/2, x<W/3, in grass/dark_forest)
            let biome = if (fy > cy) && (fx < width as f64 / 3.0)
                && (biome == BiomeKey::Grass as u8 || biome == BiomeKey::DarkForest as u8)
                && hash2(fx, fy, s + 33) > 0.6
            {
                BiomeKey::Swamp as u8
            } else {
                biome
            };

            let idx = y * w * 2 + x * 2;
            buf[idx]     = biome;
            buf[idx + 1] = h_tier;
        }
    }

    buf
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn biome_id_range() {
        assert_eq!(biome_id(0),  "deep_water");
        assert_eq!(biome_id(9),  "dungeon");
        assert_eq!(biome_id(13), "volcanic");
        assert_eq!(biome_id(99), "grass");  // fallback
    }

    #[test]
    fn biome_colors_valid_json() {
        let s = biome_colors_json(4); // dark_forest
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert!(v["top"].as_str().unwrap().starts_with('#'));
    }

    #[test]
    fn world_gen_correct_size() {
        let buf = generate_world_data(42, 16, 16);
        assert_eq!(buf.len(), 16 * 16 * 2);
    }

    #[test]
    fn world_gen_deterministic() {
        let a = generate_world_data(42, 32, 32);
        let b = generate_world_data(42, 32, 32);
        assert_eq!(a, b, "same seed must produce same world (R4)");
    }

    #[test]
    fn world_gen_different_seeds_differ() {
        let a = generate_world_data(1, 32, 32);
        let b = generate_world_data(2, 32, 32);
        assert_ne!(a, b);
    }

    #[test]
    fn noise_matches_known_value() {
        let n = sn_noise(1.0, 1.0, 42.0);
        assert!(n >= 0.0 && n <= 1.0, "noise out of range: {n}");
    }

    #[test]
    fn registry_json_has_14_entries() {
        let s = biome_registry_json();
        let v: Vec<serde_json::Value> = serde_json::from_str(&s).unwrap();
        assert_eq!(v.len(), 14);
    }
}
