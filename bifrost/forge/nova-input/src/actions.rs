//! Input types, action map, and per-frame query API.

use std::collections::{BTreeMap, BTreeSet};
use serde::{Deserialize, Serialize};

// ─── KeyCode ──────────────────────────────────────────────────────────────────

/// Keyboard key codes — names match `KeyboardEvent.code` in the browser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum KeyCode {
    KeyW, KeyA, KeyS, KeyD,
    ArrowUp, ArrowDown, ArrowLeft, ArrowRight,
    Space, ShiftLeft, ControlLeft, AltLeft,
    KeyE, KeyQ, KeyF, KeyR, KeyT, KeyK, KeyI, KeyM, KeyP, KeyH,
    Digit1, Digit2, Digit3, Digit4, Digit5,
    Digit6, Digit7, Digit8, Digit9, Digit0,
    Escape, Enter, Tab,
    F1, F2, F3, F4, F5,
}

// ─── MouseButton ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MouseButton { Left, Middle, Right }

// ─── ActionId ─────────────────────────────────────────────────────────────────

/// A named game action.  Use the constants in [`actions`] rather than
/// constructing these directly.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ActionId(pub String);

impl ActionId {
    pub fn new(s: impl Into<String>) -> Self { Self(s.into()) }
}

// ─── Standard action constants ────────────────────────────────────────────────

/// Pre-defined MMO action names.  Import with `use nova_input::game_actions::*;`.
pub mod actions {
    use super::ActionId;
    pub fn move_forward()  -> ActionId { ActionId::new("move_forward")  }
    pub fn move_back()     -> ActionId { ActionId::new("move_back")     }
    pub fn move_left()     -> ActionId { ActionId::new("move_left")     }
    pub fn move_right()    -> ActionId { ActionId::new("move_right")    }
    pub fn attack()        -> ActionId { ActionId::new("attack")        }
    pub fn interact()      -> ActionId { ActionId::new("interact")      }
    pub fn open_quest()    -> ActionId { ActionId::new("open_quest")    }
    pub fn open_skills()   -> ActionId { ActionId::new("open_skills")   }
    pub fn open_map()      -> ActionId { ActionId::new("open_map")      }
    pub fn use_skill_1()   -> ActionId { ActionId::new("use_skill_1")   }
    pub fn use_skill_2()   -> ActionId { ActionId::new("use_skill_2")   }
    pub fn use_skill_3()   -> ActionId { ActionId::new("use_skill_3")   }
    pub fn use_skill_4()   -> ActionId { ActionId::new("use_skill_4")   }
    pub fn cancel()        -> ActionId { ActionId::new("cancel")        }
    pub fn sprint()        -> ActionId { ActionId::new("sprint")        }
}

// ─── Binding ──────────────────────────────────────────────────────────────────

/// One concrete input source bound to an action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Binding {
    Key(KeyCode),
    MouseBtn(MouseButton),
}

// ─── InputMap ─────────────────────────────────────────────────────────────────

/// Maps [`ActionId`]s to their [`Binding`]s.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct InputMap {
    bindings: BTreeMap<ActionId, Vec<Binding>>,
}

impl InputMap {
    pub fn bind_key(&mut self, action: ActionId, key: KeyCode) {
        self.bindings.entry(action).or_default().push(Binding::Key(key));
    }

    pub fn bind_mouse(&mut self, action: ActionId, btn: MouseButton) {
        self.bindings.entry(action).or_default().push(Binding::MouseBtn(btn));
    }

