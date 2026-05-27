# bKG — Bifrost Architecture

> *bKG · best known Garbage · Distributed Reality Compute Fabric*
>
> Single source of truth. DELPHOS decides. Players compute.

---

## Two-System Model

| System | Role |
|---|---|
| **DELPHOS** (Rust) | Deterministic ontology, event kernel, authority |
| **MMO Fabric** (Node + WASM) | Distributed real-time simulation + render mesh |

These must NOT be fully merged. **Bifrost** is the bridge between them.

---

## Authority Table

The server is **only** authoritative over:

| Responsibility | Central | Rationale |
|---|---|---|
| Identity / Auth | YES | Prevents Sybil attacks |
| Ledger / Replay | YES | Tamper-evident history |
| Epoch Authority | YES | Consensus anchor |
| Cheat Detection | YES | Cross-peer verification |
| Global Time | YES | Tick authority |
| Settlement | YES | Economy finalization |
| Physics | NO | Swarm-computed |
| Rendering | NO | Client-computed |
| NPC cognition | NO | Distributed agents |
| Combat | NO | Witness-verified |

---

## The Inverse MMO Effect

Normal MMOs:
```
More players = more server load = more lag
```

bKG — Bifrost:
```
More players = more compute = more simulation capacity
```

The player population **is** the supercomputer.

---

## Stack Diagram

```
┌─────────────────────────────────────────────────────────┐
│                  DELPHOS / BIFROST SERVER               │
│  bifrost-lockstep · bifrost-witness · bifrost-physics   │
│  bifrost-wac · bifrost-run · bifrost-synthesis          │
│  bifrost-aigm · bifrost-safe-city                       │
├─────────────────────────────────────────────────────────┤
│                   NEXUS VOXEL KERNEL                    │
│   greedy meshing · NavMesh · biome generator · WAC RT   │
├─────────────────────────────────────────────────────────┤
│               BIFROST CLIENT RUNTIME (nova-*)           │
│   ECS · WebGPU · AnimFSM · Camera3D · Input             │
├─────────────────────────────────────────────────────────┤
│                     GAME CLIENT                         │
│         app/game.html · isometric · Canvas/WebGPU       │
└─────────────────────────────────────────────────────────┘
```

---

## Bifrost Layer

Bifrost translates DELPHOS epoch authorities into spatial chunk authorities for the swarm:

```
DELPHOS                        MMO Fabric
─────────                      ──────────
KernelEpoch  ──► ChunkAuthority ──► PeerWitness ──► VoxelInstruction
EventLedger  ──► ChunkReplay   ──► LocalPhysics ──► StateHash
```

---

## Deterministic Witness Execution

```
1 Authority Peer
+ 2 Witness Peers
+ N Advisory Peers
```

All compute the same tick. Then `hash(state_after_tick)`:

- 3/3 agree → **Accepted**
- Any mismatch → **Contested** → conflict replay + trust reduction

---

## Hierarchical Reality

Simulation fidelity scales with distance:

| Distance | Simulation Level |
|---|---|
| Near (0–200 units) | Full voxel + debris + particles + combat |
| Mid (200–2000) | Aggregated physics, simplified NPCs |
| Far (2000–20000) | Statistical simulation only |
| Very far (20000+) | Narrative state only |

---

## Proof-of-Render Reputation

Peers earn `trustScore` for:
- Stable tile frames
- Low latency
- Deterministic physics matches
- Witness vote agreement

Higher `trustScore` → more authority, more render responsibility, better rewards.

---

## See Also

- [`engine/bifrost-protocol.md`](bifrost-protocol.md) — Protocol specification
- [`engine/wac.md`](wac.md) — World Asset Compiler
- [`game/world.md`](../game/world.md) — World design
