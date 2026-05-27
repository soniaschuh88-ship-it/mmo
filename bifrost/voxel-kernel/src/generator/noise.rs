//! Noise primitives — deterministic, seed-based, no external RNG.
//!
//! All functions are pure: same (x, y, seed) → same output on every machine.
//! Used by the terrain and biome generators.

// ── Hash ──────────────────────────────────────────────────────────────────────

/// Deterministic smooth noise in [0, 1] via sin-hash.
/// Numerically equivalent to the JS game's noise — cross-system consistent.
#[inline(always)]
pub fn hash2(x: f64, y: f64, seed: u64) -> f64 {
    let n = (x * 127.1 + y * 311.7 + seed as f64 * 74.3).sin() * 43758.5453;
    n - n.floor()
}

/// 3D variant.
#[inline(always)]
pub fn hash3(x: f64, y: f64, z: f64, seed: u64) -> f64 {
    let n = (x * 127.1 + y * 311.7 + z * 547.3 + seed as f64 * 74.3).sin() * 43758.5453;
    n - n.floor()
}

// ── Smooth interpolation ──────────────────────────────────────────────────────

/// Quintic ease curve — smoothstep for noise.
#[inline(always)]
fn smooth(t: f64) -> f64 { t * t * t * (t * (t * 6.0 - 15.0) + 10.0) }

#[inline(always)]
fn lerp(a: f64, b: f64, t: f64) -> f64 { a + (b - a) * t }

// ── Value noise 2D ────────────────────────────────────────────────────────────

/// Bilinear-interpolated value noise in [0, 1].
pub fn value_noise_2d(x: f64, y: f64, seed: u64) -> f64 {
    let xi = x.floor() as i64;
    let yi = y.floor() as i64;
    let xf = x - xi as f64;
    let yf = y - yi as f64;
    let ux = smooth(xf);
    let uy = smooth(yf);
    let v00 = hash2(xi as f64,       yi as f64,       seed);
    let v10 = hash2(xi as f64 + 1.0, yi as f64,       seed);
    let v01 = hash2(xi as f64,       yi as f64 + 1.0, seed);
    let v11 = hash2(xi as f64 + 1.0, yi as f64 + 1.0, seed);
    lerp(lerp(v00, v10, ux), lerp(v01, v11, ux), uy)
}

/// Trilinear-interpolated value noise in [0, 1].
pub fn value_noise_3d(x: f64, y: f64, z: f64, seed: u64) -> f64 {
    let xi = x.floor() as i64;
    let yi = y.floor() as i64;
    let zi = z.floor() as i64;
    let xf = smooth(x - xi as f64);
    let yf = smooth(y - yi as f64);
    let zf = smooth(z - zi as f64);
    let v000 = hash3(xi as f64,       yi as f64,       zi as f64,       seed);
    let v100 = hash3(xi as f64 + 1.0, yi as f64,       zi as f64,       seed);
    let v010 = hash3(xi as f64,       yi as f64 + 1.0, zi as f64,       seed);
    let v110 = hash3(xi as f64 + 1.0, yi as f64 + 1.0, zi as f64,       seed);
    let v001 = hash3(xi as f64,       yi as f64,       zi as f64 + 1.0, seed);
    let v101 = hash3(xi as f64 + 1.0, yi as f64,       zi as f64 + 1.0, seed);
    let v011 = hash3(xi as f64,       yi as f64 + 1.0, zi as f64 + 1.0, seed);
    let v111 = hash3(xi as f64 + 1.0, yi as f64 + 1.0, zi as f64 + 1.0, seed);
    let x0 = lerp(lerp(v000, v100, xf), lerp(v010, v110, xf), yf);
    let x1 = lerp(lerp(v001, v101, xf), lerp(v011, v111, xf), yf);
    lerp(x0, x1, zf)
}

// ── Fractal Brownian Motion ───────────────────────────────────────────────────

/// fBm (fractal Brownian motion) 2D — sum of octaves, result in [0, 1].
///
/// - `frequency` — initial sampling frequency (try 0.02–0.1)
/// - `octaves`   — number of detail layers (4–8)
/// - `lacunarity` — frequency multiplier per octave (typically 2.0)
/// - `persistence` — amplitude multiplier per octave (typically 0.5)
pub fn fbm_2d(
    x:           f64,
    y:           f64,
    octaves:     u32,
    seed:        u64,
    frequency:   f64,
    lacunarity:  f64,
    persistence: f64,
) -> f64 {
    let mut v = 0.0_f64;
    let mut amp = 0.5_f64;
    let mut freq = frequency;
    let mut max = 0.0_f64;
    for i in 0..octaves {
        v += value_noise_2d(x * freq, y * freq, seed.wrapping_add(i as u64 * 17)) * amp;
        max += amp;
        freq *= lacunarity;
        amp  *= persistence;
    }
    v / max
}

