//! Handles that point at an entity slot.

use std::fmt;

/// A handle to an entity.
///
/// The handle carries a generation counter as well as an index. When an entity
/// is despawned and its slot reused, the generation is bumped, so an old
/// handle no longer matches the new occupant. This turns a whole class of
/// use-after-free bugs into a cheap equality check.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity {
    index: u32,
    generation: u32,
}

impl Entity {
    /// The slot index. Only the indexer in [`crate::World`] should care.
    pub(crate) const fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    pub const fn index(self) -> u32 {
        self.index
    }

    pub const fn generation(self) -> u32 {
        self.generation
    }
}

impl fmt::Debug for Entity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Entity({}v{})", self.index, self.generation)
    }
}
