//! Named material constants — stable IDs, part of the WAC wire format.
//!
//! IDs 0–63 are reserved for built-in materials.
//! IDs 64–65535 are available for AI-generated custom materials.
pub const AIR:           u16 = 0;
pub const STONE:         u16 = 1;
pub const DIRT:          u16 = 2;
pub const GRASS:         u16 = 3;
pub const SAND:          u16 = 4;
pub const WATER:         u16 = 5;
pub const GRAVEL:        u16 = 6;
pub const WOOD:          u16 = 7;
pub const LEAVES:        u16 = 8;
pub const SNOW:          u16 = 9;
pub const ICE:           u16 = 10;
pub const OBSIDIAN:      u16 = 11;
pub const LAVA:          u16 = 12;
pub const CRYSTAL_RED:   u16 = 13;
pub const CRYSTAL_BLUE:  u16 = 14;
pub const NIGHTWOOD:     u16 = 15;
pub const MUSHROOM:      u16 = 16;
pub const MOSS:          u16 = 17;
pub const NEON_BLOCK:    u16 = 18;
pub const VOID_MATTER:   u16 = 19;
pub const MAGMA_ROCK:    u16 = 20;
pub const CORAL:         u16 = 21;
pub const BONE:          u16 = 22;
pub const GLOWSTONE:     u16 = 23;
pub const DARK_CRYSTAL:  u16 = 24;
pub const FIRST_CUSTOM:  u16 = 64; // first available custom ID
