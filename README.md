# mmo

**Distributed Reality Compute Fabric (DRCF)**

> The players ARE the engine.

---

## Architecture

```
                ┌──────────────────────┐
                │   DELPHOS KERNEL     │
                │   Rust Determinism   │
                └──────────┬───────────┘
                           │
                 Domain Events / Truth
                           │
            ┌──────────────┴──────────────┐
            │                             │
    ┌───────▼────────┐          ┌────────▼────────┐
    │  MMO FABRIC    │          │  WORLD MEMORY   │
    │  Node + WASM   │          │  VLDB Storage   │
    └───────┬────────┘          └────────┬────────┘
            │                             │
    WebRTC Mesh / GPU P2P        Chunk Event Replay
            │                             │
            └──────────────┬──────────────┘
                           │
                ┌──────────▼──────────┐
                │ CLIENT SWARM NODES  │
                │ players ARE engine  │
                └─────────────────────┘
```

**DELPHOS decides truth. The players compute reality.**

---

## Bifrost Layer — Phase 1 Crates

| Crate | Purpose |
|---|---|
| `bifrost-vis` | Voxel Instruction Set — deterministic opcode system |
| `bifrost-chunk` | Chunk Authority Epochs — spatial peer authority rotation |
| `bifrost-lockstep` | Lockstep Tick Scheduler — all peers advance in sync |
| `bifrost-witness` | Witness Quorum Executor — 1 authority + 2 witnesses |
| `bifrost-physics` | Deterministic WASM Physics Kernel — browser/server identical |

---

## Core Invariant

The server is only authoritative over:

| Responsibility | Central |
|---|---|
| Identity / Auth | YES |
| Ledger / Replay | YES |
| Epoch Authority | YES |
| Cheat Detection | YES |
| Global Time | YES |
| Settlement | YES |

Everything else → **Swarm**.

---

## Docs

- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — full system architecture
- [`docs/PLAN.md`](docs/PLAN.md) — phase roadmap
- [`docs/BIFROST-SPEC.md`](docs/BIFROST-SPEC.md) — Bifrost protocol specification
