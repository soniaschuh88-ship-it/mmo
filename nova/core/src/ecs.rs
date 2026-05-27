//! Sparse-set entity component system.
//!
//! ## Design
//!
//! - Components are any `'static + Send + Sync` type — no trait impl needed.
//! - One [`BTreeMap`]-backed [`Store`] per component type → deterministic
//!   iteration order across platforms (required for lockstep networking).
//! - [`World::query`] returns an iterator; [`World::query2_ids`] returns the
//!   set of entities that carry both of two component types.

use std::any::{Any, TypeId};
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::transform::Transform3D;

// ─── EntityId ─────────────────────────────────────────────────────────────────

/// Stable, opaque entity handle.  Monotonically increasing; never reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EntityId(pub u64);

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "e{}", self.0)
    }
}

// ─── Component marker ─────────────────────────────────────────────────────────

/// Blanket marker — every `'static + Send + Sync` type is a component.
pub trait Component: Any + Send + Sync + 'static {}
impl<T: Any + Send + Sync + 'static> Component for T {}

// ─── Internal type-erased store ───────────────────────────────────────────────

struct Store {
    data: BTreeMap<EntityId, Box<dyn Any + Send + Sync>>,
}

impl Store {
    fn new() -> Self { Self { data: BTreeMap::new() } }

    fn insert<C: Component>(&mut self, id: EntityId, c: C) {
        self.data.insert(id, Box::new(c));
    }

    fn get<C: Component>(&self, id: EntityId) -> Option<&C> {
        self.data.get(&id)?.downcast_ref()
    }

    fn get_mut<C: Component>(&mut self, id: EntityId) -> Option<&mut C> {
        self.data.get_mut(&id)?.downcast_mut()
    }

    fn remove(&mut self, id: EntityId) { self.data.remove(&id); }

    fn ids(&self) -> impl Iterator<Item = EntityId> + '_ { self.data.keys().copied() }
}

// ─── World ────────────────────────────────────────────────────────────────────

/// The central ECS container.
///
/// # Example
///
/// ```rust,ignore
/// use nova_core::{World, Transform3D, Vec3, Name};
///
/// let mut world = World::new();
/// let player = world.spawn();
/// world.insert(player, Transform3D::at(Vec3::new(10.0, 0.0, 10.0)));
/// world.insert(player, Name::new("Player"));
///
/// for (id, t) in world.query::<Transform3D>() {
///     println!("{id}  pos={:?}", t.position);
/// }
/// ```
pub struct World {
    next:    u64,
    alive:   BTreeMap<EntityId, ()>,
    stores:  BTreeMap<TypeId, Store>,
}

impl World {
    pub fn new() -> Self {
        Self { next: 1, alive: BTreeMap::new(), stores: BTreeMap::new() }
    }

    // ── Entity lifecycle ──────────────────────────────────────────────────────

    /// Allocate a new entity with no components.
    pub fn spawn(&mut self) -> EntityId {
        let id = EntityId(self.next);
        self.next += 1;
        self.alive.insert(id, ());
        id
    }

    /// Destroy an entity and all its components.
    pub fn despawn(&mut self, id: EntityId) {
        self.alive.remove(&id);
        for store in self.stores.values_mut() { store.remove(id); }
    }

    pub fn is_alive(&self, id: EntityId) -> bool { self.alive.contains_key(&id) }
    pub fn entity_count(&self) -> usize { self.alive.len() }

    // ── Component access ──────────────────────────────────────────────────────

    /// Attach or replace component `C` on `id`.
    pub fn insert<C: Component>(&mut self, id: EntityId, c: C) {
        self.stores
            .entry(TypeId::of::<C>())
            .or_insert_with(Store::new)
            .insert(id, c);
    }

    /// Detach component `C` from `id` (no-op if absent).
    pub fn remove<C: Component>(&mut self, id: EntityId) {
        if let Some(s) = self.stores.get_mut(&TypeId::of::<C>()) { s.remove(id); }
    }

    pub fn get<C: Component>(&self, id: EntityId) -> Option<&C> {
        self.stores.get(&TypeId::of::<C>())?.get(id)
    }

    pub fn get_mut<C: Component>(&mut self, id: EntityId) -> Option<&mut C> {
        self.stores.get_mut(&TypeId::of::<C>())?.get_mut(id)
    }

    pub fn has<C: Component>(&self, id: EntityId) -> bool { self.get::<C>(id).is_some() }

