//! The world: the container for entities and their components.
//!
//! Nothing here is global. A `World` is an ordinary struct you can own, pass
//! around, and (later) snapshot. Systems are plain functions that take
//! `&mut World`.

use std::any::TypeId;
use std::collections::HashMap;

use crate::entity::Entity;
use crate::query::Query;
use crate::storage::{ComponentStore, SparseSet};

/// Which slot is alive, and which generation it is on.
struct Slot {
    generation: u32,
    alive: bool,
}

/// Holds every entity and component.
///
/// Components are stored in sparse sets keyed by `TypeId`. A component type is
/// any `'static` value, so `#[derive(Clone, Copy)] struct Position(f32, f32)`
/// works with no registration.
pub struct World {
    slots: Vec<Slot>,
    free: Vec<u32>,
    stores: HashMap<TypeId, Box<dyn ComponentStore>>,
}

impl World {
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            free: Vec::new(),
            stores: HashMap::new(),
        }
    }

    /// Creates an entity with no components.
    pub fn spawn(&mut self) -> Entity {
        if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index as usize];
            slot.alive = true;
            Entity::new(index, slot.generation)
        } else {
            let index = self.slots.len() as u32;
            self.slots.push(Slot {
                generation: 0,
                alive: true,
            });
            Entity::new(index, 0)
        }
    }

    /// Destroys an entity and drops all of its components.
    ///
    /// The slot is reused later with a bumped generation, so any old handle to
    /// it stops matching.
    pub fn despawn(&mut self, entity: Entity) {
        if !self.is_alive(entity) {
            return;
        }
        let index = entity.index();
        for store in self.stores.values_mut() {
            store.remove_index(index);
        }
        let slot = &mut self.slots[index as usize];
        slot.alive = false;
        slot.generation = slot.generation.wrapping_add(1);
        self.free.push(index);
    }

    /// True if `entity` still refers to a living entity.
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.slots
            .get(entity.index() as usize)
            .is_some_and(|s| s.alive && s.generation == entity.generation())
    }

    /// Number of living entities.
    pub fn entity_count(&self) -> usize {
        self.slots.iter().filter(|s| s.alive).count()
    }

    /// Inserts or overwrites a component on `entity`, returning the old value.
    ///
    /// # Panics
    /// Panics if `entity` is not alive.
    pub fn insert<T: 'static>(&mut self, entity: Entity, value: T) -> Option<T> {
        assert!(self.is_alive(entity), "insert on a dead entity: {entity:?}");
        self.store_mut::<T>().insert(entity.index(), value)
    }

    /// Removes a component, returning it if it was present.
    pub fn remove<T: 'static>(&mut self, entity: Entity) -> Option<T> {
        if !self.is_alive(entity) {
            return None;
        }
        self.store_mut::<T>().remove(entity.index())
    }

    /// Borrows a component.
    pub fn get<T: 'static>(&self, entity: Entity) -> Option<&T> {
        if !self.is_alive(entity) {
            return None;
        }
        self.store::<T>().and_then(|s| s.get(entity.index()))
    }

    /// Mutably borrows a component.
    pub fn get_mut<T: 'static>(&mut self, entity: Entity) -> Option<&mut T> {
        if !self.is_alive(entity) {
            return None;
        }
        self.store_mut::<T>().get_mut(entity.index())
    }

    /// True if `entity` has a `T`.
    pub fn has<T: 'static>(&self, entity: Entity) -> bool {
        self.get::<T>(entity).is_some()
    }

    /// Iterates over every `(Entity, &T)` pair.
    pub fn iter<T: 'static>(&self) -> impl Iterator<Item = (Entity, &T)> {
        self.store::<T>()
            .into_iter()
            .flat_map(|s| s.iter().map(|(index, v)| (self.entity_at(index), v)))
    }

    /// Iterates over every `(Entity, &mut T)` pair.
    pub fn iter_mut<T: 'static>(&mut self) -> impl Iterator<Item = (Entity, &mut T)> {
        // First, snapshot the generation of every slot that has a `T`, using
        // only a shared borrow. The mutable borrow of the store comes after.
        let entries: Vec<(u32, u32)> = match self.store::<T>() {
            Some(store) => store
                .indices()
                .iter()
                .map(|&index| (index, self.slots[index as usize].generation))
                .collect(),
            None => Vec::new(),
        };
        self.store_mut::<T>()
            .iter_mut()
            .zip(entries)
            .map(|((index, value), (_, generation))| (Entity::new(index, generation), value))
    }

    /// Starts a query over one or more component types.
    ///
    /// ```
    /// # use ferrite_ecs::World;
    /// #[derive(Debug, PartialEq)] struct Pos(i32);
    /// let mut world = World::new();
    /// let e = world.spawn();
    /// world.insert(e, Pos(3));
    /// let found: Vec<_> = world.query::<&Pos>().map(|(_, p)| p.0).collect();
    /// assert_eq!(found, vec![3]);
    /// ```
    pub fn query<Q: Query>(&self) -> crate::query::QueryIter<'_, Q> {
        crate::query::QueryIter::new(self)
    }

    /// Removes every entity and component.
    pub fn clear(&mut self) {
        self.slots.clear();
        self.free.clear();
        for store in self.stores.values_mut() {
            store.clear();
        }
    }

    // --- internals -------------------------------------------------------

    fn entity_at(&self, index: u32) -> Entity {
        Entity::new(index, self.slots[index as usize].generation)
    }

    /// The entity for a slot index, for the query iterator.
    pub(crate) fn entity_at_index(&self, index: u32) -> Entity {
        self.entity_at(index)
    }

    fn store<T: 'static>(&self) -> Option<&SparseSet<T>> {
        self.stores
            .get(&TypeId::of::<T>())
            .and_then(|s| s.as_any().downcast_ref::<SparseSet<T>>())
    }

    fn store_mut<T: 'static>(&mut self) -> &mut SparseSet<T> {
        let store = self
            .stores
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(SparseSet::<T>::new()));
        store
            .as_any_mut()
            .downcast_mut::<SparseSet<T>>()
            .expect("store registered under the wrong type id")
    }

    /// Used by the query iterator: the raw sparse set for `T`, if any.
    pub(crate) fn raw_store<T: 'static>(&self) -> Option<&SparseSet<T>> {
        self.store::<T>()
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Pos(i32, i32);
    #[derive(Debug, PartialEq)]
    struct Vel(i32);

    #[test]
    fn spawn_get_insert_remove() {
        let mut w = World::new();
        let e = w.spawn();
        assert!(w.is_alive(e));
        assert!(w.get::<Pos>(e).is_none());

        w.insert(e, Pos(1, 2));
        assert_eq!(w.get::<Pos>(e), Some(&Pos(1, 2)));

        w.get_mut::<Pos>(e).unwrap().0 = 9;
        assert_eq!(w.get::<Pos>(e), Some(&Pos(9, 2)));

        assert_eq!(w.remove::<Pos>(e), Some(Pos(9, 2)));
        assert!(w.get::<Pos>(e).is_none());
    }

    #[test]
    fn insert_overwrites() {
        let mut w = World::new();
        let e = w.spawn();
        assert_eq!(w.insert(e, Vel(1)), None);
        assert_eq!(w.insert(e, Vel(2)), Some(Vel(1)));
        assert_eq!(w.get::<Vel>(e), Some(&Vel(2)));
    }

    #[test]
    fn despawn_removes_components_and_invalidates_the_handle() {
        let mut w = World::new();
        let e = w.spawn();
        w.insert(e, Pos(0, 0));
        w.despawn(e);
        assert!(!w.is_alive(e));
        assert!(w.get::<Pos>(e).is_none());
        assert_eq!(w.entity_count(), 0);
    }

    #[test]
    fn reused_slot_gets_a_new_generation() {
        let mut w = World::new();
        let a = w.spawn();
        w.despawn(a);
        let b = w.spawn();
        // Same slot index, different generation.
        assert_eq!(a.index(), b.index());
        assert_ne!(a.generation(), b.generation());
        // The old handle must not alias the new entity.
        assert!(!w.is_alive(a));
        assert!(w.is_alive(b));
    }

    #[test]
    fn despawn_is_idempotent() {
        let mut w = World::new();
        let e = w.spawn();
        w.despawn(e);
        w.despawn(e); // second call is a no-op
        assert_eq!(w.entity_count(), 0);
    }

    #[test]
    fn components_are_independent_per_type() {
        let mut w = World::new();
        let e = w.spawn();
        w.insert(e, Pos(1, 1));
        w.insert(e, Vel(2));
        w.remove::<Pos>(e);
        assert!(!w.has::<Pos>(e));
        assert!(w.has::<Vel>(e));
    }

    #[test]
    fn clear_empties_everything() {
        let mut w = World::new();
        let e = w.spawn();
        w.insert(e, Pos(1, 1));
        w.clear();
        assert_eq!(w.entity_count(), 0);
        assert!(w.get::<Pos>(e).is_none());
    }

    #[test]
    #[should_panic(expected = "dead entity")]
    fn insert_on_dead_entity_panics() {
        let mut w = World::new();
        let e = w.spawn();
        w.despawn(e);
        w.insert(e, Pos(0, 0));
    }

    #[test]
    fn query_single_component_yields_only_its_holders() {
        let mut w = World::new();
        let a = w.spawn();
        let _b = w.spawn();
        w.insert(a, Pos(1, 1));
        // `_b` has no Pos, so it must not appear.
        let found: Vec<_> = w.query::<&Pos>().map(|(e, p)| (e, p.0)).collect();
        assert_eq!(found, vec![(a, 1)]);
    }

    #[test]
    fn query_tuple_requires_every_component() {
        let mut w = World::new();
        let both = w.spawn();
        let only_pos = w.spawn();
        let only_vel = w.spawn();
        w.insert(both, Pos(1, 0));
        w.insert(both, Vel(2));
        w.insert(only_pos, Pos(9, 9));
        w.insert(only_vel, Vel(9));

        let found: Vec<_> = w.query::<(&Pos, &Vel)>().map(|(e, _)| e).collect();
        assert_eq!(found, vec![both]);
    }

    #[test]
    fn query_three_components() {
        #[derive(Debug, PartialEq)]
        struct Tag;
        let mut w = World::new();
        let e = w.spawn();
        w.insert(e, Pos(1, 1));
        w.insert(e, Vel(2));
        w.insert(e, Tag);
        let count = w.query::<(&Pos, &Vel, &Tag)>().count();
        assert_eq!(count, 1);
    }

    #[test]
    fn query_on_a_missing_type_yields_nothing() {
        let mut w = World::new();
        let e = w.spawn();
        w.insert(e, Pos(1, 1));
        // No Vel was ever inserted, so there is no store to drive iteration.
        assert_eq!(w.query::<&Vel>().count(), 0);
        assert_eq!(w.query::<(&Pos, &Vel)>().count(), 0);
    }

    #[test]
    fn query_skips_despawned_entities() {
        let mut w = World::new();
        let a = w.spawn();
        let b = w.spawn();
        w.insert(a, Pos(1, 1));
        w.insert(b, Pos(2, 2));
        w.despawn(a);
        let remaining: Vec<_> = w.query::<&Pos>().map(|(e, _)| e).collect();
        assert_eq!(remaining, vec![b]);
    }

    #[test]
    fn iter_mut_yields_valid_handles_and_allows_edits() {
        let mut w = World::new();
        let a = w.spawn();
        let b = w.spawn();
        w.insert(a, Vel(1));
        w.insert(b, Vel(2));
        let mut seen = Vec::new();
        for (e, v) in w.iter_mut::<Vel>() {
            seen.push(e);
            v.0 += 10;
        }
        seen.sort_by_key(|e| e.index());
        assert_eq!(seen.len(), 2);
        assert_eq!(w.get::<Vel>(a).unwrap().0, 11);
        assert_eq!(w.get::<Vel>(b).unwrap().0, 12);
    }
}
