# TODO

## Drift Fix PRs

- [ ] PR 2 — unify `BiomeIR` (nexus/bridge vs bifrost/wac)
- [ ] PR 2 — `Vec3Payload` in `bifrost-aigm/event.rs` → `nova_core::Vec3`
- [ ] PR 3 — quest HTTP routes (`/aigm/quests`) + game.html API fetch
- [ ] PR 3 — NPC HTTP routes (`/aigm/npcs`) + game.html API fetch
- [ ] PR 4 — wire run-end → WorldDirector (self-evolving world loop)
- [ ] PR 5 — `AnimationGraphIR::to_nova_fsm()` bridge

## Game Client

- [ ] Replace Canvas 2D renderer with nova-render WebGPU pass
- [ ] Connect quests/NPCs to bifrost-aigm HTTP API

## Content

- [ ] Monster: Crystal Bat, Crimson Wraith (crimson_forest)
- [ ] Monster: Ice Golem, Frost Wyrm (snow)
- [ ] Monster: Lava Golem, Ash Fiend (volcanic)
- [ ] NPC: merchant system (vendor inventory, restock)
- [ ] Quest: exploration + economy quest types

## Infrastructure

- [ ] Enable Docker rootless on host
- [ ] WASM compilation target for nova-core + nova-render
- [ ] GCP Cloud Run staging deployment (see index.ts)
