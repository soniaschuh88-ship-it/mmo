# TODO

## Drift Fix PRs

- [ ] **PR 1** — merge #4: noise smoothstep JS=Rust + canonical biome registry
- [ ] **PR 2** — unify `BiomeIR` (nexus/bridge vs bifrost/wac)
- [ ] **PR 2** — `Vec3Payload` in `bifrost-aigm/event.rs` → `nova_core::Vec3`
- [ ] **PR 3** — quest HTTP routes (`/aigm/quests`) + game.html API fetch
- [ ] **PR 3** — NPC HTTP routes (`/aigm/npcs`) + game.html API fetch
- [ ] **PR 4** — wire run-end → WorldDirector (self-evolving world loop)
- [ ] **PR 5** — `AnimationGraphIR::to_nova_fsm()` bridge

## Game Client

- [ ] Replace Canvas 2D renderer with nova-render WebGPU pass
- [ ] Connect quest/NPC data from bifrost-aigm API (remove hardcoded arrays)

## Content

- [ ] Monster: Crimson Bat, Crimson Wraith (crimson_forest biome)
- [ ] Monster: Ice Golem, Frost Wyrm (snow biome)
- [ ] Monster: Lava Golem, Ash Fiend (volcanic biome)
- [ ] NPC: merchant system (vendor inventory, restock)
- [ ] Quest: exploration quest type
- [ ] Quest: economy quest type (auction house delivery)

## Infrastructure

- [ ] Enable Docker rootless mode on host
- [ ] WASM compilation target for nova-core + nova-render
- [ ] Cloud Run staging deployment (see index.ts Pulumi stack)
