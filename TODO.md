# TODO

## Drift Fix PRs (in progress)

- [ ] PR 1 — merge: noise smoothstep + canonical biome registry (open: #4)
- [ ] PR 2 — unify `BiomeIR` duplicate (nexus/bridge vs bifrost/wac)
- [ ] PR 2 — `Vec3Payload` in `bifrost-aigm/event.rs` → `nova_core::Vec3`
- [ ] PR 3 — quest HTTP routes (`/aigm/quests`) + game.html API fetch
- [ ] PR 3 — NPC HTTP routes (`/aigm/npcs`) + game.html API fetch
- [ ] PR 4 — wire run-end → WorldDirector (self-evolving world loop)
- [ ] PR 5 — `AnimationGraphIR::to_nova_fsm()` bridge

## Game Client

- [ ] Replace Canvas 2D renderer with nova-render WebGPU pass
- [ ] Connect quest/NPC data to bifrost-aigm HTTP API (removes hardcoded arrays)

## Infrastructure

- [ ] Enable Docker rootless mode on host
- [ ] Verify `docker compose up` works under rootless Docker
- [ ] WASM compilation target for nova-core + nova-render
