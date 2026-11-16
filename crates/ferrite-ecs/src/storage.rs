//! Type-erased component storage.
//!
//! Each component type gets its own [`SparseSet`]. `World` keeps them in a map
//! keyed by `TypeId` and talks to them through the [`ComponentStore`] trait, so
//! it can hold heterogeneous types while the per-type storage stays strongly
//! typed and fast.

use std::any::Any;
use std::collections::HashMap;

/// Operations `World` needs from a store without knowing the component type.
pub(crate) trait ComponentStore: Any {
    /// Removes the component for `index`, if present.
    fn remove_index(&mut self, index: u32);
    /// Drops every component.
    fn clear(&mut self);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// A sparse set of components: dense storage for iteration, an index map for
/// O(1) lookup.
///
/// `dense` holds the components back to back, `entities` holds the slot index
/// of each dense entry, and `sparse` maps a slot index to its position in
/// `dense`. Removing swaps the last element into the hole, so it does not
/// preserve order (which nothing relies on).
pub(crate) struct SparseSet<T> {
    sparse: HashMap<u32, u32>,
    entities: Vec<u32>,
    dense: Vec<T>,
}

impl<T> SparseSet<T> {
    pub(crate) fn new() -> Self {
        Self {
            sparse: HashMap::new(),
            entities: Vec::new(),
            dense: Vec::new(),
        }
    }

    /// Inserts a value, returning the previous one if this slot was occupied.
    pub(crate) fn insert(&mut self, index: u32, value: T) -> Option<T> {
        if let Some(&pos) = self.sparse.get(&index) {
            Some(std::mem::replace(&mut self.dense[pos as usize], value))
        } else {
            let pos = self.dense.len() as u32;
            self.sparse.insert(index, pos);
            self.entities.push(index);
            self.dense.push(value);
            None
        }
    }

    pub(crate) fn get(&self, index: u32) -> Option<&T> {
        self.sparse.get(&index).map(|&pos| &self.dense[pos as usize])
    }

    pub(crate) fn get_mut(&mut self, index: u32) -> Option<&mut T> {
        let pos = *self.sparse.get(&index)?;
        Some(&mut self.dense[pos as usize])
    }

    pub(crate) fn remove(&mut self, index: u32) -> Option<T> {
        let pos = self.sparse.remove(&index)?;
        let value = self.dense.swap_remove(pos as usize);
        self.entities.swap_remove(pos as usize);
        // If the removed slot was not the last, the element that was at the end
        // has just swapped into `pos`; update its sparse link to match.
        if (pos as usize) < self.entities.len() {
            let moved_index = self.entities[pos as usize];
            self.sparse.insert(moved_index, pos);
        }
        Some(value)
    }

    pub(crate) fn len(&self) -> usize {
        self.dense.len()
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (u32, &T)> {
        self.entities.iter().copied().zip(self.dense.iter())
    }

    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = (u32, &mut T)> {
        self.entities.iter().copied().zip(self.dense.iter_mut())
    }

    /// The slot indices that have a component, in dense order.
    pub(crate) fn indices(&self) -> &[u32] {
        &self.entities
    }
}

impl<T> ComponentStore for SparseSet<T>
where
    T: 'static,
{
    fn remove_index(&mut self, index: u32) {
        self.remove(index);
    }

    fn clear(&mut self) {
        self.sparse.clear();
        self.entities.clear();
        self.dense.clear();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_get_and_overwrite() {
        let mut s = SparseSet::<i32>::new();
        s.insert(7, 10);
        assert_eq!(s.get(7), Some(&10));
        s.insert(7, 20); // overwrite
        assert_eq!(s.get(7), Some(&20));
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn remove_updates_the_moved_element() {
        let mut s = SparseSet::<i32>::new();
        s.insert(1, 10);
        s.insert(2, 20);
        s.insert(3, 30);
        // Remove the middle; the last element swaps into its place.
        assert_eq!(s.remove(2), Some(20));
        assert_eq!(s.get(2), None);
        // The moved element is still reachable at its index.
        assert_eq!(s.get(3), Some(&30));
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn remove_missing_is_a_no_op() {
        let mut s = SparseSet::<i32>::new();
        s.insert(1, 10);
        assert_eq!(s.remove(2), None);
        assert_eq!(s.get(1), Some(&10));
    }

    #[test]
    fn iteration_matches_contents() {
        let mut s = SparseSet::<i32>::new();
        s.insert(5, 50);
        s.insert(9, 90);
        let mut pairs: Vec<_> = s.iter().map(|(i, v)| (i, *v)).collect();
        pairs.sort();
        assert_eq!(pairs, vec![(5, 50), (9, 90)]);
    }

    #[test]
    fn iter_mut_can_change_values() {
        let mut s = SparseSet::<i32>::new();
        s.insert(1, 1);
        s.insert(2, 2);
        for (_, v) in s.iter_mut() {
            *v *= 10;
        }
        assert_eq!(s.get(1), Some(&10));
        assert_eq!(s.get(2), Some(&20));
    }

    #[test]
    fn clear_empties_the_set() {
        let mut s = SparseSet::<i32>::new();
        s.insert(1, 1);
        s.insert(2, 2);
        s.clear();
        assert_eq!(s.len(), 0);
        assert!(s.get(1).is_none());
    }
}
