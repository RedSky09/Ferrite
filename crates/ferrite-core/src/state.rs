//! A tiny state machine for game states (Menu, Playing, GameOver, ...).
//!
//! The state is an enum supplied by the game. Transitions are *requests*, not
//! direct assignments, so a transition can be raised from inside `update` and
//! applied at a single, predictable point. This keeps `on_enter`/`on_exit`
//! hooks from firing in the middle of a partially updated state.

use std::marker::PhantomData;

/// A game state value.
///
/// Blanket-implemented for any `Copy + PartialEq` type, so games do not need
/// to implement anything. Requiring `Copy` keeps the machine cheap to sample
/// every frame and avoids surprises about when a state is cloned.
pub trait State: Copy + PartialEq + 'static {}

impl<T> State for T where T: Copy + PartialEq + 'static {}

/// Tracks the current state and any pending transition.
///
/// The game owns one of these, typically as a field on its `Game` struct.
pub struct StateMachine<S: State> {
    current: S,
    next: Option<S>,
    /// Bumped on every applied transition. Games can compare it to detect
    /// "the state changed this frame" without tracking the previous value.
    revision: u64,
    _marker: PhantomData<S>,
}

impl<S: State> StateMachine<S> {
    /// Starts in `state`.
    pub fn new(state: S) -> Self {
        Self {
            current: state,
            next: None,
            revision: 0,
            _marker: PhantomData,
        }
    }

    /// The state active right now.
    pub fn current(&self) -> S {
        self.current
    }

    /// Requests a transition to `state`.
    ///
    /// The change takes effect on the next call to [`StateMachine::apply`], so
    /// the remainder of the current update still sees the old state. A single
    /// frame can request several times; the last request wins.
    pub fn request(&mut self, state: S) {
        self.next = Some(state);
    }

    /// Applies a pending transition, if any.
    ///
    /// Returns `Some((from, to))` when the state actually changed, otherwise
    /// `None`. Call this once per update, after the game's logic has run.
    pub fn apply(&mut self) -> Option<(S, S)> {
        let next = self.next.take()?;
        if next == self.current {
            return None;
        }
        let from = self.current;
        self.current = next;
        self.revision += 1;
        Some((from, next))
    }

    /// True if a transition is waiting to be applied.
    pub fn is_pending(&self) -> bool {
        self.next.is_some()
    }

    /// How many transitions have been applied. Useful to detect a change.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// True if the current state is `state`.
    pub fn is(&self, state: S) -> bool {
        self.current == state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum GameState {
        Menu,
        Playing,
        GameOver,
    }

    #[test]
    fn starts_in_the_given_state() {
        let sm = StateMachine::new(GameState::Menu);
        assert!(sm.is(GameState::Menu));
        assert_eq!(sm.revision(), 0);
    }

    #[test]
    fn a_request_is_applied_on_apply() {
        let mut sm = StateMachine::new(GameState::Menu);
        sm.request(GameState::Playing);
        // Not applied yet; the current state is unchanged.
        assert!(sm.is(GameState::Menu));
        assert!(sm.is_pending());

        let change = sm.apply();
        assert_eq!(change, Some((GameState::Menu, GameState::Playing)));
        assert!(sm.is(GameState::Playing));
        assert_eq!(sm.revision(), 1);
    }

    #[test]
    fn requesting_the_current_state_is_a_no_op() {
        let mut sm = StateMachine::new(GameState::Menu);
        sm.request(GameState::Menu);
        assert_eq!(sm.apply(), None);
        assert_eq!(sm.revision(), 0);
    }

    #[test]
    fn apply_with_no_request_returns_none() {
        let mut sm = StateMachine::new(GameState::Playing);
        assert_eq!(sm.apply(), None);
    }

    #[test]
    fn the_last_request_in_a_frame_wins() {
        let mut sm = StateMachine::new(GameState::Menu);
        sm.request(GameState::Playing);
        sm.request(GameState::GameOver);
        assert_eq!(sm.apply(), Some((GameState::Menu, GameState::GameOver)));
    }

    #[test]
    fn revision_only_counts_real_changes() {
        let mut sm = StateMachine::new(GameState::Menu);
        sm.request(GameState::Menu); // no-op
        sm.apply();
        sm.request(GameState::Playing);
        sm.apply();
        assert_eq!(sm.revision(), 1);
    }
}
