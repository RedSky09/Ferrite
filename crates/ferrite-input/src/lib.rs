use std::collections::HashSet;

/// Keys the engine knows about. Physical positions on the keyboard, so WASD
/// stays in the same place on other layouts (QWERTY, AZERTY, ...).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Left,
    Right,
    Up,
    Down,
    W,
    A,
    S,
    D,
    Space,
    Enter,
    Escape,
}

/// Keyboard state, as seen by game code.
///
/// The engine feeds it with `handle_key`, and clears the "just pressed" and
/// "just released" lists after every game update.
#[derive(Default, Debug)]
pub struct Input {
    down: HashSet<Key>,
    pressed: HashSet<Key>,
    released: HashSet<Key>,
}

impl Input {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a key event. Repeated "pressed" events for a key that is
    /// already down (OS key repeat) are ignored.
    pub fn handle_key(&mut self, key: Key, is_pressed: bool) {
        if is_pressed {
            if self.down.insert(key) {
                self.pressed.insert(key);
            }
        } else if self.down.remove(&key) {
            self.released.insert(key);
        }
    }

    /// Releases every held key, for example when the window loses focus.
    pub fn release_all(&mut self) {
        for key in self.down.drain() {
            self.released.insert(key);
        }
    }

    /// True while the key is held down.
    pub fn is_down(&self, key: Key) -> bool {
        self.down.contains(&key)
    }

    /// True if the key went down since the last game update.
    pub fn just_pressed(&self, key: Key) -> bool {
        self.pressed.contains(&key)
    }

    /// True if the key was released since the last game update.
    pub fn just_released(&self, key: Key) -> bool {
        self.released.contains(&key)
    }

    /// Forgets the "just pressed" and "just released" lists. The engine calls
    /// this after each game update, so every press is seen by exactly one update.
    pub fn clear_transient(&mut self) {
        self.pressed.clear();
        self.released.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn press_sets_down_and_just_pressed() {
        let mut input = Input::new();
        input.handle_key(Key::W, true);
        assert!(input.is_down(Key::W));
        assert!(input.just_pressed(Key::W));
        assert!(!input.just_released(Key::W));
    }

    #[test]
    fn clear_transient_keeps_held_keys() {
        let mut input = Input::new();
        input.handle_key(Key::W, true);
        input.clear_transient();
        assert!(input.is_down(Key::W));
        assert!(!input.just_pressed(Key::W));
    }

    #[test]
    fn os_key_repeat_does_not_retrigger_just_pressed() {
        let mut input = Input::new();
        input.handle_key(Key::Space, true);
        input.clear_transient();
        input.handle_key(Key::Space, true); // repeat event from the OS
        assert!(!input.just_pressed(Key::Space));
        assert!(input.is_down(Key::Space));
    }

    #[test]
    fn release_sets_just_released() {
        let mut input = Input::new();
        input.handle_key(Key::A, true);
        input.clear_transient();
        input.handle_key(Key::A, false);
        assert!(!input.is_down(Key::A));
        assert!(input.just_released(Key::A));
    }

    #[test]
    fn press_and_release_between_updates_is_not_lost() {
        let mut input = Input::new();
        input.handle_key(Key::Enter, true);
        input.handle_key(Key::Enter, false);
        // No update ran in between, so both edges are still visible.
        assert!(input.just_pressed(Key::Enter));
        assert!(input.just_released(Key::Enter));
        assert!(!input.is_down(Key::Enter));
    }

    #[test]
    fn release_all_releases_held_keys() {
        let mut input = Input::new();
        input.handle_key(Key::Left, true);
        input.handle_key(Key::D, true);
        input.clear_transient();
        input.release_all();
        assert!(!input.is_down(Key::Left) && !input.is_down(Key::D));
        assert!(input.just_released(Key::Left) && input.just_released(Key::D));
    }
}
