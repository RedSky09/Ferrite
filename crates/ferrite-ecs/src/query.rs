//! Queries: iterate entities that have a given set of components.
//!
//! A query is described by a type: `&Pos` for a single component,
//! `(&Pos, &mut Vel)` for a combination. The iterator walks the smallest
//! involved store and skips entities that do not have the others, which keeps
//! the common "lots of one component, few of another" case fast.

use crate::entity::Entity;
use crate::World;

/// A query over immutable component references.
pub trait Query {
    /// The yielded item for one matching entity.
    type Item<'a>;
    /// The smallest store to drive iteration, or `None` if any store is missing.
    fn driver(world: &World) -> Option<Vec<u32>>;
    /// Fetches the item for `index`, if every part matches.
    fn fetch<'a>(world: &'a World, index: u32) -> Option<(Entity, Self::Item<'a>)>;
}

impl<T: 'static> Query for &T {
    type Item<'a> = &'a T;

    fn driver(world: &World) -> Option<Vec<u32>> {
        let store = world.raw_store::<T>()?;
        Some(store.indices().to_vec())
    }

    fn fetch<'a>(world: &'a World, index: u32) -> Option<(Entity, Self::Item<'a>)> {
        let store = world.raw_store::<T>()?;
        let item = store.get(index)?;
        Some((world.entity_at_index(index), item))
    }
}

impl<A, B> Query for (&A, &B)
where
    A: 'static,
    B: 'static,
{
    type Item<'a> = (&'a A, &'a B);

    fn driver(world: &World) -> Option<Vec<u32>> {
        let a = world.raw_store::<A>()?;
        let b = world.raw_store::<B>()?;
        // Drive from the smaller store.
        let driver = if a.len() <= b.len() {
            a.indices()
        } else {
            b.indices()
        };
        Some(driver.to_vec())
    }

    fn fetch<'a>(world: &'a World, index: u32) -> Option<(Entity, Self::Item<'a>)> {
        let a = world.raw_store::<A>()?.get(index)?;
        let b = world.raw_store::<B>()?.get(index)?;
        Some((world.entity_at_index(index), (a, b)))
    }
}

impl<A, B, C> Query for (&A, &B, &C)
where
    A: 'static,
    B: 'static,
    C: 'static,
{
    type Item<'a> = (&'a A, &'a B, &'a C);

    fn driver(world: &World) -> Option<Vec<u32>> {
        let a = world.raw_store::<A>()?;
        let b = world.raw_store::<B>()?;
        let c = world.raw_store::<C>()?;
        let smallest = [a.len(), b.len(), c.len()].into_iter().min()?;
        let pick = if a.len() == smallest {
            a.indices()
        } else if b.len() == smallest {
            b.indices()
        } else {
            c.indices()
        };
        Some(pick.to_vec())
    }

    fn fetch<'a>(world: &'a World, index: u32) -> Option<(Entity, Self::Item<'a>)> {
        let a = world.raw_store::<A>()?.get(index)?;
        let b = world.raw_store::<B>()?.get(index)?;
        let c = world.raw_store::<C>()?.get(index)?;
        Some((world.entity_at_index(index), (a, b, c)))
    }
}

/// The iterator returned by [`World::query`].
pub struct QueryIter<'w, Q: Query> {
    world: &'w World,
    indices: std::vec::IntoIter<u32>,
    _marker: std::marker::PhantomData<Q>,
}

impl<'w, Q: Query> QueryIter<'w, Q> {
    pub(crate) fn new(world: &'w World) -> Self {
        let indices = Q::driver(world).unwrap_or_default();
        Self {
            world,
            indices: indices.into_iter(),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<'w, Q: Query> Iterator for QueryIter<'w, Q> {
    type Item = (Entity, Q::Item<'w>);

    fn next(&mut self) -> Option<Self::Item> {
        for index in self.indices.by_ref() {
            if let Some(item) = Q::fetch(self.world, index) {
                return Some(item);
            }
        }
        None
    }
}
