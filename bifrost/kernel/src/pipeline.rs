//! [`EventPipeline`] — required event processing gateway.
//!
//! **Rule 3 — EventPipeline required.**
//!
//! Every event that changes world state MUST be processed through
//! `EventPipeline::process()`.  Raw event construction and direct ledger
//! appends without going through the pipeline are prohibited.
//!
//! ## Responsibilities
//!
//! 1. **Sequence assignment** — stamps each event with a monotonic
//!    [`SequencedInstant`] unique within the zone.
//! 2. **BLAKE3 chain advancement** — computes `world_hash[N]` from
//!    `world_hash[N-1]` and the new event's content hash.
//! 3. **Budget enforcement** — rejects events that exceed the per-tick
//!    event cap (configurable via [`PipelineConfig`]).
//! 4. **Ledger append** — delegates to the caller-supplied append callback,
//!    keeping the pipeline itself free of I/O dependencies.
//!
//! ## Example
//!
//! ```rust
//! use bifrost_kernel::{EventPipeline, SequencedInstant};
//!
//! let mut pipeline = EventPipeline::new("zone-overworld", 0, [0u8; 32]);
//! // process() returns the sequenced, hash-chained event
//! // (caller is responsible for the concrete event type via the RawEvent trait)
//! ```

use thiserror::Error;

use crate::clock::SequencedInstant;

// ─── Config ───────────────────────────────────────────────────────────────────

/// Per-tick event budget for a pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PipelineConfig {
    /// Maximum events accepted per tick before `TooManyEvents` is returned.
    pub max_events_per_tick: u32,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self { max_events_per_tick: 500 }
    }
}

// ─── Errors ───────────────────────────────────────────────────────────────────

/// Errors returned by [`EventPipeline::process`].
#[derive(Debug, Error, PartialEq)]
pub enum PipelineError {
    #[error("event budget exceeded: max {max} events per tick")]
    TooManyEvents { max: u32 },

    #[error("event zone_id '{event_zone}' does not match pipeline zone '{pipeline_zone}'")]
    ZoneMismatch { event_zone: String, pipeline_zone: String },

    #[error("event sequence {given} is already consumed; pipeline is at {current}")]
    StaleSequence { given: SequencedInstant, current: SequencedInstant },
}

// ─── RawEvent ─────────────────────────────────────────────────────────────────

/// Minimal interface the pipeline needs to stamp and chain an event.
///
/// Implement this on your concrete event type (e.g. `WorldEvent`).
pub trait RawEvent {
    /// Zone this event belongs to.
    fn zone_id(&self) -> &str;

    /// Compute a content hash over all payload fields.
    ///
    /// Must NOT include `instant` or `world_hash` — those are set by the
    /// pipeline after this call.
    fn content_hash(&self) -> [u8; 32];

    /// Stamp the event with its pipeline-assigned sequence position.
    fn set_instant(&mut self, instant: SequencedInstant);

    /// Set the BLAKE3 chain link computed by the pipeline.
    fn set_world_hash(&mut self, hash: [u8; 32]);
}

// ─── EventPipeline ────────────────────────────────────────────────────────────

/// The required gateway for all world-state-changing events.
///
/// Maintains the monotonic sequence counter and BLAKE3 integrity chain
/// for one zone.  A server node creates one pipeline per zone it owns.
pub struct EventPipeline {
    zone_id:       String,
    current:       SequencedInstant,
    head_hash:     [u8; 32],
    config:        PipelineConfig,
    events_this_tick: u32,
}

impl EventPipeline {
    /// Create a new pipeline for `zone_id`.
    ///
    /// `start_tick` is the first tick's number; `genesis_hash` is the
    /// initial chain head (use `[0u8; 32]` for the first run).
    pub fn new(
        zone_id:      impl Into<String>,
        start_tick:   u64,
        genesis_hash: [u8; 32],
    ) -> Self {
        Self {
            zone_id:          zone_id.into(),
            current:          SequencedInstant::new(start_tick, 0),
            head_hash:        genesis_hash,
            config:           PipelineConfig::default(),
            events_this_tick: 0,
        }
    }

    /// Override the default pipeline configuration.
    pub fn with_config(mut self, config: PipelineConfig) -> Self {
        self.config = config;
        self
    }

    /// Advance the pipeline clock to a new tick.
    ///
    /// Resets the per-tick event counter.  Must be called at the start of
    /// each simulation tick before processing that tick's events.
    pub fn advance_tick(&mut self, new_tick: u64) {
        self.current          = SequencedInstant::new(new_tick, 0);
        self.events_this_tick = 0;
    }

