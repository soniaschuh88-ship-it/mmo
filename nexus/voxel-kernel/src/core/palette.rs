//! MaterialPalette — maps material IDs to their physical and visual properties.
//!
//! The built-in palette covers all standard biome materials.
//! Custom materials (ID ≥ 64) are registered at runtime by AI-generated biomes.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::core::materials;

// ── MaterialFlags ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterialFlags {
    pub solid:       bool,
    pub liquid:      bool,
    pub transparent: bool,
    pub emissive:    bool,
    pub flammable:   bool,
}

impl MaterialFlags {
    pub fn solid() -> Self {
        Self { solid: true, liquid: false, transparent: false, emissive: false, flammable: false }
    }
    pub fn liquid() -> Self {
        Self { solid: false, liquid: true, transparent: true, emissive: false, flammable: false }
    }
    pub fn transparent_solid() -> Self {
        Self { solid: true, liquid: false, transparent: true, emissive: false, flammable: false }
    }
    pub fn emissive_solid() -> Self {
        Self { solid: true, liquid: false, transparent: false, emissive: true, flammable: false }
    }
}

// ── MaterialDef ───────────────────────────────────────────────────────────────

/// Full definition of a voxel material.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialDef {
    /// Stable numeric ID (0 = AIR, 1–63 = builtin, 64+ = custom).
    pub id:        u16,
    /// Human-readable name, used in WAC specs.
    pub name:      String,
    /// RGBA color (used for rendering and minimap).
    pub color:     [u8; 4],
    /// Physical + visual flags.
    pub flags:     MaterialFlags,
    /// Light emission (0–15).
    pub emission:  u8,
    /// Hardness — how many damage units to break (0 = indestructible).
    pub hardness:  f32,
    /// Weight for terrain density calculations (0.0–1.0).
    pub weight:    f32,
}

impl MaterialDef {
    fn builtin(id: u16, name: &str, color: [u8; 4], flags: MaterialFlags, emission: u8, hardness: f32) -> Self {
        Self {
            id, name: name.into(), color, flags, emission, hardness, weight: 1.0,
        }
    }
}

// ── MaterialPalette ───────────────────────────────────────────────────────────

/// Global material registry.
///
/// Built with `MaterialPalette::builtin()`. Custom materials added via
/// `register()` — typically called by the WAC adapter when processing
/// AI-generated biomes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialPalette {
    entries:      BTreeMap<u16, MaterialDef>,
    name_index:   BTreeMap<String, u16>,
    next_custom:  u16,
}

