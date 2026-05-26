# NOVA — Next-gen Open Voxel Architecture

Die Kernidee: Die Welt ist kein State — sie ist eine History.

## Das fundamentale Problem mit VSL + Delta Log

Dein aktueller Ansatz:

State (current) + Delta Log (mutations) + CRC32 Heal (conflict)

Das Problem: State und Log existieren parallel. Sie können divergieren. Heal ist reaktiv.

## Idee: Pure Event Sourcing als einzige Quelle der Wahrheit

```
World = fold(events, ∅)
```

Es gibt keinen gespeicherten State — nur ein immutables, append-only Event Ledger. Der aktuelle Zustand ist immer `events.reduce(apply)`.

```ts
Event = {
  seq:       u64,        // global monotonic
  type:      string,     // "voxel.set" | "entity.move" | "ai.quest" | ...
  payload:   any,
  author:    AgentId,    // Player | NPC | AIGm | System
  worldHash: u64,        // CRC of world at seq-1
  ts:        u64
}
```

Was das bringt:

- Kein Konflikt möglich — Events werden sequenziert, nicht gemergt
- Time Travel: Welt zu jedem Zeitpunkt rekonstruierbar
- AI GM schreibt Events genau wie ein Spieler — kein Sonderfall
- Perfekte Audit-Trail, Replay, Rollback

---

## NOVA Engine — Complete System Blueprint

### Overview

NOVA (Next-gen Open Voxel Architecture) is an event-sourced, ECS-driven MMO engine with real-time AI Game Master. Every game state is derived from an immutable event ledger. No state divergence. No conflict resolution. Full time travel.

```json
{
  "engine":   "nova",
  "version":  "0.1.0",
  "paradigm": "event-sourced ECS",
  "target":   "browser (WASM) + server (Node/Rust)",
  "layers": [
    "event-ledger",
    "ecs-simulation",
    "voxel-world",
    "ai-gm",
    "network",
    "client-renderer"
  ]
}
```

---

## Layer 1: Event Ledger

Die einzige Quelle der Wahrheit. Kein gespeicherter State — nur Events.

### Event Schema

```ts
interface WorldEvent {
  seq:       bigint;       // global monotonic counter, u64
  type:      EventType;    // see catalogue below
  payload:   unknown;      // type-specific data
  author:    AuthorId;     // player | npc:<id> | aigm | system
  worldHash: number;       // crc32 of world at seq-1 (chain integrity)
  zoneId:    string;       // zone this event belongs to
  ts:        number;       // unix ms
}

type AuthorId =
  | `player:${string}`
  | `npc:${string}`
  | `aigm`
  | `system`;
```

### Event Type Catalogue

```ts
type EventType =
  // World mutation
  | "voxel.set"            // place/remove single voxel
  | "voxel.batch"          // bulk terrain mutation
  | "voxel.explosion"      // radial destruction

  // Entity lifecycle
  | "entity.spawn"
  | "entity.despawn"
  | "entity.move"
  | "entity.teleport"

  // Combat
  | "combat.attack"
  | "combat.damage"
  | "combat.death"
  | "combat.resurrect"
  | "combat.status.apply"
  | "combat.status.remove"

  // AI Game Master
  | "aigm.quest.create"
  | "aigm.quest.update"
  | "aigm.event.world"     // weather, disasters, invasions
  | "aigm.npc.speak"
  | "aigm.npc.goal.set"
  | "aigm.story.beat"

  // Player
  | "player.join"
  | "player.leave"
  | "player.input"
  | "player.inventory.change"
  | "player.levelup"
  | "player.reputation.change"

  // Economy
  | "economy.trade"
  | "economy.loot.drop"
  | "economy.loot.pickup"

  // Zone
  | "zone.load"
  | "zone.unload"
  | "zone.authority.transfer";
```

### Event Payloads

