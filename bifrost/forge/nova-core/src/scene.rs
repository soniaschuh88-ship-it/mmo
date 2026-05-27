//! Scene graph — parent/child entity hierarchy with world-matrix computation.
//!
//! Each entity may have one parent and any number of children.
//! World matrices are computed recursively: `M_world = M_parent_world * M_local`.

use std::collections::BTreeMap;

use crate::ecs::EntityId;
use crate::transform::{Mat4, Transform3D};

// ─── SceneNode ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct SceneNode {
    pub parent:   Option<EntityId>,
    pub children: Vec<EntityId>,
}

// ─── SceneGraph ───────────────────────────────────────────────────────────────

/// Manages the parent/child hierarchy for the scene.
///
/// Entities that are not explicitly attached to a parent are **roots**.
#[derive(Debug, Default)]
pub struct SceneGraph {
    nodes: BTreeMap<EntityId, SceneNode>,
    roots: Vec<EntityId>,
}

impl SceneGraph {
    pub fn new() -> Self { Self::default() }

    /// Register `id` as a root-level entity (no parent).
    pub fn add_root(&mut self, id: EntityId) {
        self.nodes.entry(id).or_default();
        if !self.roots.contains(&id) { self.roots.push(id); }
    }

    /// Attach `child` under `parent`.  Detaches from previous parent if any.
    pub fn attach(&mut self, parent: EntityId, child: EntityId) {
        // Detach from current parent / root list
        if let Some(old_p) = self.nodes.get(&child).and_then(|n| n.parent) {
            if let Some(pn) = self.nodes.get_mut(&old_p) {
                pn.children.retain(|&c| c != child);
            }
        } else {
            self.roots.retain(|&r| r != child);
        }
        self.nodes.entry(child).or_default().parent = Some(parent);
        self.nodes.entry(parent).or_default().children.push(child);
    }

    /// Remove `id` from the graph.  Its children are reparented to `id`'s parent,
    /// or promoted to roots if `id` was a root.
    pub fn remove(&mut self, id: EntityId) {
        let Some(node) = self.nodes.remove(&id) else { return };

        for &ch in &node.children {
            if let Some(cn) = self.nodes.get_mut(&ch) { cn.parent = node.parent; }
            match node.parent {
                Some(gp) => {
                    if let Some(gpn) = self.nodes.get_mut(&gp) {
                        if !gpn.children.contains(&ch) { gpn.children.push(ch); }
                    }
                }
                None => { if !self.roots.contains(&ch) { self.roots.push(ch); } }
            }
        }

        if let Some(pid) = node.parent {
            if let Some(pn) = self.nodes.get_mut(&pid) { pn.children.retain(|&c| c != id); }
        } else {
            self.roots.retain(|&r| r != id);
        }
    }

    pub fn parent(&self, id: EntityId) -> Option<EntityId> {
        self.nodes.get(&id)?.parent
    }

    pub fn children(&self, id: EntityId) -> &[EntityId] {
        self.nodes.get(&id).map(|n| n.children.as_slice()).unwrap_or(&[])
    }

    pub fn roots(&self) -> &[EntityId] { &self.roots }

    /// Compute the world matrix for `id` by multiplying up the hierarchy.
    ///
    /// `get_local` is a closure that looks up the local [`Transform3D`] for
    /// any entity — typically `|id| world.get::<Transform3D>(id)`.
    pub fn world_matrix<'a>(
        &self,
        id: EntityId,
        get_local: &dyn Fn(EntityId) -> Option<&'a Transform3D>,
    ) -> Mat4 {
        let local = get_local(id)
            .map(|t| t.to_matrix())
            .unwrap_or(Mat4::IDENTITY);

        match self.nodes.get(&id).and_then(|n| n.parent) {
            Some(parent) => Mat4::mul(&self.world_matrix(parent, get_local), &local),
            None         => local,
        }
    }

    /// Breadth-first traversal — visits roots first, then their children.
    pub fn bfs(&self) -> Vec<EntityId> {
        let mut out = Vec::new();
        let mut q: std::collections::VecDeque<EntityId> =
            self.roots.iter().copied().collect();
        while let Some(id) = q.pop_front() {
            out.push(id);
            for &ch in self.children(id) { q.push_back(ch); }
        }
        out
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u64) -> EntityId { EntityId(n) }

    #[test]
    fn root_registration() {
        let mut sg = SceneGraph::new();
        sg.add_root(id(1));
        assert!(sg.roots().contains(&id(1)));
    }

    #[test]
    fn parent_child_attachment() {
        let mut sg = SceneGraph::new();
        sg.add_root(id(1));
        sg.attach(id(1), id(2));
        assert_eq!(sg.parent(id(2)), Some(id(1)));
        assert!(sg.children(id(1)).contains(&id(2)));
        assert!(!sg.roots().contains(&id(2)));
    }

    #[test]
    fn remove_promotes_children_to_grandparent() {
        let mut sg = SceneGraph::new();
        sg.add_root(id(1));
        sg.attach(id(1), id(2));
        sg.attach(id(2), id(3));
        sg.remove(id(2));
        assert!(sg.children(id(1)).contains(&id(3)));
        assert!(!sg.roots().contains(&id(3)));
    }

    #[test]
    fn bfs_breadth_first() {
        let mut sg = SceneGraph::new();
        sg.add_root(id(1));
        sg.attach(id(1), id(2));
        sg.attach(id(1), id(3));
        sg.attach(id(2), id(4));
        let order = sg.bfs();
        assert_eq!(order[0], id(1));
        // id(2) and id(3) must come before id(4)
        let pos = |x: EntityId| order.iter().position(|&o| o == x).unwrap();
        assert!(pos(id(2)) < pos(id(4)));
        assert!(pos(id(3)) < pos(id(4)));
    }
}