    // ── Queries ───────────────────────────────────────────────────────────────

    /// Iterate all alive entities that carry component `C`.
    pub fn query<C: Component>(&self) -> impl Iterator<Item = (EntityId, &C)> {
        self.stores
            .get(&TypeId::of::<C>())
            .into_iter()
            .flat_map(|s| {
                s.ids()
                    .filter_map(|id| s.get::<C>(id).map(|c| (id, c)))
            })
            .filter(|(id, _)| self.alive.contains_key(id))
    }

    /// Returns every alive entity that has **both** component `A` and `B`.
    pub fn query2_ids<A: Component, B: Component>(&self) -> Vec<EntityId> {
        let Some(sa) = self.stores.get(&TypeId::of::<A>()) else { return vec![] };
        let tb = TypeId::of::<B>();
        sa.ids()
            .filter(|&id| {
                self.alive.contains_key(&id)
                    && self.stores
                        .get(&tb)
                        .map(|sb| sb.get::<B>(id).is_some())
                        .unwrap_or(false)
            })
            .collect()
    }

    /// How many alive entities carry component `C`.
    pub fn component_count<C: Component>(&self) -> usize {
        self.stores
            .get(&TypeId::of::<C>())
            .map(|s| s.ids().filter(|id| self.alive.contains_key(id)).count())
            .unwrap_or(0)
    }
}

impl Default for World { fn default() -> Self { Self::new() } }

// ─── Built-in tag components ──────────────────────────────────────────────────

/// Human-readable debug label for an entity (shown in nova-editor inspector).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Name(pub String);
impl Name {
    pub fn new(s: impl Into<String>) -> Self { Self(s.into()) }
    pub fn as_str(&self) -> &str { &self.0 }
}

/// Arbitrary string tags; used for filtering queries ("enemy", "boss", "npc").
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Tags(pub Vec<String>);
impl Tags {
    pub fn has(&self, tag: &str) -> bool { self.0.iter().any(|t| t == tag) }
    pub fn add(&mut self, tag: impl Into<String>) { self.0.push(tag.into()); }
}

/// Marks an entity as inactive — most systems skip entities with this component.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Disabled;

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transform::Vec3;

    #[derive(Debug, Clone)]
    struct Health { hp: i32 }

    #[test]
    fn spawn_and_despawn() {
        let mut w = World::new();
        let e = w.spawn();
        assert!(w.is_alive(e));
        assert_eq!(w.entity_count(), 1);
        w.despawn(e);
        assert!(!w.is_alive(e));
        assert_eq!(w.entity_count(), 0);
    }

    #[test]
    fn insert_and_get() {
        let mut w = World::new();
        let e = w.spawn();
        w.insert(e, Health { hp: 100 });
        assert_eq!(w.get::<Health>(e).unwrap().hp, 100);
    }

    #[test]
    fn remove_component() {
        let mut w = World::new();
        let e = w.spawn();
        w.insert(e, Health { hp: 50 });
        w.remove::<Health>(e);
        assert!(w.get::<Health>(e).is_none());
    }

    #[test]
    fn query_all() {
        let mut w = World::new();
        for i in 0..5 {
            let e = w.spawn();
            w.insert(e, Health { hp: i });
        }
        assert_eq!(w.query::<Health>().count(), 5);
    }

    #[test]
    fn query2_ids_intersection() {
        let mut w = World::new();
        let e1 = w.spawn();
        w.insert(e1, Health { hp: 80 });
        w.insert(e1, Transform3D::at(Vec3::new(1.0, 0.0, 0.0)));
        let e2 = w.spawn();
        w.insert(e2, Health { hp: 20 }); // no transform
        let both = w.query2_ids::<Health, Transform3D>();
        assert_eq!(both.len(), 1);
        assert_eq!(both[0], e1);
    }

    #[test]
    fn despawn_removes_all_components() {
        let mut w = World::new();
        let e = w.spawn();
        w.insert(e, Name::new("hero"));
        w.insert(e, Health { hp: 100 });
        w.despawn(e);
        assert!(w.get::<Name>(e).is_none());
        assert!(w.get::<Health>(e).is_none());
        assert_eq!(w.component_count::<Name>(), 0);
    }

    #[test]
    fn tags() {
        let mut t = Tags::default();
        t.add("enemy"); t.add("boss");
        assert!(t.has("boss"));
        assert!(!t.has("player"));
    }
}