    /// Process one event through the pipeline.
    ///
    /// On success the event is stamped with a [`SequencedInstant`] and its
    /// `world_hash` is set to the new chain head.  The caller must then
    /// append it to the ledger.
    ///
    /// # Errors
    /// - [`PipelineError::TooManyEvents`] if the per-tick budget is full.
    /// - [`PipelineError::ZoneMismatch`] if the event targets a different zone.
    pub fn process<E: RawEvent>(&mut self, mut event: E) -> Result<E, PipelineError> {
        // R3: budget gate
        if self.events_this_tick >= self.config.max_events_per_tick {
            return Err(PipelineError::TooManyEvents { max: self.config.max_events_per_tick });
        }

        // R3: zone gate
        if event.zone_id() != self.zone_id {
            return Err(PipelineError::ZoneMismatch {
                event_zone:    event.zone_id().to_owned(),
                pipeline_zone: self.zone_id.clone(),
            });
        }

        // Stamp sequence (R5: no SystemTime — only SequencedInstant)
        let instant = self.current;
        event.set_instant(instant);

        // Advance BLAKE3 chain (R4: replay-safe hash chain)
        let content_hash  = event.content_hash();
        let new_head      = Self::chain_advance(&self.head_hash, &content_hash);
        event.set_world_hash(new_head);

        // Commit
        self.head_hash        = new_head;
        self.current          = self.current.next_seq();
        self.events_this_tick += 1;

        Ok(event)
    }

    /// Current chain head hash (after all processed events).
    pub fn head_hash(&self) -> &[u8; 32] {
        &self.head_hash
    }

    /// Current sequence position.
    pub fn current_instant(&self) -> SequencedInstant {
        self.current
    }

    // BLAKE3(prev || content_hash) — the canonical chain advance function.
    fn chain_advance(prev: &[u8; 32], content: &[u8; 32]) -> [u8; 32] {
        let mut h = blake3::Hasher::new();
        h.update(prev);
        h.update(content);
        *h.finalize().as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal test event that implements RawEvent.
    #[derive(Debug)]
    struct TestEvent {
        zone_id:    String,
        payload:    [u8; 4],
        instant:    Option<SequencedInstant>,
        world_hash: Option<[u8; 32]>,
    }

    impl TestEvent {
        fn new(zone_id: impl Into<String>, payload: [u8; 4]) -> Self {
            Self { zone_id: zone_id.into(), payload, instant: None, world_hash: None }
        }
    }

    impl RawEvent for TestEvent {
        fn zone_id(&self) -> &str { &self.zone_id }

        fn content_hash(&self) -> [u8; 32] {
            let mut h = blake3::Hasher::new();
            h.update(&self.payload);
            *h.finalize().as_bytes()
        }

        fn set_instant(&mut self, instant: SequencedInstant) {
            self.instant = Some(instant);
        }

        fn set_world_hash(&mut self, hash: [u8; 32]) {
            self.world_hash = Some(hash);
        }
    }

    #[test]
    fn stamps_sequence() {
        let mut p = EventPipeline::new("zone-a", 1, [0u8; 32]);
        let ev = p.process(TestEvent::new("zone-a", [1, 2, 3, 4])).unwrap();
        assert_eq!(ev.instant, Some(SequencedInstant::new(1, 0)));
    }

    #[test]
    fn sequence_increments() {
        let mut p = EventPipeline::new("zone-a", 1, [0u8; 32]);
        let _e0 = p.process(TestEvent::new("zone-a", [0; 4])).unwrap();
        let e1  = p.process(TestEvent::new("zone-a", [1; 4])).unwrap();
        assert_eq!(e1.instant, Some(SequencedInstant::new(1, 1)));
    }

    #[test]
    fn chain_is_deterministic() {
        let mut p1 = EventPipeline::new("z", 0, [0u8; 32]);
        let mut p2 = EventPipeline::new("z", 0, [0u8; 32]);
        let e1 = p1.process(TestEvent::new("z", [1, 2, 3, 4])).unwrap();
        let e2 = p2.process(TestEvent::new("z", [1, 2, 3, 4])).unwrap();
        assert_eq!(e1.world_hash, e2.world_hash);
    }

    #[test]
    fn zone_mismatch_rejected() {
        let mut p = EventPipeline::new("zone-a", 0, [0u8; 32]);
        let err = p.process(TestEvent::new("zone-b", [0; 4])).unwrap_err();
        assert!(matches!(err, PipelineError::ZoneMismatch { .. }));
    }

    #[test]
    fn budget_enforced() {
        let cfg = PipelineConfig { max_events_per_tick: 2 };
        let mut p = EventPipeline::new("z", 0, [0u8; 32]).with_config(cfg);
        p.process(TestEvent::new("z", [0; 4])).unwrap();
        p.process(TestEvent::new("z", [1; 4])).unwrap();
        let err = p.process(TestEvent::new("z", [2; 4])).unwrap_err();
        assert!(matches!(err, PipelineError::TooManyEvents { .. }));
    }

    #[test]
    fn advance_tick_resets_budget() {
        let cfg = PipelineConfig { max_events_per_tick: 1 };
        let mut p = EventPipeline::new("z", 0, [0u8; 32]).with_config(cfg);
        p.process(TestEvent::new("z", [0; 4])).unwrap();
        p.advance_tick(1);
        // Budget reset — should succeed again
        let ev = p.process(TestEvent::new("z", [1; 4])).unwrap();
        assert_eq!(ev.instant.unwrap().tick, 1);
    }
}