impl MaterialPalette {
    /// Create the palette with all built-in materials pre-registered.
    pub fn builtin() -> Self {
        let mut p = Self { entries: BTreeMap::new(), name_index: BTreeMap::new(), next_custom: materials::FIRST_CUSTOM };

        let entries: &[(u16, &str, [u8;4], MaterialFlags, u8, f32)] = &[
            (materials::AIR,          "air",          [0,0,0,0],         MaterialFlags{solid:false,liquid:false,transparent:true,emissive:false,flammable:false}, 0, 0.0),
            (materials::STONE,        "stone",        [120,120,130,255], MaterialFlags::solid(), 0, 6.0),
            (materials::DIRT,         "dirt",         [101,67,33,255],   MaterialFlags{..MaterialFlags::solid()},   0, 2.0),
            (materials::GRASS,        "grass",        [68,140,50,255],   MaterialFlags{flammable:true,..MaterialFlags::solid()}, 0, 2.0),
            (materials::SAND,         "sand",         [210,180,100,255], MaterialFlags::solid(), 0, 1.5),
            (materials::WATER,        "water",        [30,100,200,180],  MaterialFlags::liquid(), 0, 0.0),
            (materials::GRAVEL,       "gravel",       [150,140,130,255], MaterialFlags::solid(), 0, 2.0),
            (materials::WOOD,         "wood",         [120,80,20,255],   MaterialFlags{flammable:true,..MaterialFlags::solid()}, 0, 3.0),
            (materials::LEAVES,       "leaves",       [30,100,20,200],   MaterialFlags{transparent:true,flammable:true,..MaterialFlags::solid()}, 0, 0.5),
            (materials::SNOW,         "snow",         [240,245,255,255], MaterialFlags::solid(), 0, 1.0),
            (materials::ICE,          "ice",          [180,220,255,200], MaterialFlags::transparent_solid(), 0, 2.0),
            (materials::OBSIDIAN,     "obsidian",     [20,10,30,255],    MaterialFlags::solid(), 0, 50.0),
            (materials::LAVA,         "lava",         [255,90,0,255],    MaterialFlags{emissive:true,..MaterialFlags::liquid()}, 12, 0.0),
            (materials::CRYSTAL_RED,  "crystal_red",  [220,30,50,230],   MaterialFlags::emissive_solid(), 8, 8.0),
            (materials::CRYSTAL_BLUE, "crystal_blue", [30,80,220,220],   MaterialFlags::emissive_solid(), 6, 8.0),
            (materials::NIGHTWOOD,    "nightwood",    [15,10,25,255],    MaterialFlags{flammable:false,..MaterialFlags::solid()}, 0, 4.0),
            (materials::MUSHROOM,     "mushroom",     [180,60,180,255],  MaterialFlags{flammable:true,..MaterialFlags::solid()}, 0, 0.5),
            (materials::MOSS,         "moss",         [40,90,30,255],    MaterialFlags{flammable:true,..MaterialFlags::solid()}, 0, 0.5),
            (materials::NEON_BLOCK,   "neon_block",   [0,255,180,255],   MaterialFlags::emissive_solid(), 15, 4.0),
            (materials::VOID_MATTER,  "void_matter",  [5,0,10,255],      MaterialFlags::solid(), 0, 100.0),
            (materials::MAGMA_ROCK,   "magma_rock",   [180,60,20,255],   MaterialFlags::emissive_solid(), 4, 8.0),
            (materials::CORAL,        "coral",        [255,100,100,255], MaterialFlags{transparent:true,..MaterialFlags::solid()}, 0, 1.5),
            (materials::BONE,         "bone",         [230,220,200,255], MaterialFlags::solid(), 0, 4.0),
            (materials::GLOWSTONE,    "glowstone",    [255,230,100,255], MaterialFlags::emissive_solid(), 15, 3.0),
            (materials::DARK_CRYSTAL, "dark_crystal", [60,0,100,220],    MaterialFlags::emissive_solid(), 5, 10.0),
        ];

        for &(id, name, color, flags, emission, hardness) in entries {
            let def = MaterialDef::builtin(id, name, color, flags, emission, hardness);
            p.name_index.insert(name.to_string(), id);
            p.entries.insert(id, def);
        }
        p
    }

    /// Look up a material by ID.
    pub fn get(&self, id: u16) -> Option<&MaterialDef> {
        self.entries.get(&id)
    }

    /// Resolve a WAC material name to its ID.
    ///
    /// Returns `materials::STONE` as fallback for unknown names.
    pub fn resolve_name(&self, name: &str) -> u16 {
        self.name_index
            .get(name)
            .copied()
            .unwrap_or(materials::STONE)
    }

    /// Register a custom material, returning its assigned ID.
    ///
    /// Called by the WAC adapter when an AI biome spec contains new materials.
    pub fn register(&mut self, name: impl Into<String>, color: [u8; 4], flags: MaterialFlags, emission: u8) -> u16 {
        let id = self.next_custom;
        self.next_custom = self.next_custom.saturating_add(1);
        let name = name.into();
        let def = MaterialDef {
            id, name: name.clone(), color, flags, emission,
            hardness: 5.0, weight: 1.0,
        };
        self.name_index.insert(name, id);
        self.entries.insert(id, def);
        id
    }

    /// Number of registered materials.
    pub fn len(&self) -> usize { self.entries.len() }

    /// True if only built-in materials are registered.
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_palette_complete() {
        let p = MaterialPalette::builtin();
        assert!(p.len() >= 25);
        assert!(p.get(materials::AIR).is_some());
        assert!(p.get(materials::STONE).is_some());
        assert!(p.get(materials::GLOWSTONE).is_some());
    }

    #[test]
    fn name_resolution() {
        let p = MaterialPalette::builtin();
        assert_eq!(p.resolve_name("stone"),       materials::STONE);
        assert_eq!(p.resolve_name("crystal_red"), materials::CRYSTAL_RED);
        assert_eq!(p.resolve_name("unknown"),     materials::STONE); // fallback
    }

    #[test]
    fn custom_material_registration() {
        let mut p = MaterialPalette::builtin();
        let id = p.register("alien_ore", [0, 200, 100, 255], MaterialFlags::emissive_solid(), 7);
        assert!(id >= materials::FIRST_CUSTOM);
        assert_eq!(p.resolve_name("alien_ore"), id);
        assert_eq!(p.get(id).unwrap().emission, 7);
    }

    #[test]
    fn lava_emissive() {
        let p = MaterialPalette::builtin();
        let lava = p.get(materials::LAVA).unwrap();
        assert!(lava.flags.emissive);
        assert_eq!(lava.emission, 12);
    }
}
