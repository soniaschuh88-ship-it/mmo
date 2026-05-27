//! Engine time state and countdown timers.

/// Per-frame time data.  Updated once per frame by the host runtime.
#[derive(Debug, Clone, Default)]
pub struct Time {
    /// Seconds since the previous frame (capped at 50 ms to prevent spiral-of-death).
    pub delta:   f32,
    /// Total seconds elapsed since the engine started.
    pub elapsed: f64,
    /// Frame counter (1-based after first [`Time::advance`] call).
    pub frame:   u64,
    /// Accumulated time not yet consumed by [`Time::consume_fixed`].
    accum:       f32,
}

impl Time {
    /// Target duration for one fixed-update step.
    pub const FIXED_STEP: f32 = 1.0 / 60.0;

    /// Advance by `raw` seconds.
    pub fn advance(&mut self, raw: f32) {
        self.delta   = raw.min(0.05);
        self.elapsed += self.delta as f64;
        self.frame   += 1;
        self.accum   += self.delta;
    }

    /// Returns `true` and consumes one fixed step if enough time has
    /// accumulated.  Call in a `while` loop to drain all pending steps.
    pub fn consume_fixed(&mut self) -> bool {
        if self.accum >= Self::FIXED_STEP {
            self.accum -= Self::FIXED_STEP;
            true
        } else {
            false
        }
    }
}

// ─── Timer ────────────────────────────────────────────────────────────────────

/// Countdown timer: fires after `duration` seconds, optionally looping.
#[derive(Debug, Clone)]
pub struct Timer {
    /// Total duration in seconds.
    pub duration:   f32,
    remaining:      f32,
    pub looping:    bool,
    finished:       bool,
    /// `true` on the exact frame the timer fired.
    pub just_fired: bool,
}

impl Timer {
    pub fn new(duration: f32, looping: bool) -> Self {
        Self { duration, remaining: duration, looping, finished: false, just_fired: false }
    }

    /// Advance the timer by `delta` seconds.  Returns `true` if it fired this tick.
    pub fn tick(&mut self, delta: f32) -> bool {
        self.just_fired = false;
        // One-shot timers stop counting down once finished.
        if self.finished && !self.looping { return false; }
        self.remaining -= delta;
        if self.remaining <= 0.0 {
            self.just_fired = true;
            if self.looping {
                self.remaining += self.duration;
            } else {
                self.remaining = 0.0;
                self.finished  = true;
            }
            return true;
        }
        false
    }

    pub fn reset(&mut self) {
        self.remaining  = self.duration;
        self.finished   = false;
        self.just_fired = false;
    }

    /// Elapsed fraction in `[0, 1]` (0 = just started, 1 = finished).
    pub fn fraction(&self) -> f32 {
        1.0 - (self.remaining / self.duration).clamp(0.0, 1.0)
    }

    pub fn is_finished(&self) -> bool { self.finished }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_advance_delta() {
        let mut t = Time::default();
        t.advance(0.016);
        assert!((t.delta - 0.016).abs() < 1e-6);
        assert_eq!(t.frame, 1);
    }

    #[test]
    fn time_delta_cap() {
        let mut t = Time::default();
        t.advance(1.0); // would cause spiral-of-death if uncapped
        assert!(t.delta <= 0.05);
    }

    #[test]
    fn time_fixed_steps() {
        let mut t = Time::default();
        t.advance(1.0 / 30.0); // two fixed steps at 60 Hz
        assert!(t.consume_fixed());
        assert!(t.consume_fixed());
        assert!(!t.consume_fixed());
    }

    #[test]
    fn timer_one_shot_fires_once() {
        let mut t = Timer::new(0.5, false);
        assert!(!t.tick(0.3));
        assert!(t.tick(0.3));  // fires at 0.6 total — past 0.5
        // After firing the one-shot timer is exhausted; remaining == 0
        assert!(!t.tick(0.3)); // no more
        assert!(t.is_finished());
    }

    #[test]
    fn timer_looping_fires_multiple() {
        let mut t = Timer::new(0.1, true);
        let mut fires = 0u32;
        for _ in 0..15 { if t.tick(0.016) { fires += 1; } }
        assert!(fires >= 2);
    }

    #[test]
    fn timer_fraction() {
        let t = Timer::new(1.0, false);
        assert!((t.fraction() - 0.0).abs() < 1e-5);
    }
}
