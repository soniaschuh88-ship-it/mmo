# bKG — Synthesis AI Civilization

> Not NPCs. A **distributed strategic civilization** competing against human players using the same rules.

---

## 1. The Symmetry Guarantee

Synthesis agents emit world-manipulation intents in the **same format as human players**:

```
SynthesisTick → FactionIntent → WAC Blueprint → validate() → compile() → World
```

No special world access. No cheating. The playing field is identical.

---

## 2. Civilization Structure

```
1 SynthesisCore   ── global strategist (backed by NVIDIA NIM)
N SubAI nodes     ── region controllers
M AgentNodes      ── squad / clan level actors
```

```rust
pub struct AiFaction {
    pub id:             String,
    pub economy:        EconomyGraph,
    pub territory:      Vec<ZoneId>,
    pub agents:         Vec<AgentNode>,
    pub strategy_model: WorldModel,
    pub memory:         FactionMemoryGraph,
}
```

---

## 3. AI vs Human Asymmetry

| Dimension | Human Players | Synthesis AI |
|---|---|---|
| Decision style | Individual, creative, emergent | Coordinated, long-term, optimized |
| Scale | Clan-level groups | Global state optimization |
| Reaction speed | Fast, opportunistic | Slow, strategic |
| Memory | Session-scoped | Persistent across runs |

The AI does not play better — it plays **differently**. Human creativity and emergent coordination are the counter.

---

## 4. AI World Control Loop

Each tick:

```
1. Sense world        ← BIFROST snapshot
2. Update strategy    ← StrategyEngine::evaluate()
3. Emit intents       ← FactionIntent per active agent
4. [Caller] Validate  ← WAC validation layer
5. [Caller] Execute   ← WAC compile → world mutation
```

---

## 5. Zone Control Strategy

```
Player builds fortress in zone A

AI detects:
  → resource concentration shift
  → sends 3 agents to adjacent zones
  → modifies biome humidity
  → destabilizes supply chain
```

This is **system war**, not NPC attack. The AI manipulates the physics of the economy and terrain.

---

## 6. AI Behavior in Safe City

Synthesis agents participate in the Safe City economy:

- Buy and sell on the Auction House
- Manipulate supply to shift prices
- Invest influence in zones
- Monitor player crafting patterns

---

## 7. Memory Across Runs

The AI carries knowledge between runs:

```rust
pub struct AiMetaFaction {
    pub memory_across_runs: RunMemoryGraph,
    pub strategy_evolution: EvolutionTree,
}
```

The AI **learns the meta** between worlds and generates counter-strategies.

---

## 8. Emergent Behaviors

When both sides operate with full symmetry:

**AI begins:**
- Industrializing regions
- Building defensive biome evolution
- Optimizing supply chains

**Players begin:**
- Exploiting AI patterns
- Creating fake economy loops
- Territorial guerilla strategies

This produces **simulated civilization competition in real time** — not scripted content.

---

## 9. World Director (Meta-AI)

Above both factions sits the World Director:

- Prevents stagnation and snowball effects
- Generates crises when balance tips too far
- Balances faction evolution over time
- Feeds the run-end world generation pipeline

---

## See Also

- [`docs/WORLD.md`](WORLD.md) — Run system and world design
- [`docs/WAC.md`](WAC.md) — How both factions compile world changes
