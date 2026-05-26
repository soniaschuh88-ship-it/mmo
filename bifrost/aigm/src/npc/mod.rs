//! # NPC system — three-layer architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │ Layer 1 — Reactive (every tick, O(1) per NPC)               │
//! │   State machine: Idle → Patrol → Chase → Flee → Speak       │
//! │   No LLM. No async. Pure deterministic transitions.          │
//! └─────────────────────────────────────────────────────────────┘
//!          │ player_speak | world_event | quest_trigger
//!          ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │ Layer 2 — LLM trigger (event-driven, rate-limited)          │
//! │   Builds NpcLlmRequest from AiContext + trigger event.       │
//! │   Checks Layer 3 cache before dispatching.                   │
//! │   Emits AigmNpcSpeak + AigmNpcGoalSet events.               │
//! └─────────────────────────────────────────────────────────────┘
//!          │ cache miss
//!          ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │ Layer 3 — Prompt cache (BLAKE3-keyed)                       │
//! │   key = BLAKE3(model || system_prompt || context_window)     │
//! │   Identical context → identical response, zero LLM calls.   │
//! │   Bounded ring buffer (max 512 entries per zone).            │
//! └─────────────────────────────────────────────────────────────┘
//! ```

pub mod behavior;
pub mod context;
pub mod dialogue;
pub mod memory;
pub mod registry;

pub use behavior::{NpcBehavior, BehaviorState, BehaviorTransition};
pub use context::{AiContext, Mood};
pub use dialogue::{
    NpcDialogueTrigger, NpcLlmRequest, NpcLlmResponse, NpcAction,
    DialogueQueue, PendingDialogue,
};
pub use memory::{ShortTermMemory, MemoryEntry, PromptCache};
pub use registry::{NpcState, NpcRegistry};
