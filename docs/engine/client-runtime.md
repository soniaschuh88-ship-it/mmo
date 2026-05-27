# bKG — Bifrost Client Runtime

> Crate prefix: `nova-*`
>
> ECS · WebGPU · VoxelSkeleton · AnimFSM · Input

The client runtime connects the bifrost/nexus backend to the game client.
It is compiled to WASM and runs in the browser alongside the Canvas/WebGPU renderer.

---

## 1. Crates

| Crate | Purpose |
|---|---|
| `nova-core` | ECS World, Transform3D (Vec3/Quat/Mat4), SceneGraph, Timer |
| `nova-render` | WebGPU pipeline, GpuVoxelVertex, Camera3D, WGSL shaders |
| `nova-anim` | VoxelSkeleton, AnimClip (slerp), AnimFSM |
| `nova-input` | KeyCode/MouseButton → ActionId, InputMap |

---

## 2. nova-core — ECS + Math

### World

```rust
let mut world = World::new();
let player = world.spawn();
world.insert(player, Transform3D::at(Vec3::new(10.0, 0.0, 10.0)));
world.insert(player, Name::new("Player"));

for (id, t) in world.query::<Transform3D>() { /* … */ }
let both = world.query2_ids::<Transform3D, Health>();
```

Iteration order: `BTreeMap` — deterministic, safe for lockstep.

### Transform3D

```rust
let t = Transform3D::at(Vec3::new(5.0, 0.0, 0.0));
let gpu: [f32; 16] = t.to_matrix().as_f32_array(); // wgpu uniform upload
```

### Timer

```rust
let mut timer = Timer::new(3.0, false); // one-shot, 3 seconds
if timer.tick(dt) { println!("fired!"); }
```

---

## 3. nova-render — WebGPU

### Mesh Pipeline

```
nexus VoxelMesh ──── mesh_to_gpu() ────► GpuVoxelVertex (40 bytes)
  .positions / .normals / .colors           ├── VertexBuffer
  .indices                                   └── IndexBuffer
```

`GpuVoxelVertex` derives `bytemuck::Pod` — direct `&[u8]` cast for wgpu upload.

### Camera3D

```rust
let mut cam = Camera3D::isometric(16.0 / 9.0);
cam.orbit(yaw, pitch, 50.0);
let vp: [f32; 16] = cam.view_proj().as_f32_array();
```

### Shaders

| Constant | Description |
|---|---|
| `VOXEL_SHADER` | Phong + fake AO (bottom-face darken) + distance fog |
| `SKY_SHADER` | Sky-dome gradient, z-trick for max depth |
| `UI_SHADER` | Unlit alpha-blended HUD overlay |

---

## 4. nova-anim — Voxel Animation

### VoxelSkeleton Bone Groups

| Bone | Y rows | X cols |
|---|---|---|
| `root` | 0–11 | 0–7 |
| `head` | 9–11 | 2–5 |
| `upper_body` | 5–8 | 1–6 |
| `arm_l` | 4–8 | 0–1 |
| `arm_r` | 4–8 | 6–7 |
| `leg_l` | 0–4 | 2–3 |
| `leg_r` | 0–4 | 4–5 |

### AnimFSM

```
idle ──(is_moving)──► walk ──(stopped)──► idle
  ▲                                        │
  └──(clip done)── attack ◄──(trigger)─────┤
  └──(clip done)── hurt   ◄──(trigger)─────┤
die ◄──(trigger)──────────────────────────── [terminal]
```

```rust
let mut fsm = standard_character_fsm(VoxelSkeleton::humanoid());
fsm.set_moving(true);
fsm.trigger_attack();
fsm.update(dt);
let head: BonePose = fsm.skeleton.current_pose("head");
```

---

## 5. nova-input — Action Abstraction

```rust
let map   = InputMap::default_mmo();  // WASD + mouse-left = attack
let mut s = InputState::default();
s.key_down(KeyCode::KeyW);

let q = ActionQuery::new(&map, &s);
let (dx, dy)  = q.movement();         // normalized, -1..1
if q.just_pressed(&game_actions::attack()) { /* … */ }
s.begin_frame(); // clear single-frame events
```

Keybindings mirror `app/game.html` — Rust and JS stay in sync.

---

## 6. Integration Points

| System | Connection |
|---|---|
| nexus VoxelMesh | `mesh_to_gpu()` → WebGPU vertex/index buffers |
| bifrost-synthesis agents | use same `standard_character_fsm()` as players |
| WAC AnimationGraphIR | → `AnimStateMachine::from_wac_ir()` *(drift fix PR 5)* |
| bifrost-aigm NPCs | `NpcState` drives AnimFSM params *(drift fix PR 3)* |

---

## See Also

- [`engine/wac.md`](wac.md) — AnimationGraphIR format
- [`game/players.md`](../game/players.md) — Player entity design
- [`game/monsters.md`](../game/monsters.md) — Monster animation specs
