# Bifrost Client Runtime

> ECS · WebGPU · VoxelSkeleton · AnimFSM · Input
>
> Crate prefix: `nova-*` — part of the **bKG — Bifrost** stack.

The client runtime sits between the bifrost/nexus backend and the game client.
It provides a deterministic, GPU-ready runtime for rendering, animation, and input —
built on top of the existing bifrost/nexus stack.

---

## 1. Crate Overview

| Crate | Purpose |
|---|---|
| `nova-core` | ECS World, Transform3D, SceneGraph, Time |
| `nova-render` | WebGPU pipeline, Camera3D, WGSL shaders |
| `nova-anim` | VoxelSkeleton, AnimClip, AnimFSM |
| `nova-input` | KeyCode/MouseButton → ActionId abstraction |

---

## 2. nova-core

### ECS World

Sparse-set ECS. `BTreeMap` iteration is deterministic — safe for lockstep networking.

```rust
let mut world = World::new();
let player = world.spawn();
world.insert(player, Transform3D::at(Vec3::new(10.0, 0.0, 10.0)));
world.insert(player, Name::new("Player"));

for (id, t) in world.query::<Transform3D>() {
    println!("{id}  pos={:?}", t.position);
}
```

### Transform3D

`Vec3` / `Quat` / `Mat4` — all `f32` for GPU compatibility.

```rust
let t = Transform3D::at(Vec3::new(5.0, 0.0, 0.0));
let matrix: Mat4 = t.to_matrix();          // TRS column-major
let gpu_data: [f32; 16] = matrix.as_f32_array();  // wgpu uniform upload
```

### SceneGraph

Parent/child hierarchy with recursive world-matrix computation.

```rust
let mut scene = SceneGraph::new();
scene.add_root(root_id);
scene.attach(root_id, child_id);

let world_matrix = scene.world_matrix(child_id, |id| world.get::<Transform3D>(id));
```

### Time

Delta time with fixed-update budget and countdown timers.

```rust
let mut time = Time::default();
time.advance(delta_seconds);
while time.consume_fixed() { /* 60 Hz fixed update */ }

let mut timer = Timer::new(3.0, false);
if timer.tick(dt) { println!("fired!"); }
```

---

## 3. nova-render

### WebGPU Pipeline

```
nexus-voxel-kernel                 nova-render
──────────────────                 ────────────────────────
VoxelChunk
  │  build_mesh()                  WebGPU VoxelPass
  ▼                                ├── VertexBuffer (GpuVoxelVertex ×N)
VoxelMesh ──── mesh_to_gpu() ────► ├── IndexBuffer  (u32 ×M)
  .positions: Vec<[f32;3]>         ├── voxel.wgsl
  .normals:   Vec<[f32;3]>         │   ├── Phong diffuse
  .colors:    Vec<[u8;4]>          │   ├── Fake AO (bottom-face darken)
  .indices:   Vec<u32>             │   └── Distance fog
                                   └── ChunkMeshRegistry
```

### GpuVoxelVertex

40-byte vertex: `position[f32;3]` · `normal[f32;3]` · `color[f32;4]`.
Derives `bytemuck::Pod` — safe to cast to `&[u8]` for GPU upload.

### Camera3D

```rust
let mut cam = Camera3D::perspective(16.0 / 9.0);
cam.orbit(yaw, pitch, distance);

let vp: Mat4 = cam.view_proj();  // upload to Camera uniform buffer
```

### WGSL Shaders

| Constant | Description |
|---|---|
| `VOXEL_SHADER` | Phong + fake ambient occlusion + distance fog |
| `SKY_SHADER` | Sky-dome gradient, z-trick for max depth |
| `UI_SHADER` | Unlit alpha-blended HUD overlay |

---

## 4. nova-anim

### VoxelSkeleton

Named bone groups for the 8×12 voxel humanoid character model.
Shared between `nova-render` and `app/game.html`.

| Bone | Y rows | X cols | Notes |
|---|---|---|---|
| `root` | 0–11 | 0–7 | whole body |
| `head` | 9–11 | 2–5 | rotates for look direction |
| `upper_body` | 5–8 | 1–6 | tilts for walk lean |
| `arm_l` | 4–8 | 0–1 | swings on walk, attack |
| `arm_r` | 4–8 | 6–7 | primary attack arm |
| `leg_l` | 0–4 | 2–3 | alternating step |
| `leg_r` | 0–4 | 4–5 | alternating step |

### AnimFSM

```
idle ──(is_moving=true)──► walk ──(is_moving=false)──► idle
 ▲                          ▲
 │    ◄──(clip done)────────┤
 │                       attack ◄──(trigger: attack)── any
 │
 │ ◄──(clip done)── hurt ◄──(trigger: hurt)── any
 │
die ◄──(trigger: die)── any   [terminal — no exit]
```

```rust
let mut fsm = standard_character_fsm(VoxelSkeleton::humanoid());

// Each frame:
fsm.set_moving(player_is_moving);
if attacked { fsm.trigger_attack(); }
fsm.update(delta_time);

// Read bone poses for renderer:
let head_pose: BonePose = fsm.skeleton.current_pose("head");
```

---

## 5. nova-input

```rust
let map   = InputMap::default_mmo();   // WASD + mouse-left = attack
let mut state = InputState::default();

// On browser event:
state.key_down(KeyCode::KeyW);
state.mouse_down(MouseButton::Left);

// Each frame query:
let q = ActionQuery::new(&map, &state);
let (dx, dy) = q.movement();           // normalized (-1..1, -1..1)
if q.just_pressed(&game_actions::attack()) { /* … */ }

// End of frame:
state.begin_frame();
```

Keybindings match `app/game.html` — Rust and JS input layers stay in sync.

---

## 6. Integration with Bifrost

- `standard_character_fsm()` drives both player characters and `bifrost-synthesis` AI agents
- `InputMap::default_mmo()` keybindings are the source of truth for `app/game.html`
- `Camera3D::isometric()` matches the 2.5D view in `app/game.html`
- WAC `AnimationGraphIR` will be connected to `AnimStateMachine` (Drift Fix PR 2)