    pub fn bindings_for(&self, action: &ActionId) -> &[Binding] {
        self.bindings.get(action).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Default WASD + mouse-left bindings for the NOVA MMO client.
    ///
    /// Matches the keybindings in `app/game.html` so the Rust and JS
    /// layers stay in sync.
    pub fn default_mmo() -> Self {
        let mut m = Self::default();
        use KeyCode::*;
        use actions::*;

        // Movement
        m.bind_key(move_forward(), KeyW);
        m.bind_key(move_forward(), ArrowUp);
        m.bind_key(move_back(),    KeyS);
        m.bind_key(move_back(),    ArrowDown);
        m.bind_key(move_left(),    KeyA);
        m.bind_key(move_left(),    ArrowLeft);
        m.bind_key(move_right(),   KeyD);
        m.bind_key(move_right(),   ArrowRight);
        // Interaction
        m.bind_key(interact(),     KeyE);
        m.bind_key(open_quest(),   KeyQ);
        m.bind_key(open_skills(),  KeyK);
        m.bind_key(open_map(),     KeyM);
        // Skills
        m.bind_key(use_skill_1(),  Digit1);
        m.bind_key(use_skill_2(),  Digit2);
        m.bind_key(use_skill_3(),  Digit3);
        m.bind_key(use_skill_4(),  Digit4);
        // System
        m.bind_key(cancel(),       Escape);
        m.bind_key(sprint(),       ShiftLeft);
        m.bind_mouse(attack(),     MouseButton::Left);
        m
    }
}

// ─── InputState ───────────────────────────────────────────────────────────────

/// Per-frame raw input state.  Updated by the host (browser event handlers or
/// the test harness).
#[derive(Debug, Default)]
pub struct InputState {
    /// Keys currently held (persists across frames).
    pub held_keys:        BTreeSet<KeyCode>,
    /// Keys pressed *this* frame (cleared by [`InputState::begin_frame`]).
    pub pressed_keys:     BTreeSet<KeyCode>,
    /// Keys released this frame.
    pub released_keys:    BTreeSet<KeyCode>,
    /// Mouse buttons currently held.
    pub held_buttons:     BTreeSet<MouseButton>,
    /// Mouse buttons pressed this frame.
    pub pressed_buttons:  BTreeSet<MouseButton>,
    /// Current mouse position in logical CSS pixels.
    pub mouse_pos:        (f32, f32),
    /// Mouse delta since last frame.
    pub mouse_delta:      (f32, f32),
    /// Scroll wheel delta this frame.
    pub scroll_delta:     f32,
}

impl InputState {
    /// Call **once at the start of each frame** to clear single-frame events.
    pub fn begin_frame(&mut self) {
        self.pressed_keys.clear();
        self.released_keys.clear();
        self.pressed_buttons.clear();
        self.mouse_delta  = (0.0, 0.0);
        self.scroll_delta = 0.0;
    }

    pub fn key_down(&mut self, k: KeyCode) {
        if self.held_keys.insert(k) { self.pressed_keys.insert(k); }
    }

    pub fn key_up(&mut self, k: KeyCode) {
        self.held_keys.remove(&k);
        self.released_keys.insert(k);
    }

    pub fn mouse_down(&mut self, b: MouseButton) {
        if self.held_buttons.insert(b) { self.pressed_buttons.insert(b); }
    }

    pub fn mouse_up(&mut self, b: MouseButton) { self.held_buttons.remove(&b); }
}

// ─── ActionQuery ──────────────────────────────────────────────────────────────

/// Query action state for the current frame.
///
/// ```rust,ignore
/// let map   = InputMap::default_mmo();
/// let state = InputState::default();
///
/// let q = ActionQuery::new(&map, &state);
/// let (dx, dy) = q.movement();
/// if q.just_pressed(&actions::attack()) { ... }
/// ```
pub struct ActionQuery<'a> {
    pub map:   &'a InputMap,
    pub state: &'a InputState,
}

impl<'a> ActionQuery<'a> {
    pub fn new(map: &'a InputMap, state: &'a InputState) -> Self { Self { map, state } }

    /// `true` while any binding for `action` is held.
    pub fn held(&self, action: &ActionId) -> bool {
        self.map.bindings_for(action).iter().any(|b| match b {
            Binding::Key(k)       => self.state.held_keys.contains(k),
            Binding::MouseBtn(mb) => self.state.held_buttons.contains(mb),
        })
    }

