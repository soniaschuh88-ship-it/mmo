# bKG Phase Roadmap

> DRCF — Distributed Reality Compute Fabric

---

## Phase 1 — Bifrost Layer (Current)

Build the deterministic bridge between DELPHOS Kernel and the MMO Fabric.

### Batch 1 — Core Bifrost Crates

| Crate | Status | Description |
|---|---|---|
| `bifrost-vis` | ✅ | Voxel Instruction Set opcodes |
| `bifrost-chunk` | ✅ | Chunk authority epoch system |
| `bifrost-lockstep` | ✅ | Lockstep tick scheduler |
| `bifrost-witness` | ✅ | Witness quorum execution |
| `bifrost-physics` | ✅ | Deterministic WASM physics kernel |

### Batch 2 — Hierarchical Simulation

- [ ] `bifrost-lod` — Level-of-detail simulation abstraction
- [ ] `bifrost-combat` — Probabilistic distant combat
- [ ] `bifrost-prediction` — Delta prediction fabric

### Batch 3 — Reputation + GPU Distribution

- [ ] `bifrost-reputation` — Proof-of-Render trust score engine
- [ ] `bifrost-gpu-market` — GPU task distribution market
- [ ] `bifrost-tile-verify` — Tile witness verification + malicious detection

### Batch 4 — Sovereign Worlds

- [ ] `bifrost-npc` — Fully distributed NPC cognition
- [ ] `bifrost-economy` — Emergent player-driven economy
- [ ] `bifrost-shard` — Player-hosted sovereign world shards

---

## Phase 2 — MMO Fabric Integration

- Node.js WebRTC mesh bridge
- Browser WASM runtime deployment
- GPU P2P distribution network

## Phase 3 — DELPHOS ↔ Bifrost Protocol

- Formal epoch handshake protocol
- Ledger-backed chunk replay
- Settlement finalization bridge

---

## Core Invariants (Never Violate)

1. Physics is never centralized
2. Rendering is never centralized
3. NPC cognition is never centralized
4. Combat resolution is never centralized
5. DELPHOS decides truth; players compute reality
