#![warn(missing_docs)]

use std::collections::BTreeSet;
use std::fmt::Debug;
use std::iter::FusedIterator;

use serde::{Deserialize, Serialize};
use slotmap::{Key, SecondaryMap, SlotMap};

/// A directed multigraph where an vertex's inbound and outbound edges are indexed.
///
/// `DiMulGraph` does **not** allocate vertices `V`. The user shall use an external
/// [`SlotMap<V, _>`] for allocating vertices, which also allows the user to associate data with
/// each vertex.
///
/// `DiMulGraph` **does** allocate edges `E` as they are added. Additional data can be associated
/// with edges via an external [`SecondaryMap<E, _>`].
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(from = "EdgeList<V, E>", into = "EdgeList<V, E>")]
pub struct DiMulGraph<V, E>
where
    V: Key,
    E: Key,
{
    /// Edge list (src, dst).
    edges: SlotMap<E, (V, V)>,

    /// Successors for each vert.
    succs: SecondaryMap<V, Vec<E>>,
    /// Predecessors for each vert.
    preds: SecondaryMap<V, Vec<E>>,
}
impl<V, E> Default for DiMulGraph<V, E>
where
    V: Key,
    E: Key,
{
    fn default() -> Self {
        let (edges, succs, preds) = Default::default();
        Self {
            edges,
            succs,
            preds,
        }
    }
}
impl<V, E> DiMulGraph<V, E>
where
    V: Key,
    E: Key,
{
    /// Creates an empty `DiMulGraph`.
    pub fn new() -> Self {
        Default::default()
    }

    /// Creates a `DiMulGraph` with pre-allocated memory for `capacity` vertices and `capacity`
    /// edges.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            // Estimate 1 edge per vertex.
            edges: SlotMap::with_capacity_and_key(capacity),
            succs: SecondaryMap::with_capacity(capacity),
            preds: SecondaryMap::with_capacity(capacity),
        }
    }

    /// Assert that `self` is in a consistent state, for debugging. This is computationally
    /// expensive for large graphs.
    pub fn assert_valid(&self) {
        // Ensure each edge exists in the adj lists.
        for (edge_id, &(src, dst)) in self.edges.iter() {
            assert!(self.succs[src].contains(&edge_id));
            assert!(self.preds[dst].contains(&edge_id));
        }

        // Ensure no duplicate preds or succs
        for succ_list in self.succs.values() {
            let set: BTreeSet<&E> = succ_list.iter().collect();
            assert_eq!(set.len(), succ_list.len());
        }
        for pred_list in self.succs.values() {
            let set: BTreeSet<&E> = pred_list.iter().collect();
            assert_eq!(set.len(), pred_list.len());
        }

        // Note: Missing edges and duplicate edges could cancel each other out. But that case is
        // caught by the above.
        assert_eq!(
            self.edges.len(),
            self.succs.values().map(|vec| vec.len()).sum::<usize>(),
            "succs broken (contains duplicate or removed edge?)"
        );
        assert_eq!(
            self.edges.len(),
            self.preds.values().map(|vec| vec.len()).sum::<usize>(),
            "preds broken (contains duplicate or removed edge?)"
        );
    }

    /// HELPER, get the list out of the adj list `adj_list` for a particular vertex `v`.
    fn get_adj_edges(adj_list: &mut SecondaryMap<V, Vec<E>>, v: V) -> &mut Vec<E> {
        if !adj_list.contains_key(v) {
            adj_list.insert(v, Default::default());
        }
        &mut adj_list[v]
    }

    /// Creates an edge going from `src` to `dst` and returns the edge ID.
    pub fn insert_edge(&mut self, src: V, dst: V) -> E {
        let e = self.edges.insert((src, dst));
        Self::get_adj_edges(&mut self.succs, src).push(e);
        Self::get_adj_edges(&mut self.preds, dst).push(e);
        e
    }

    /// For an `edge` from `A --> B`, insert a new vertex `V` along that edge to create
    /// `A --e0--> V --e1--> B`. Returns the pair of new edge IDs in and out of `V`, i.e.
    /// `(e0, e1)`.
    ///
    /// Returns `None` if the edge doesn't exist.
    ///
    /// `edge` is removed from the graph, both returned edge IDs are new.
    pub fn insert_intermediate_vertex(&mut self, new_vertex: V, edge: E) -> Option<(E, E)> {
        self.assert_valid();

        // Remove old edge from edges.
        let (src, dst) = self.edges.remove(edge)?;

        // Insert new edges into edges.
        let e0 = self.edges.insert((src, new_vertex));
        let e1 = self.edges.insert((new_vertex, dst));

        // Remove old & add new edges in succs/preds.
        let succ_vec_idx = self.succs[src].iter().position(|&e| e == edge).unwrap();
        let pred_vec_idx = self.preds[dst].iter().position(|&e| e == edge).unwrap();
        assert_eq!(
            edge,
            std::mem::replace(&mut self.succs[src][succ_vec_idx], e0)
        );
        assert_eq!(
            edge,
            std::mem::replace(&mut self.preds[dst][pred_vec_idx], e1)
        );

        // Insert new vertex succs/preds.
        assert!(
            self.preds.insert(new_vertex, vec![e0]).is_none(),
            "Cannot insert intermediate vertex that already exists"
        );
        assert!(
            self.succs.insert(new_vertex, vec![e1]).is_none(),
            "Cannot insert intermediate vertex that already exists"
        );

        self.assert_valid();
        Some((e0, e1))
    }

    /// For a vertex with one incoming edge and one outgoing edge, removes the vertex. Inserts a new edge.
    /// Returns `(new edge, (old edge in, old edge out))`.
    /// Returns `None` if `vertex` is not in the graph or does not have the right degree in/out.
    pub fn remove_intermediate_vertex(&mut self, vertex: V) -> Option<(E, (E, E))> {
        let preds = self.preds.remove(vertex)?;
        let &[pred_edge] = &*preds else {
            return None;
        };
        let succs = self.succs.remove(vertex).unwrap();
        let &[succ_edge] = &*succs else {
            return None;
        };

        let (src, _v) = self.edges.remove(pred_edge).unwrap();
        let (_v, dst) = self.edges.remove(succ_edge).unwrap();

        self.succs[src].retain(|&e| e != pred_edge);
        self.preds[dst].retain(|&e| e != succ_edge);

        let new_edge = self.insert_edge(src, dst);
        Some((new_edge, (pred_edge, succ_edge)))
    }

    /// Remove an edge from the graph. If the edgeId is found then the edge is removed from the graph and returned.
    /// If the edgeId was not found in the graph then nothing is returned and nothing is done.
    pub fn remove_edge(&mut self, e: E) -> Option<(V, V)> {
        let (src, dst) = self.edges.remove(e)?;

        self.succs[src].retain(|x| *x != e);
        self.preds[dst].retain(|x| *x != e);

        Some((src, dst))
    }

    /// Remove a vertex from the graph, it must have no edges to or from it when doing this.
    pub fn remove_vertex(&mut self, v: V) {
        assert!(self.preds[v].is_empty() && self.succs[v].is_empty());

        self.preds.remove(v);
        self.succs.remove(v);
    }

    /// Get the source and destination vertex IDs for the given edge ID.
    pub fn edge(&self, e: E) -> Option<(V, V)> {
        self.edges.get(e).copied()
    }

    /// Return an iterator over all edge IDs `E`.
    pub fn edge_ids(&self) -> slotmap::basic::Keys<'_, E, (V, V)> {
        self.edges.keys()
    }

    /// Return an iterator over all edges in form `(E, (V, V))`.
    pub fn edges(
        &self,
    ) -> impl '_ + ExactSizeIterator<Item = (E, (V, V))> + FusedIterator + Clone + Debug {
        self.edges.iter().map(|(e, &(src, dst))| (e, (src, dst)))
    }

    /// Return an iterator of all edge IDs coming out of `v`.
    pub fn successor_edges(&self, v: V) -> std::iter::Copied<std::slice::Iter<'_, E>> {
        self.succs
            .get(v)
            .map(|v| v.iter())
            .unwrap_or_else(|| [].iter())
            .copied()
    }

    /// Return an iterator of all edge IDs going into `v`.
    pub fn predecessor_edges(&self, v: V) -> std::iter::Copied<std::slice::Iter<'_, E>> {
        self.preds
            .get(v)
            .map(|v| v.iter())
            .unwrap_or_else(|| [].iter())
            .copied()
    }

    /// Return an iterator of all successor vertex IDs of `v`.
    ///
    /// If there are multiple edges from `v` to the same vertex, that vertex will appear multiple times in the iterator.
    pub fn successor_vertices(
        &self,
        v: V,
    ) -> impl '_ + DoubleEndedIterator<Item = V> + ExactSizeIterator + FusedIterator + Clone + Debug
    {
        self.successor_edges(v).map(|edge_id| self.edges[edge_id].1)
    }

    /// Return an iterator of all predecessor vertex IDs of `v`.
    ///
    /// If there are multiple edges from a vertex to `v`, that vertex will appear multiple times in the iterator.
    pub fn predecessor_vertices(
        &self,
        v: V,
    ) -> impl '_ + DoubleEndedIterator<Item = V> + ExactSizeIterator + FusedIterator + Clone + Debug
    {
        self.predecessor_edges(v)
            .map(|edge_id| self.edges[edge_id].0)
    }

    /// Return an iterator of all successor edge IDs _and_ vertex IDs of `v` in form `(E, V)`.
    pub fn successors(
        &self,
        v: V,
    ) -> impl '_ + DoubleEndedIterator<Item = (E, V)> + ExactSizeIterator + FusedIterator + Clone + Debug
    {
        self.successor_edges(v)
            .map(|edge_id| (edge_id, self.edges[edge_id].1))
    }

    /// Return an iterator of all predecessor edge IDs _and_ vertex IDs of `v` in form `(E, V)`.
    pub fn predecessors(
        &self,
        v: V,
    ) -> impl '_ + DoubleEndedIterator<Item = (E, V)> + ExactSizeIterator + FusedIterator + Clone + Debug
    {
        self.predecessor_edges(v)
            .map(|edge_id| (edge_id, self.edges[edge_id].0))
    }

    /// The degree (number of edges/vertices) coming out of `v`, i.e. the number of successors.
    pub fn degree_out(&self, v: V) -> usize {
        self.succs.get(v).map(Vec::len).unwrap_or_default()
    }

    /// The degree (number of edges/vertices) going into `v`, i.e. the number of predecessors.
    pub fn degree_in(&self, v: V) -> usize {
        self.preds.get(v).map(Vec::len).unwrap_or_default()
    }
}

impl<V, E> From<DiMulGraph<V, E>> for EdgeList<V, E>
where
    V: Key,
    E: Key,
{
    fn from(value: DiMulGraph<V, E>) -> Self {
        value.edges
    }
}

impl<V, E> From<EdgeList<V, E>> for DiMulGraph<V, E>
where
    V: Key,
    E: Key,
{
    fn from(edges: EdgeList<V, E>) -> Self {
        let mut out = Self {
            edges,
            ..Default::default()
        };
        for (edge, &(src, dst)) in out.edges.iter() {
            out.succs.entry(src).unwrap().or_default().push(edge);
            out.preds.entry(dst).unwrap().or_default().push(edge);
        }
        out
    }
}

/// A compact edge list representation of a [`DiMulGraph`], used for serialization.
#[expect(type_alias_bounds, reason = "code readability")]
pub type EdgeList<V, E>
where
    V: Key,
    E: Key,
= SlotMap<E, (V, V)>;
