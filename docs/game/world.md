# bKG — World Design

> The world is not an endless map. It is a **discrete competitive epoch** — a run.

---

## 1. The Run System

Each run is a competitive season:

```
Run N
  ├── Human factions claim zones, build bases, fight for dominance
  ├── Synthesis AI competes using the same rules
  ├── EndCondition is evaluated each tick
  └── Run ends → World Director generates Run N+1
```

### End Conditions

| Condition | Description |
|---|---|
| `FirstToControlZones(n)` | Hold *n* zones simultaneously |
| `FirstToReachTechLevel(n)` | Advance tech tree to level *n* |
| `EconomicDominance(f)` | Control fraction *f* of auction house volume |
| `SurvivalUntilTime(t)` | Survive to tick *t* |

Multiple victory paths prevent a single dominant meta strategy.

---

## 2. World Generation Pipeline

```
Run End
  → WorldRunDirector evaluates result
  → PressureGraph from run outcome
  → WorldDirector ticks (WAC blueprint emission)
  → WAC compiles biome + loot + faction assets
  → nexus-voxel-kernel generates chunks
  → Run N+1 begins
```

The AI **analyzes the previous run meta** and generates **counter-worlds**:

| Player dominated via | Next world adapts with |
|---|---|
| Economy exploitation | Scarcity biomes, volatile markets |
| Fortress turtling | Open terrain, mobile resource nodes |
| Zerg rush | Defensive biome evolution, choke points |

---

## 3. Zone Layout

```
SAFE CITY ──── permanent anchor, no combat, auction house
OUTER ZONES ── contested, war economy, medium risk
DEEP ZONES ─── high risk, high reward, boss encounters
```

### Zone States

| State | Description |
|---|---|
| `Safe` | Protected, no combat |
| `Contested` | Multiple factions competing |
| `Controlled(faction)` | Single faction holds majority |
| `Collapsing` | Resources exhausted, biome corrupting |

---

## 4. Safe City

The permanent cross-run anchor:

- **Never destroyed or captured**
- **Auction House** — only global market; all trades route through here
- **Crafting hub** — WAC-powered structure creation
- **Respawn anchor** — guaranteed after death
- **Meta progression** — skill unlocks happen here

---

## 5. Biome Evolution During a Run

Biomes mutate in response to world events:

| Trigger | Effect |
|---|---|
| Combat intensity | Terrain corruption, craters |
| Economy imbalance | Resource mutation, scarcity zones |
| AI pressure | Defensive biome adaptation |
| Player inaction | Forest overgrowth, reclamation |

---

## 6. Meta Progression

| Layer | Scope | Contains |
|---|---|---|
| Run Progression | Resets each run | Skills, gear, bases, territory |
| Meta Progression | Persistent | Unlocks, archetypes, starting perks |

`PLAYER POWER = RUN STATE + META STATE`

---

## 7. Economy Model

Loot is generated per run, based on:
- Current biome state
- Faction dominance
- AI adaptation
- World Director pressure graph

Economy zones:
```
Safe City:   stable prices, crafting hub, skill trading
Outer Zones: volatile prices, faction influence, loot-driven
Deep Zones:  highest drop rates, boss loot, no guarantees
```

---

## 8. World Tick Loop

```
Each Tick:
  1. World State Snapshot    (bifrost-lockstep)
  2. Synthesis AI Planning   (bifrost-synthesis)
  3. Player Intent Collection
  4. Safe City routing       (bifrost-safe-city)
  5. WAC compilation         (bifrost-wac)
  6. Physics + Economy       (bifrost-physics)
  7. Persistence             (event ledger)
```

---

## See Also

- [`game/factions.md`](factions.md) — Synthesis AI design
- [`engine/wac.md`](../engine/wac.md) — World Asset Compiler
- [`engine/bifrost-protocol.md`](../engine/bifrost-protocol.md) — Protocol spec
