//! A small, self-contained entity-component system.
//!
//! The design follows the engine's rules: no global state, components are
//! plain data, and everything lives in an ordinary struct ([`World`]) that a
//! game can own, pass around, and later serialize.
//!
//! # Model
//!
//! - An [`Entity`] is a handle: an index plus a generation. Despawning bumps
//!   the generation, so a stale handle never aliases a reused slot.
//! - A component is any `'static` value. Each type is stored in its own sparse
//!   set, keyed by `TypeId` under the hood; no registration step.
//! - [`World::spawn`] makes entities, [`World::insert`] attaches components,
//!   and queries read them back.
//!
//! # Querying
//!
//! [`World::query`] iterates immutable references over a single component or a
//! tuple of them:
//!
//! ```
//! # use ferrite_ecs::World;
//! struct Pos(i32, i32);
//! struct Vel(i32, i32);
//! let mut world = World::new();
//! let e = world.spawn();
//! world.insert(e, Pos(0, 0));
//! world.insert(e, Vel(1, 0));
//!
//! // Single component.
//! for (entity, pos) in world.query::<&Pos>() {
//!     let _ = (entity, pos);
//! }
//!
//! // Tuples of components: only entities that have all of them match.
//! for (entity, (pos, vel)) in world.query::<(&Pos, &Vel)>() {
//!     let _ = (entity, pos, vel);
//! }
//! ```
//!
//! Mutable access goes through [`World::iter_mut`] or [`World::get_mut`]. A
//! combined mutable query (for example `&mut Vel` alongside `&Pos`) is left for
//! later once the demo game shows it is needed.

mod entity;
mod query;
mod storage;
mod world;

pub use entity::Entity;
pub use query::{Query, QueryIter};
pub use world::World;
