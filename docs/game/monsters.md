# bKG — Monsters

> Types · Stats · Drops · AI · Boss Mechanics

---

## 1. Monster Types

| ID | Name | HP | ATK | DEF | XP | Gold | Biome | Boss |
|---|---|---|---|---|---|---|---|---|
| `wolf` | Wolf | 35 | 9 | 3 | 18 | 4 | grass, dark_forest | — |
| `goblin` | Goblin | 32 | 11 | 5 | 22 | 6 | rock, mountain | — |
| `spider` | Spider | 25 | 14 | 2 | 20 | 5 | dark_forest, swamp | — |
| `skeleton` | Skeleton | 50 | 14 | 8 | 35 | 9 | dungeon | — |
| `rat` | Rat | 18 | 6 | 1 | 9 | 2 | dungeon, swamp | — |
| `troll` | Forest Troll | 60 | 16 | 6 | 42 | 12 | dark_forest, crimson_forest | — |
| `elemental` | Fire Elemental | 45 | 18 | 4 | 40 | 10 | volcanic | — |
| `goblin_chief` | Goblin Chief | 150 | 22 | 12 | 200 | 60 | mountain | ✓ |
| `lich` | Dungeon Lich | 250 | 28 | 16 | 400 | 120 | dungeon | ✓ |

---

## 2. Drop Tables

| Monster | Drops |
|---|---|
| `wolf` | wolf_fang (35%), health_pot (25%), leather_vest (8%) |
| `goblin` | goblin_totem (30%), health_pot (30%), iron_sword (10%) |
| `spider` | health_pot (20%), shadow_cloak (6%) |
| `skeleton` | bone_sword (25%), chain_mail (12%), skull_ring (8%) |
| `rat` | health_pot (15%) |
| `troll` | steel_sword (20%), chain_mail (15%), elixir (10%) |
| `elemental` | obsidian_spear (18%), mana_pot (30%), crystal_plate (10%) |
| `goblin_chief` | steel_sword (90%), crystal_plate (60%), mega_pot (80%) |
| `lich` | obsidian_spear (95%), crystal_plate (90%), elixir (95%) |

---

## 3. AI Behavior

- **Aggro range**: 5.5 tiles — chases player
- **Patrol**: random 0.4-tile wander when idle
- **Blocked**: `deep_water`, `water` tiles impassable
- Boss speed: 1.5× normal

---

## 4. Boss Mechanics

### Phase 2 (at 50% HP)

- ATK × 1.4 permanently
- Screen shake (600ms / 10px)
- Purple particle burst + `ENRAGES!` notification

### Boss HP Bar

Centered at top of screen, 320px wide:
- > 50% HP: purple · 25–50%: orange · < 25%: red

---

## 5. Animation States

All monsters use `standard_character_fsm()`:
`idle → walk → attack → hurt → die`

---

## 6. Planned Future Types

| Biome | Monsters |
|---|---|
| `crimson_forest` | Crystal Bat, Crimson Wraith |
| `snow` | Ice Golem, Frost Wyrm |
| `swamp` | Bog Witch, Mud Elemental |
| `volcanic` | Lava Golem, Ash Fiend |
| `dungeon` | Bone Archer, Shadow Lich (tier 2 boss) |

---

## See Also

- [`engine/wac.md`](../engine/wac.md) — drop table compilation
- [`game/quests.md`](quests.md) — kill quest targets
- [`game/world.md`](world.md) — biome spawn zones