```ts
// voxel.set
interface VoxelSetPayload {
  x: number; y: number; z: number;
  material: number;   // palette index
  prev:     number;   // previous material (for rollback)
}

// entity.move
interface EntityMovePayload {
  entityId: string;
  from: Vec3; to: Vec3;
  velocity: Vec3;
  tick: number;
}

// aigm.quest.create
interface AiGmQuestPayload {
  questId:    string;
  title:      string;
  giverNpcId: string;
  targetIds:  string[];
  objectives: QuestObjective[];
  reward:     QuestReward;
  expiresAt:  number | null;    // dynamic quests can expire
  context:    string;           // AI reasoning (debug)
}

// aigm.story.beat
interface StoryBeatPayload {
  beatId:      string;
  title:       string;
  description: string;
  affectedZones: string[];
  consequence: WorldConsequence[];  // world state changes this triggers
}
```

### Ledger Storage

```ts
interface LedgerSegment {
  worldId:    string;
  segmentId:  string;         // "<worldId>.<seqStart>-<seqEnd>"
  seqStart:   bigint;
  seqEnd:     bigint;
  events:     WorldEvent[];
  compressed: boolean;        // gzip after segment is sealed
  createdAt:  number;
}

// Segment lifecycle:
// OPEN: receiving new events (latest segment only)
// SEALED: seqEnd set, immutable, candidate for compression
// ARCHIVED: gzip-compressed, moved to cold storage (S3)

interface LedgerIndex {
  worldId:   string;
  segments:  LedgerSegmentMeta[];
  headSeq:   bigint;
  headHash:  number;          // crc32 of last event
}
```

---

## Layer 2: ECS Simulation

### Core Types

```ts
type EntityId = bigint;       // u64

interface Component {
  readonly __type: string;
}

interface System {
  readonly name: string;
  readonly reads: string[];         // component types read
  readonly writes: string[];        // component types written
  tick(world: EcsWorld, events: WorldEvent[]): WorldEvent[];
}

interface EcsWorld {
  tick:      number;
  entities:  Map<EntityId, ComponentMap>;
  query<T extends Component[]>(...types: ComponentTypes<T>): EntityId[];
  get<T extends Component>(entityId: EntityId, type: ComponentType<T>): T | null;
  set(entityId: EntityId, component: Component): void;
  remove(entityId: EntityId, type: string): void;
  spawn(): EntityId;
  despawn(entityId: EntityId): void;
}
```

### Component Catalogue

```ts
interface PositionComponent extends Component {
  __type: "Position";
  x: number; y: number; z: number;
  zoneId: string;
}

interface VelocityComponent extends Component {
  __type: "Velocity";
  vx: number; vy: number; vz: number;
  onGround: boolean;
  gravity: number;
}

interface HealthComponent extends Component {
  __type: "Health";
  current: number;
  max: number;
  regenRate: number;       // per tick
  isImmortal: boolean;
}

interface CombatComponent extends Component {
  __type: "Combat";
  damage: number;
  defense: number;
  attackSpeed: number;     // attacks per second
  range: number;
  statusEffects: StatusEffect[];
  aiPattern: AiPattern;
  aggroRange: number;
  leashRange: number;
  immuneTo: DamageType[];
  weakTo: DamageType[];
}

interface InventoryComponent extends Component {
  __type: "Inventory";
  slots: InventorySlot[];
  maxWeight: number;
  gold: number;
}

interface NpcComponent extends Component {
  __type: "Npc";
  npcId: string;
  faction: string;
  behavior: "friendly" | "neutral" | "hostile";
  schedule: ScheduleEntry[];
  quests: string[];
  isShopkeeper: boolean;
}

interface AiContextComponent extends Component {
  __type: "AiContext";
  model: string;              // "llama3-8b" | "mistral-7b" | ...
  systemPrompt: string;       // base personality
  shortTermMemory: Memory[];  // last N interactions
  vectorIds: string[];        // long-term memory references
  currentGoal: string;
  mood: Mood;
  knownFacts: string[];
  lastSpokenAt: number;
  cooldownMs: number;
}

interface PlayerComponent extends Component {
  __type: "Player";
  playerId: string;
  class: string;
  level: number;
  xp: number;
  activeQuestIds: string[];
  completedQuestIds: string[];
  reputation: Record<string, number>;
  stats: PlayerStats;
}

interface ObserverComponent extends Component {
  __type: "Observer";
  viewDistance: number;       // in chunks
  subscribedChunks: ChunkKey[];
}
```

