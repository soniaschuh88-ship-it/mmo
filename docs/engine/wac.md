# bKG — World Asset Compiler (WAC)

> LLM or designer intent → strictly typed, validated, deterministic asset IR → BIFROST world.

---

## 1. Core Rule

```
LLM MAY:              LLM MAY NOT:
  describe rules        set voxels directly
  define constraints    spawn loot directly
  specify semantics     define animation frames
  provide a seed        mutate world state
```

LLM output **never touches the engine directly**. It always goes through validation and compilation.

---

## 2. Pipeline

```
LLM / Designer / AI Faction Intent
          ↓
  AssetBlueprint  (spec + constraints + seed)
          ↓
  validate()   ← WacError on violation
          ↓
  compile()    → AssetIR (typed, version-stamped)
          ↓
  AssetCache   (BLAKE3 key = semantic hash)
          ↓
  Runtime Translators:
    voxel_engine · loot_engine · animation_engine · biome_engine
          ↓
  BIFROST WORLD
```

---

## 3. Asset Intent Types

| Intent | Description |
|---|---|
| `TileMap` | 2-D tile map layout (structure, dungeon, base) |
| `BiomeDefinition` | Terrain generation rules, materials, entity spawns |
| `LootTable` | Drop table with conditional entries |
| `AnimationGraph` | Entity animation state machine |
| `EntityPrefab` | Fully specified entity (stats + behavior + animation) |

---

## 4. Asset Blueprint

```rust
pub struct AssetBlueprint {
    pub id:                    Uuid,
    pub asset_type:            AssetIntent,
    pub natural_language_spec: String,   // "ein biome mit roten kristallwäldern"
    pub constraints:           Vec<String>, // "no floating tiles"
    pub seed:                  u64,      // must not be zero
}
```

---

## 5. Biome System — bifrost-wac is the World Type Authority

Biome definitions live in `bifrost/wac/src/biomes.rs`. Every system reads from `BiomeRegistry::global()`.

```rust
pub struct BiomeDefinition {
    pub id:                      BiomeKey,
    pub temperature:             f32,
    pub humidity:                f32,
    pub colors:                  (&'static str, &'static str, &'static str),
    pub ambient_fx:              AmbientFx,
    pub risk_tier:               u8,
    pub hostile_density:         f32,
    pub strategic_value:         f32,  // Synthesis AI scoring
    pub loot_weight_multiplier:  f32,
    pub mutation_cost:           f32,  // World Director budget
    pub passable:                bool,
    pub voxel_fill_rate:         f32,
}
```

### Canonical Biome IDs

| Index | ID | Display |
|---|---|---|
| 0 | `deep_water` | Deep Sea |
| 1 | `water` | Shore |
| 2 | `sand` | Sandy Banks |
| 3 | `grass` | Green Plains |
| 4 | `dark_forest` | Dark Forest |
| 5 | `crimson_forest` | Crimson Forest |
| 6 | `rock` | Rocky Highlands |
| 7 | `mountain` | Mountains |
| 8 | `snow` | Frost Peaks |
| 9 | `dungeon` | The Dungeon |
| 10 | `village` | Village |
| 11 | `building` | Building |
| 12 | `swamp` | Swamp |
| 13 | `volcanic` | Volcanic Wastes |

Index = tile-palette index shared between Rust and the JS `BIOME` constant.

---

## 6. Loot System

LLM defines **loot logic**, not items:

```
"rare crystals drop from glowing bats at night"
```

WAC compiles to:

```rust
LootTable {
    entries: [
        { item: "crystal_shard", drop_rate: 0.03,
          conditions: [Night, BatKill, Biome(CrimsonForest)] }
    ]
}
```

---

## 7. Animation Graph

WAC produces state machine IR that `nova-anim::AnimStateMachine` can consume directly:

```json
{
  "states": ["idle", "search", "attack", "flee"],
  "transitions": [
    { "from": "idle",   "to": "search", "condition": "player_near" },
    { "from": "search", "to": "attack", "condition": "enemy_visible" }
  ]
}
```

---

## 8. Asset Cache

Identical intent + seed → identical world:

```rust
pub struct AssetCache {
    // BLAKE3(spec + constraints + seed) → compiled AssetIR
}
```

Same natural-language spec with the same seed always produces the same world, no matter who calls it.

---

## 9. NVIDIA NIM Integration

The `nvidia-nim` feature enables LLM-backed spec generation:

```
Synthesis AI Intent → NimClient::generate_asset_spec() → natural_language_spec → WAC
```

Environment variables:
- `NVIDIA_API_KEY` — bearer token
- `NVIDIA_NIM_BASE_URL` — default: `https://integrate.api.nvidia.com/v1`
- `NVIDIA_NIM_MODEL` — default: `meta/llama-3.3-70b-instruct`

---

## See Also

- [`docs/WORLD.md`](WORLD.md) — World generation pipeline
- [`docs/BIFROST-SPEC.md`](BIFROST-SPEC.md) — Bifrost protocol
