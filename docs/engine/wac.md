# bKG — World Asset Compiler (WAC)

> LLM or designer intent → strictly typed, validated, deterministic asset IR → Bifrost world.

---

## 1. Core Rule

```
LLM MAY:              LLM MAY NOT:
  describe rules        set voxels directly
  define constraints    spawn loot directly
  specify semantics     define animation frames
  provide a seed        mutate world state
```

LLM output **never touches the engine directly**. Every change goes through validate → compile.

---

## 2. Pipeline

```
LLM / Designer / Synthesis AI Intent
          ↓
  AssetBlueprint  (spec + constraints + seed)
          ↓
  validate()   ← WacError on violation
          ↓
  compile()    → AssetIR (typed, version-stamped)
          ↓
  AssetCache   (BLAKE3 key = semantic hash)
          ↓
  Runtime translators:
    voxel_engine · loot_engine · animation_engine · biome_engine
          ↓
  Bifrost World
```

---

## 3. Asset Types

| Intent | Description |
|---|---|
| `TileMap` | 2-D tile layout (dungeon, base, structure) |
| `BiomeDefinition` | Terrain rules, materials, entity spawns |
| `LootTable` | Drop table with conditional entries |
| `AnimationGraph` | Entity animation state machine |
| `EntityPrefab` | Full entity spec (stats + behavior + animation) |

---

## 4. AssetBlueprint

```rust
pub struct AssetBlueprint {
    pub id:                    Uuid,
    pub asset_type:            AssetIntent,
    pub natural_language_spec: String,   // "crimson crystal forest with nocturnal glow"
    pub constraints:           Vec<String>, // "no floating tiles"
    pub seed:                  u64,
}
```

---

## 5. Biome Registry — bifrost-wac is the World Type Authority

All biome definitions live in `bifrost/wac/src/biomes.rs`.
Every system reads from `BiomeRegistry::global()` — no scattered constants.

### Canonical Biome IDs

| Index | ID | Display | Risk |
|---|---|---|---|
| 0 | `deep_water` | Deep Sea | 0 |
| 1 | `water` | Shore | 0 |
| 2 | `sand` | Sandy Banks | 1 |
| 3 | `grass` | Green Plains | 1 |
| 4 | `dark_forest` | Dark Forest | 1 |
| 5 | `crimson_forest` | Crimson Forest | 2 |
| 6 | `rock` | Rocky Highlands | 1 |
| 7 | `mountain` | Mountains | 2 |
| 8 | `snow` | Frost Peaks | 2 |
| 9 | `dungeon` | The Dungeon | 3 |
| 10 | `village` | Village | 0 |
| 11 | `building` | Building | 0 |
| 12 | `swamp` | Swamp | 2 |
| 13 | `volcanic` | Volcanic Wastes | 3 |

Index = tile-palette index, shared between Rust and the JS `BIOME` constant.

### BiomeDefinition fields

```rust
pub struct BiomeDefinition {
    pub id:                      BiomeKey,
    pub temperature / humidity:  f32,      // nexus terrain generator
    pub colors:                  (...),    // nova-render + JS BC constant
    pub ambient_fx:              AmbientFx,// nova-render overlay
    pub risk_tier:               u8,       // Synthesis AI + spawn tables
    pub hostile_density:         f32,
    pub strategic_value:         f32,      // AI faction scoring
    pub loot_weight_multiplier:  f32,
    pub mutation_cost:           f32,      // world director budget
    pub passable:                bool,
    pub voxel_fill_rate:         f32,
}
```

---

## 6. Loot System

LLM defines **loot logic**, not items directly:

```
"rare crystals drop from glowing bats at night"
```

WAC compiles to:

```rust
LootTable {
    entries: [{
        item: "crystal_shard", drop_rate: 0.03,
        conditions: [Night, BatKill, Biome(CrimsonForest)]
    }]
}
```

See [`game/monsters.md`](../game/monsters.md) for per-monster drop tables.

---

## 7. Animation Graph

WAC produces FSM IR that `nova-anim::AnimStateMachine` consumes directly:

```json
{
  "states": ["idle", "search", "attack", "flee"],
  "transitions": [
    { "from": "idle",   "to": "search", "condition": "player_near" },
    { "from": "search", "to": "attack", "condition": "enemy_visible" }
  ]
}
```

See [`game/monsters.md`](../game/monsters.md) and [`engine/client-runtime.md`](client-runtime.md).

---

## 8. Asset Cache

Same spec + same seed → same world, always:

```rust
// BLAKE3(natural_language_spec + constraints + seed) → AssetIR
pub struct AssetCache { ... }
```

---

## 9. NVIDIA NIM Integration

`nvidia-nim` feature enables LLM-backed spec generation:

```
Synthesis AI Intent → NimClient::generate_asset_spec() → spec → WAC pipeline
```

| Variable | Default | Description |
|---|---|---|
| `NVIDIA_API_KEY` | — | Bearer token |
| `NVIDIA_NIM_BASE_URL` | `https://integrate.api.nvidia.com/v1` | Endpoint |
| `NVIDIA_NIM_MODEL` | `meta/llama-3.3-70b-instruct` | Model |

---

## See Also

- [`engine/architecture.md`](architecture.md) — System overview
- [`game/world.md`](../game/world.md) — World generation pipeline
- [`game/monsters.md`](../game/monsters.md) — Monster drop tables