### System Execution Order

```ts
const SYSTEM_ORDER: string[] = [
  "InputSystem",            // translate player inputs to events
  "PhysicsSystem",          // velocity, gravity, collision
  "CombatSystem",           // attack resolution, damage
  "StatusEffectSystem",     // apply/tick/expire status effects
  "NpcBehaviorSystem",      // AI pattern execution (patrol, chase, etc.)
  "AiContextSystem",        // LLM NPC dialogue trigger
  "QuestSystem",            // objective progress evaluation
  "EconomySystem",          // trade, loot, gold
  "ZoneTransitionSystem",   // enter/exit zone portals
  "AiGmSystem",             // AI Game Master — last to run
  "EventEmitSystem",        // commit generated events to ledger
];
```

---

## Layer 3: Voxel World

### Octree Structure

```
World (1024³)
└── Regions (128³)       LOD 3 — nur Metadata
    └── Chunks (32³)     LOD 2 — grobe Geometrie
        └── Micro (8³)   LOD 1 — Detail
            └── Voxel    LOD 0 — full resolution
```

Sparse: Leere Regions existieren nicht. Eine Ozean-Welt braucht fast keinen RAM.

```ts
interface ChunkData {
  key:      ChunkKey;         // "<worldId>:cx,cy,cz"
  level:    2;
  palette:  number[];         // material IDs used in this chunk (max 256)
  data:     Uint8Array;       // 32³ = 32768 bytes, each byte = palette index
  version:  bigint;           // ledger seq when last mutated
  crc:      number;
}

// index = x + y*32 + z*1024
// material = palette[data[index]]
```

### Material Registry

```ts
interface Material {
  id:          number;         // u16, 0 = air
  name:        string;
  category:    MaterialCategory;
  solid:       boolean;
  transparent: boolean;
  light:       number;         // 0-15 emission
  hardness:    number;         // ticks to mine
  drops:       DropEntry[];
  textureAtlasIndex: number;
  sounds: {
    place: string;
    break: string;
    walk:  string;
  };
}

type MaterialCategory =
  | "terrain"    // stone, dirt, sand, grass
  | "ore"        // iron, gold, diamond
  | "wood"       // oak, pine, magic
  | "fluid"      // water, lava, acid
  | "structure"  // brick, plank, glass
  | "special"    // portal, spawner, chest
  | "air";
```

### Chunk Cache

```ts
interface ChunkCache {
  // L1: hot — in-process Map, max 512 chunks
  hot: Map<ChunkKey, ChunkData>;

  // L2: warm — LRU, max 4096 chunks, shared via Redis
  warm: LruCache<ChunkKey, ChunkData>;

  // L3: cold — Object Storage (S3/GCS), binary gzip
  cold: ObjectStore;

  get(key: ChunkKey): Promise<ChunkData>;
  set(key: ChunkKey, chunk: ChunkData): Promise<void>;
  evict(key: ChunkKey): void;
  invalidate(keys: ChunkKey[]): void;
}
```

---

## Layer 4: AI Game Master

### Architecture

```
Every tick:
  AiGmSystem reads:
    - active players + their recent actions
    - current world events
    - story beat history
    - NPC states + moods
    - economy state
    - quest completion rates

  AiGmSystem decides:
    - Should a new quest emerge?
    - Should an NPC react?
    - Should a world event happen?
    - Is the story progressing?

  AiGmSystem emits:
    - aigm.quest.create
    - aigm.npc.goal.set
    - aigm.event.world
    - aigm.story.beat
```

### AI GM State