    /// `true` only on the **first frame** a binding is pressed.
    pub fn just_pressed(&self, action: &ActionId) -> bool {
        self.map.bindings_for(action).iter().any(|b| match b {
            Binding::Key(k)       => self.state.pressed_keys.contains(k),
            Binding::MouseBtn(mb) => self.state.pressed_buttons.contains(mb),
        })
    }

    /// `true` on the frame a binding was released.
    pub fn just_released(&self, action: &ActionId) -> bool {
        self.map.bindings_for(action).iter().any(|b| match b {
            Binding::Key(k)       => self.state.released_keys.contains(k),
            Binding::MouseBtn(_)  => false,
        })
    }

    /// Normalized 2-D movement vector `(x, y)` from WASD / arrows.
    ///
    /// `y < 0` = forward, `y > 0` = back (matches the isometric world in `game.html`).
    pub fn movement(&self) -> (f32, f32) {
        let mut x = 0.0_f32;
        let mut y = 0.0_f32;
        if self.held(&actions::move_right())   { x += 1.0; }
        if self.held(&actions::move_left())    { x -= 1.0; }
        if self.held(&actions::move_back())    { y += 1.0; }
        if self.held(&actions::move_forward()) { y -= 1.0; }
        let l = (x*x + y*y).sqrt();
        if l > 0.0 { (x/l, y/l) } else { (0.0, 0.0) }
    }

    /// `true` when the sprint modifier is held.
    pub fn is_sprinting(&self) -> bool { self.held(&actions::sprint()) }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (InputMap, InputState) {
        let map   = InputMap::default_mmo();
        let state = InputState::default();
        (map, state)
    }

    #[test]
    fn move_forward_wasd() {
        let (map, mut state) = setup();
        state.begin_frame();
        state.key_down(KeyCode::KeyW);

        let q = ActionQuery::new(&map, &state);
        let (_, dy) = q.movement();
        assert!(dy < 0.0, "forward should be negative Y, got {dy}");
        assert!(q.held(&actions::move_forward()));
        assert!(q.just_pressed(&actions::move_forward()));
    }

    #[test]
    fn just_pressed_is_single_frame() {
        let (map, mut state) = setup();

        // Frame 1 — key pressed
        state.begin_frame();
        state.key_down(KeyCode::KeyE);
        assert!(ActionQuery::new(&map, &state).just_pressed(&actions::interact()));

        // Frame 2 — key still held but NOT just pressed
        state.begin_frame();
        assert!(!ActionQuery::new(&map, &state).just_pressed(&actions::interact()));
        assert!( ActionQuery::new(&map, &state).held(&actions::interact()));
    }

    #[test]
    fn key_released() {
        let (map, mut state) = setup();
        state.begin_frame();
        state.key_down(KeyCode::KeyW);
        state.begin_frame();
        state.key_up(KeyCode::KeyW);
        assert!(ActionQuery::new(&map, &state).just_released(&actions::move_forward()));
        assert!(!ActionQuery::new(&map, &state).held(&actions::move_forward()));
    }

    #[test]
    fn mouse_attack() {
        let (map, mut state) = setup();
        state.begin_frame();
        state.mouse_down(MouseButton::Left);
        assert!(ActionQuery::new(&map, &state).just_pressed(&actions::attack()));
    }

    #[test]
    fn diagonal_movement_normalized() {
        let (map, mut state) = setup();
        state.begin_frame();
        state.key_down(KeyCode::KeyW);
        state.key_down(KeyCode::KeyD);
        let (x, y) = ActionQuery::new(&map, &state).movement();
        let len = (x*x + y*y).sqrt();
        assert!((len - 1.0).abs() < 1e-5, "diagonal should be normalized, len={len}");
    }

    #[test]
    fn sprint_key() {
        let (map, mut state) = setup();
        state.begin_frame();
        state.key_down(KeyCode::ShiftLeft);
        assert!(ActionQuery::new(&map, &state).is_sprinting());
    }
}
