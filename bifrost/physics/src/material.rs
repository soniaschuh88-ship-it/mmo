//! Material IDs and properties.
//!
//! Materials are identified by a `u8` ID. The ID table is part of the
//! wire format — never change existing IDs, only add new ones.

use serde::{Deserialize, Serialize};

// ─── Canonical material IDs ────────────────────────────────────────────────────

/// Empty space. Voxels absent from the world map are implicitly air.
pub const MAT_AIR:   u8 = 0;
pub const MAT_STONE: u8 = 1;
pub const MAT_DIRT:  u8 = 2;
pub const MAT_GRASS: u8 = 3;
pub const MAT_WOOD:  u8 = 4;
pub const MAT_WATER: u8 = 5;
pub const MAT_SAND:  u8 = 6;
pub const MAT_IRON:  u8 = 7;

// ─── Material properties ───────────────────────────────────────────────────────

/// Static properties for a material type.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MaterialProps {
    /// Base durability (how much damage before the voxel is destroyed).
    pub durability: u16,
    /// Whether this material can catch fire.
    pub flammable: bool,
    /// Whether this material flows like a liquid.
    pub liquid: bool,
    /// Relative density (1.0 = stone, lower = lighter).
    pub density: f64,
}

impl MaterialProps {
    /// Properties for a given material ID. Returns stone properties for unknown IDs.
    pub fn for_material(id: u8) -> Self {
        match id {
            MAT_AIR   => Self { durability: 0,      flammable: false, liquid: false, density: 0.0   },
            MAT_STONE => Self { durability: 10_000,  flammable: false, liquid: false, density: 1.0   },
            MAT_DIRT  => Self { durability: 2_000,   flammable: false, liquid: false, density: 0.8   },
            MAT_GRASS => Self { durability: 1_500,   flammable: true,  liquid: false, density: 0.7   },
            MAT_WOOD  => Self { durability: 5_000,   flammable: true,  liquid: false, density: 0.6   },
            MAT_WATER => Self { durability: 0,       flammable: false, liquid: true,  density: 0.5   },
            MAT_SAND  => Self { durability: 1_000,   flammable: false, liquid: false, density: 0.9   },
            MAT_IRON  => Self { durability: 30_000,  flammable: false, liquid: false, density: 2.5   },
            // Unknown materials default to stone-like
            _         => Self { durability: 10_000,  flammable: false, liquid: false, density: 1.0   },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn air_has_zero_durability() {
        assert_eq!(MaterialProps::for_material(MAT_AIR).durability, 0);
    }

    #[test]
    fn wood_is_flammable() {
        assert!(MaterialProps::for_material(MAT_WOOD).flammable);
        assert!(!MaterialProps::for_material(MAT_STONE).flammable);
    }
}
