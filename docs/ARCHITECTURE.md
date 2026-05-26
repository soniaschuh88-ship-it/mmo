# bKG Architecture — Distributed Reality Compute Fabric

> Single source of truth. DELPHOS decides. Players compute.

---

## Two-System Model

The architecture deliberately maintains **two separate systems**:

| System | Role |
|---|---|
| **DELPHOS** (Rust) | Deterministic ontology, event kernel, authority |
| **MMO Fabric** (Node + WASM) | Massive distributed real-time simulation + render mesh |

These must NOT be fully merged. Bifrost is the bridge between them.

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

DRCF:
```
More players = more compute = more simulation capacity
```

The player population is the supercomputer.

---

## Bifrost Layer

Bifrost sits between DELPHOS and the MMO Fabric. It translates DELPHOS epoch authorities into spatial chunk authorities that the swarm can execute.

```
DELPHOS                        MMO Fabric
─────────                      ──────────
KernelEpoch  ──► ChunkAuthority ──► PeerWitness ──► VoxelInstruction
EventLedger  ──► ChunkReplay   ──► LocalPhysics ──► StateHash
```

---

## Deterministic Witness Execution

Not one peer simulates. Instead:

```
1 Authority Peer
+ 2 Witness Peers  
+ N Advisory Peers
```

All compute the same tick. Then:

```
hash(state_after_tick)
```

- 3/3 agree → **Accepted**
- Mismatch → **Contested** → conflict replay + trust reduction

---

## Hierarchical Reality

Simulation fidelity scales with distance:

| Distance | Simulation Level |
|---|---|
| Near (0–200 units) | Full voxel + debris + particles + combat |
| Mid (200–2000) | Aggregated physics, simplified NPCs |
| Far (2000–20000) | Statistical simulation only |
| Very far (20000+) | Narrative state only |

This is the only path to 100k-player battles.

---

## Voxel Instruction Set (VIS)

Instead of individual voxel events, the network transmits **opcodes**:

```
SIM_EXPLOSION(radius=12, material=stone)
```

Each peer executes the opcode locally with identical deterministic physics.
Network cost: O(1) per explosion, not O(radius³).

---

## Proof-of-Render Reputation

Peers earn `trustScore` for:
- Stable tile frames
- Low latency
- Deterministic physics matches
- Witness vote agreement

Higher `trustScore` → more authority, more render responsibility, better rewards.

This is a **Reputation Consensus Engine**, not a cryptocurrency.