```ts
interface AiGmState {
  worldId: string;

  storyArcs: StoryArc[];
  activeBeatId: string | null;
  completedBeats: string[];

  playerProfiles: Map<string, PlayerProfile>;

  recentEvents: WorldEvent[];   // last 200 events
  worldMood: WorldMood;         // tense | calm | war | festive | ...

  model:       string;
  temperature: number;          // 0.85 default
  maxTokens:   number;          // 2048

  lastQuestAt: number;
  lastBeatAt:  number;
  questCooldownMs: number;      // 60_000 default
}

interface PlayerProfile {
  playerId:    string;
  playtime:    number;
  killCount:   number;
  quests:      { completed: number; failed: number; active: number };
  playstyle:   "explorer" | "fighter" | "trader" | "socializer";
  recentZones: string[];
  relationships: Record<string, number>;   // npcId → trust score
}
```

### NPC LLM Call

```ts
interface NpcLlmRequest {
  npcId:        string;
  trigger:      "player_approach" | "player_speak" | "world_event" | "scheduled";
  playerContext: {
    name:    string;
    level:   number;
    class:   string;
    quests:  string[];
    rep:     number;   // reputation with npc's faction
  };
  worldContext: string;   // recent world events, natural language
  aiContext:    AiContextComponent;
}

interface NpcLlmResponse {
  dialogue:   string;
  mood:       Mood;
  action:     NpcAction | null;   // offer quest, open shop, attack, flee, ...
  memory:     string | null;      // what the NPC remembers from this interaction
  goalUpdate: string | null;      // new goal if changed
}

// NPC call flows through:
// 1. Build prompt from AiContextComponent + request
// 2. Call LLM (Ollama local | NVIDIA NIM | OpenRouter)
// 3. Parse JSON response
// 4. Emit aigm.npc.speak event with dialogue
// 5. Update AiContextComponent.shortTermMemory
// 6. Upsert to vector DB for long-term memory
```

---

## Layer 5: Network Protocol

### Connection Model

```
Client                    Zone Server              Event Ledger
  │                           │                        │
  │── player.join ──────────► │                        │
  │                           │── zone.load ─────────► │
  │◄─ snapshot(seq N) ────── │◄─ events[0..N] ──────── │
  │                           │                        │
  │── InputEvent ───────────► │                        │
  │                           │── (simulate tick) ─── │
  │                           │── emit events ───────► │
  │◄─ EventDelta(seq N+1) ── │                        │
  │                           │                        │
  │ (simulate same tick       │                        │
  │  locally for prediction)  │                        │
```

### Message Types

```ts
// Client → Server
type ClientMessage =
  | { type: "input";    tick: number; input: InputSnapshot }
  | { type: "chat";     text: string }
  | { type: "interact"; targetEntityId: string; action: InteractAction }
  | { type: "chunk.request"; keys: ChunkKey[] };

// Server → Client
type ServerMessage =
  | { type: "snapshot";    seq: bigint; state: ZoneSnapshot }
  | { type: "event.delta"; events: WorldEvent[] }
  | { type: "chunk.data";  key: ChunkKey; data: ArrayBuffer }
  | { type: "npc.speak";   npcId: string; dialogue: string; emotion: string }
  | { type: "tick.ack";    clientTick: number; serverSeq: bigint };

interface InputSnapshot {
  move:    Vec3;          // normalized direction
  look:    Quat;
  actions: ActionFlag;    // bitmask: jump|attack|use|crouch|...
}
```

### Rollback

```ts
interface RollbackBuffer {
  frames: Frame[];        // last 128 frames (2 seconds @ 64Hz)
  confirmed: number;      // last confirmed server tick

  predict(input: InputSnapshot): void;
  confirm(seq: bigint, events: WorldEvent[]): void;
  rollback(toTick: number): void;
}
```

---

## Layer 6: Client Renderer (WASM)

### Rust Crates

```toml
[dependencies]
bevy          = { version = "0.14", features = ["webgl2"] }
bevy_rapier3d = "0.27"
serde         = { version = "1", features = ["derive"] }
serde_json    = "1"
wasm-bindgen  = "0.2"
web-sys       = "0.3"

[profile.release]
opt-level = "z"
lto       = true
```

