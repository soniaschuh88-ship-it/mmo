//! [`StateTransitionFn`] and [`ApplyTransition`] — single mutation path.
//!
//! **Rule 2 — Single mutation path.**
//!
//! All state changes in the BIFROST system MUST be expressed as a
//! [`StateTransitionFn`].  Direct field assignment (`s.field = x`) is
//! prohibited outside of the owning type's constructor or a named
//! transition function.
//!
//! ## Usage pattern
//!
//! ```rust
//! use bifrost_kernel::{StateTransitionFn, ApplyTransition};
//!
//! #[derive(Clone)]
//! struct RunState { active: bool, tick: u64 }
//! impl ApplyTransition for RunState {}
//!
//! fn activate(tick: u64) -> StateTransitionFn<RunState> {
//!     Box::new(move |s| RunState { active: true, tick, ..s })
//! }
//!
//! let state = RunState { active: false, tick: 0 };
//! let next  = state.apply(activate(42));
//! assert!(next.active);
//! assert_eq!(next.tick, 42);
//! ```
//!
//! ## Why `Box<dyn Fn(S) -> S>`?
//!
//! Using a boxed closure allows transitions to capture parameters
//! (e.g. the winning faction, a tick number) without requiring a separate
//! enum variant for every possible state change.

/// A pure function that produces the next state from the current state.
///
/// All mutation paths MUST go through this type.  Closures capture
/// any parameters needed for the transition.
///
/// **Invariant:** the function must be pure — no side-effects, no I/O,
/// no `SystemTime`, no randomness.  Same input → same output always.
pub type StateTransitionFn<S> = Box<dyn FnOnce(S) -> S + Send + 'static>;

/// Extension trait that lets any `Sized` type apply a [`StateTransitionFn`].
///
/// Implement this on your state structs to opt into the single mutation path.
pub trait ApplyTransition: Sized {
    /// Consume `self`, apply the transition, and return the new state.
    ///
    /// This is the **only** valid way to mutate BIFROST state.
    fn apply(self, f: StateTransitionFn<Self>) -> Self {
        f(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, PartialEq, Debug)]
    struct Counter { value: u32 }
    impl ApplyTransition for Counter {}

    #[test]
    fn apply_transition_produces_new_state() {
        let c = Counter { value: 0 };
        let next = c.apply(Box::new(|s| Counter { value: s.value + 5 }));
        assert_eq!(next.value, 5);
    }

    #[test]
    fn chained_transitions() {
        let c = Counter { value: 10 };
        let c2 = c.apply(Box::new(|s| Counter { value: s.value + 1 }));
        let c3 = c2.apply(Box::new(|s| Counter { value: s.value * 2 }));
        assert_eq!(c3.value, 22); // (10+1)*2
    }
}