/// fBm 3D.
pub fn fbm_3d(
    x: f64, y: f64, z: f64,
    octaves:     u32,
    seed:        u64,
    frequency:   f64,
    lacunarity:  f64,
    persistence: f64,
) -> f64 {
    let mut v = 0.0_f64;
    let mut amp = 0.5_f64;
    let mut freq = frequency;
    let mut max = 0.0_f64;
    for i in 0..octaves {
        v += value_noise_3d(x*freq, y*freq, z*freq, seed.wrapping_add(i as u64 * 17)) * amp;
        max += amp;
        freq *= lacunarity;
        amp  *= persistence;
    }
    v / max
}

// ── Ridge noise (for mountains, ridges) ───────────────────────────────────────

/// Ridge noise — fBm variant that creates sharp ridges.
/// Output is [0, 1]; values near 1.0 are ridge peaks.
pub fn ridge_noise_2d(x: f64, y: f64, octaves: u32, seed: u64, frequency: f64) -> f64 {
    let mut v   = 0.0_f64;
    let mut amp = 0.5_f64;
    let mut freq = frequency;
    let mut max = 0.0_f64;
    let mut prev = 1.0_f64;
    for i in 0..octaves {
        let n = value_noise_2d(x * freq, y * freq, seed.wrapping_add(i as u64 * 17));
        let ridged = (1.0 - n.abs() * 2.0 - 1.0).abs(); // fold
        v   += ridged * amp * prev;
        max += amp;
        prev = ridged;
        freq *= 2.0;
        amp  *= 0.5;
    }
    (v / max).clamp(0.0, 1.0)
}

// ── Worley / Cellular noise ───────────────────────────────────────────────────

/// Worley noise (F1 distance) in [0, 1] — creates organic cell-like patterns.
/// Useful for biome boundaries and cave systems.
pub fn worley_2d(x: f64, y: f64, seed: u64) -> f64 {
    let xi = x.floor() as i64;
    let yi = y.floor() as i64;
    let mut min_dist = f64::MAX;
    for dy in -1i64..=1 {
        for dx in -1i64..=1 {
            let cx = (xi + dx) as f64;
            let cy = (yi + dy) as f64;
            let px = cx + hash2(cx, cy, seed);
            let py = cy + hash2(cx, cy, seed.wrapping_add(1));
            let dist = (x - px).powi(2) + (y - py).powi(2);
            if dist < min_dist { min_dist = dist; }
        }
    }
    min_dist.sqrt().clamp(0.0, 1.0)
}

// ── Domain-warped noise ───────────────────────────────────────────────────────

/// Domain-warped fBm — feeds noise output back as input offset.
/// Creates flowing, organic terrain features.
pub fn warped_fbm_2d(x: f64, y: f64, octaves: u32, seed: u64, frequency: f64) -> f64 {
    let wx = fbm_2d(x, y, 4, seed,           frequency, 2.0, 0.5);
    let wy = fbm_2d(x, y, 4, seed + 1000, frequency, 2.0, 0.5);
    fbm_2d(x + wx * 2.0, y + wy * 2.0, octaves, seed + 2000, frequency, 2.0, 0.5)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash2_range() {
        for i in 0..100 {
            let h = hash2(i as f64 * 1.1, i as f64 * 2.7, 42);
            assert!(h >= 0.0 && h <= 1.0, "h={h} out of range");
        }
    }

    #[test]
    fn value_noise_range() {
        for i in 0..50 {
            let n = value_noise_2d(i as f64 * 0.3, i as f64 * 0.7, 123);
            assert!(n >= 0.0 && n <= 1.0, "n={n}");
        }
    }

    #[test]
    fn fbm_deterministic() {
        let a = fbm_2d(3.14, 2.71, 6, 42, 0.05, 2.0, 0.5);
        let b = fbm_2d(3.14, 2.71, 6, 42, 0.05, 2.0, 0.5);
        assert_eq!(a, b);
    }

    #[test]
    fn fbm_range() {
        for i in 0..100 {
            let n = fbm_2d(i as f64 * 0.7, i as f64 * 1.3, 6, 42, 0.05, 2.0, 0.5);
            assert!(n >= 0.0 && n <= 1.0, "fbm={n}");
        }
    }

    #[test]
    fn worley_range() {
        for i in 0..50 {
            let n = worley_2d(i as f64 * 0.4, i as f64 * 0.9, 7);
            assert!(n >= 0.0 && n <= 1.0, "worley={n}");
        }
    }

    #[test]
    fn ridge_range() {
        for i in 0..50 {
            let n = ridge_noise_2d(i as f64 * 0.3, i as f64 * 0.5, 5, 99, 0.04);
            assert!(n >= 0.0 && n <= 1.0, "ridge={n}");
        }
    }

    #[test]
    fn different_seeds_different_output() {
        let a = fbm_2d(1.0, 1.0, 4, 1, 0.1, 2.0, 0.5);
        let b = fbm_2d(1.0, 1.0, 4, 2, 0.1, 2.0, 0.5);
        assert_ne!(a, b);
    }
}