### LOD Strategy

```
Distance from player (in chunks) → LOD level
  0–4:   LOD 0 — full resolution (voxel)
  5–8:   LOD 1 — micro (8³ averaged)
  9–16:  LOD 2 — chunk (32³ averaged)
  17–32: LOD 3 — region (128³ averaged, impostor quad)
  >32:   not loaded
```

---

## Services

### Service Map

```
nova-ledger        — Event Ledger (NATS JetStream backend)
nova-zone          — Zone Server (ECS simulation per zone)
nova-aigm          — AI Game Master (LLM orchestration)
nova-voxel         — Chunk service (read/write, cache)
nova-api           — REST API (blueprint, players, auth)
nova-cdn           — WASM client + asset delivery
nova-vectordb      — NPC long-term memory (Qdrant)
```

### nova-ledger API

```
POST /ledger/:worldId/append
  body: WorldEvent[]
  → { seqStart: bigint, seqEnd: bigint, hash: number }

GET  /ledger/:worldId/events?from=<seq>&to=<seq>
  → WorldEvent[]

GET  /ledger/:worldId/head
  → { seq: bigint, hash: number, ts: number }

GET  /ledger/:worldId/replay?from=<seq>
  → SSE stream of WorldEvent[]
```

### nova-zone API

```
WS   /zone/:worldId/:zoneId
  ← ClientMessage
  → ServerMessage

POST /zone/:worldId/:zoneId/snapshot
  → ZoneSnapshot (current ECS state)

GET  /zone/:worldId/list
  → ZoneInfo[]
```

### nova-aigm API

```
POST /aigm/:worldId/tick
  body: { recentEvents: WorldEvent[], playerProfiles: PlayerProfile[] }
  → WorldEvent[]

POST /aigm/npc/:npcId/speak
  body: NpcLlmRequest
  → NpcLlmResponse (SSE stream)

GET  /aigm/:worldId/story
  → StoryArc[]
```

---

## Infrastructure

### Kubernetes Cluster

```
├── nova-ledger        (NATS JetStream, 3 replicas)
├── nova-zone          (StatefulSet, 1 pod per active zone)
│     └── HPA: scale zones by player count
├── nova-aigm          (Deployment, 2 replicas + GPU nodepool)
├── nova-voxel         (Deployment, Redis + S3 backend)
├── nova-api           (Deployment, 3 replicas)
│
├── Redis Cluster      (L2 chunk cache, NPC state)
├── PostgreSQL         (blueprints, players, auth)
├── Qdrant             (NPC vector memory)
├── NATS JetStream     (event ledger backbone)
│
└── S3 / GCS           (chunk cold storage, WASM builds, assets)

CDN (CloudFront / Fastly)
└── WASM client
└── Texture atlas
└── Audio assets
```

### Pulumi Stack Layout

```
nova-infra/
├── Pulumi.yaml
├── Pulumi.dev.yaml
├── Pulumi.prod.yaml
└── index.ts
    ├── cluster.ts        — Kubernetes cluster
    ├── databases.ts      — Postgres, Redis, Qdrant
    ├── messaging.ts      — NATS JetStream
    ├── storage.ts        — S3 buckets + CDN
    ├── services.ts       — K8s deployments per service
    └── aigm.ts           — GPU node pool + AI service
```

---

## Architektur-Vergleich

| | Aktuelles System | NOVA |
|---|---|---|
| World State | State + Delta Log | Pure Event Ledger |
| Konflikt-Resolution | CRC32 + Heal | Unmöglich (Event-Ordering) |
| Voxel-Speicher | 32³ chunks, 4-bit | Sparse Octree, Palette |
| Chunk-Größe | 16 KB flat | 2–8 KB (sparse komprimiert) |
| Networking | State Sync | Input Sync + Rollback |
| AI | Offline Blueprint Gen | RT Game Master System |
| NPC | Statische Daten | LLM mit persistent Memory |
| Time Travel | Partial (delta replay) | Vollständig (event fold) |
