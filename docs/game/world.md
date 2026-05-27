# bKG — World Design

> The world is not an endless map. It is a **discrete competitive epoch** that evolves, resolves, and regenerates.

---

## 1. The Run System

The world operates in **runs** — discrete competitive seasons with a fixed duration or win condition.

```
Run N
  │
  ├── Human factions compete for zone control
  ├── Synthesis AI civilization competes using the same rules
  ├── Resources extracted, bases built, territory fought over
  │
  └── EndCondition reached
        │
        ├── Winner effects applied
        ├── World Director generates next world
        └── Run N+1 begins
```

### End Conditions

| Condition | Description |
|---|---|
| `FirstToControlZones(n)` | Control *n* zones simultaneously |
| `FirstToReachTechLevel(n)` | Advance faction tech tree to level *n* |
| `EconomicDominance(f)` | Control fraction *f* of total auction house volume |
| `SurvivalUntilTime(t)` | Survive until tick *t* |

Multiple victory paths prevent a single dominant meta strategy.

---

## 2. World Generation Pipeline

Each run end triggers deterministic world regeneration:

```
Run End
  → WorldRunDirector evaluates result
  → PressureGraph built from run outcome
  → WorldDirector ticks (WAC blueprint emission)
  → WAC compiles biome + loot + faction assets
  → nexus-voxel-kernel generates new world chunks
  → Run N+1 begins
```

The AI **analyzes the previous run meta** and generates **counter-worlds**:

| Previous dominant strategy | Next world adaptation |
|---|---|
| Economy exploitation | Scarcity biomes, volatile markets |
| Fortress turtling | Open terrain, mobile resource nodes |
| Zerg rush | Defensive biome evolution, choke points |

---

## 3. Winner / Loser Effects

### Winners receive
- Permanent meta-progression unlocks
- Rare loot injection into account vault
- Cosmetic and functional world perks
- Access to next tier world

### Losers receive
- Soft skill decay (not a full wipe)
- Reduced starting resources next run
- Reputation penalty in AI systems

> No hard wipe. Meta-progression **imbalance**, not destruction.

---

## 4. Meta Progression System

Two separate progression layers:

| Layer | Scope | Contains |
|---|---|---|
| **Run Progression** | Resets per run | Skills, gear, bases, territory |
| **Meta Progression** | Persistent | Unlocks, archetypes, starting perks, faction tech trees |

`PLAYER POWER = RUN STATE + META STATE`

---

## 5. Zone Control System

```
SAFE CITY ──── permanent anchor, no combat, auction house
OUTER ZONES ── contested, war economy, medium risk
DEEP ZONES ─── high risk, high reward, boss encounters
```

### Zone States

| State | Description |
|---|---|
| `Safe` | Protected zone, no combat events |
| `Contested` | Multiple factions competing for influence |
| `Controlled(faction)` | Single faction holds majority influence |
| `Collapsing` | Resources exhausted, biome corrupting |

Zones change ownership through influence accumulation, infrastructure buildup, combat resolution, and economic pressure.

---

## 6. Safe City

The Safe City is the persistent cross-run anchor:

- **Survives across all runs** — never destroyed, never captured
- **Auction house** — the only global market; all trades route through it
- **Crafting hub** — WAC-powered structure creation
- **Respawn anchor** — guaranteed respawn point
- **Skill progression** — meta progression happens here

### Why Safe City Matters

Without it: `chaos + inflation + AI dominance`  
With it: `controlled chaos with stable meta-economy`

The Auction House prevents:
- Inflation exploits
- Duplication loops
- AI economy collapse

---

## 7. Biome Evolution During a Run

Biomes are **not static**. They evolve in response to world events:

| Trigger | Biome Effect |
|---|---|
| Combat intensity | Terrain corruption, crater formation |
| Economy imbalance | Resource mutation, scarcity zones |
| AI pressure | Defensive biome adaptation |
| Player inaction | Forest overgrowth, reclamation |

---

## 8. Economy Model

```
SAFE CITY:   stable economy · crafting hub · respawn anchor
OUTER ZONES: volatile economy · faction influence · loot-driven survival
DEEP ZONES:  high-risk loot · boss mechanics · no respawn guarantee
```

Loot is **not static** — it is generated per run based on:
- Current biome state
- Faction dominance
- AI adaptation
- World Director pressure graph

---

## 9. Player Entity Model

```rust
pub struct PlayerEntity {
    pub id:            PlayerId,
    pub body:          Option<BodyId>,
    pub clone_charges: u32,
    pub memory_core:   MemoryGraph,
}
```

### Death Flow

1. Body dies
2. Memory snapshot saved
3. Inventory split: lost items (world drops) + secured items (safe city vault)
4. Clone spawns if charges available — same memory graph, small entropy drift

> No true permadeath for progression, but real loss for risk-taking.

---

## 10. World Tick Loop

```
Each Tick:
  1. World State Snapshot  (bifrost-lockstep)
  2. Synthesis AI Planning (bifrost-synthesis)
  3. Player Intent Collection
  4. Safe City routing     (bifrost-safe-city)
  5. WAC compilation       (bifrost-wac)
  6. Physics + Economy resolution (bifrost-physics)
  7. Persistence           (event ledger)
```

---

## See Also

- [`docs/FACTION.md`](FACTION.md) — Synthesis AI civilization design
- [`docs/WAC.md`](WAC.md) — World Asset Compiler pipeline
- [`docs/BIFROST-SPEC.md`](BIFROST-SPEC.md) — Protocol specification
