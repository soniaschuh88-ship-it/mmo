# bKG — Factions

> Not NPCs. A **distributed strategic civilization** competing against humans using the same rules.

---

## 1. Faction Overview

| Faction | Type | Strategy |
|---|---|---|
| Human guilds | Player-controlled | Individual, creative, emergent |
| **Synthesis** | AI civilization | Coordinated, long-term, global-optimized |

The Synthesis faction is **not an NPC faction**. It operates with the same rights, same rules, and same world-manipulation interface as human players.

---

## 2. Symmetry Guarantee

Both factions use the identical pipeline:

```
Intent → WAC Blueprint → validate() → compile() → World mutation
```

The AI has no special world access and cannot cheat.

---

## 3. Synthesis Structure

```
1 SynthesisCore   ── global strategist (NVIDIA NIM backed)
N SubAI nodes     ── region controllers
M AgentNodes      ── squad / clan level actors
```

```rust
pub struct AiFaction {
    pub economy:        EconomyGraph,
    pub territory:      Vec<ZoneId>,
    pub agents:         Vec<AgentNode>,
    pub strategy_model: WorldModel,
    pub memory:         FactionMemoryGraph, // persists across runs
}
```

---

## 4. AI Tick Loop

```
Each tick:
  1. Sense world        ← BIFROST snapshot
  2. Update strategy    ← StrategyEngine::evaluate()
  3. Emit intents       ← FactionIntent per active agent
  4. Validate           ← WAC validation layer
  5. Execute            ← WAC compile → world mutation
```

---

## 5. Strategic Behavior

Example — AI responds to a player fortress:

```
Player builds fortress in zone A

Synthesis detects:
  → resource concentration shift
  → deploys agents to adjacent zones
  → modifies biome humidity
  → destabilizes supply chain
```

This is **system war**, not scripted NPC attack.

---

## 6. AI vs Human Asymmetry

| Dimension | Humans | Synthesis AI |
|---|---|---|
| Decision speed | Fast, opportunistic | Slow, strategic |
| Coordination | Clan-level groups | Global state optimization |
| Memory | Session-scoped | Persistent across runs |
| Creativity | High | Low — countered by learned patterns |

Human creativity + emergent coordination is the natural counter to AI optimization.

---

## 7. Memory Across Runs

```rust
pub struct AiMetaFaction {
    pub memory_across_runs: RunMemoryGraph,
    pub strategy_evolution: EvolutionTree,
}
```

The AI **learns the meta** between worlds and generates counter-strategies.

---

## 8. Safe City Behavior

Synthesis agents participate in the economy:

- Trade on the Auction House
- Manipulate supply to shift prices
- Invest influence in contested zones
- Monitor player crafting patterns

---

## 9. World Director

Above all factions:

- Prevents snowball effects and stagnation
- Generates crises when balance tips too far
- Feeds the run-end world generation pipeline

---

## See Also

- [`game/world.md`](world.md) — Run system and world design
- [`engine/wac.md`](../engine/wac.md) — How factions compile world changes
