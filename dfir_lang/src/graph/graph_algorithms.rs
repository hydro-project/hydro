//! General graph algorithm utility functions

use std::collections::BTreeMap;

use slotmap::{Key, SecondaryMap};

/// Topologically sorts a set of nodes. Returns a list where the order of `Id`s will agree with
/// the order of any path through the graph.
///
/// This succeeds if the input is a directed acyclic graph (DAG).
///
/// If the input has a cycle, an `Err` will be returned containing the cycle. Each node in the
/// cycle will be listed exactly once.
///
/// <https://en.wikipedia.org/wiki/Topological_sorting>
pub fn topo_sort<Id, NodeIds, PredsFn, PredsIter>(
    node_ids: NodeIds,
    mut preds_fn: PredsFn,
) -> Result<Vec<Id>, Vec<Id>>
where
    Id: Copy + Eq + Ord,
    NodeIds: IntoIterator<Item = Id>,
    PredsFn: FnMut(Id) -> PredsIter,
    PredsIter: IntoIterator<Item = Id>,
{
    let (mut marked, mut order) = Default::default();

    fn pred_dfs_postorder<Id, PredsFn, PredsIter>(
        node_id: Id,
        preds_fn: &mut PredsFn,
        marked: &mut BTreeMap<Id, bool>, // `false` => temporary, `true` => permanent.
        order: &mut Vec<Id>,
    ) -> Result<(), ()>
    where
        Id: Copy + Eq + Ord,
        PredsFn: FnMut(Id) -> PredsIter,
        PredsIter: IntoIterator<Item = Id>,
    {
        match marked.get(&node_id) {
            Some(_permanent @ true) => Ok(()),
            Some(_temporary @ false) => {
                // Cycle found!
                order.clear();
                order.push(node_id);
                Err(())
            }
            None => {
                marked.insert(node_id, false);
                for next_pred in (preds_fn)(node_id) {
                    pred_dfs_postorder(next_pred, preds_fn, marked, order).map_err(|()| {
                        if order.len() == 1 || order.first().unwrap() != order.last().unwrap() {
                            order.push(node_id);
                        }
                    })?;
                }
                order.push(node_id);
                marked.insert(node_id, true);
                Ok(())
            }
        }
    }

    for node_id in node_ids {
        if pred_dfs_postorder(node_id, &mut preds_fn, &mut marked, &mut order).is_err() {
            // Cycle found.
            let end = order.last().unwrap();
            let beg = order.iter().position(|n| n == end).unwrap();
            order.drain(0..=beg);
            return Err(order);
        }
    }

    Ok(order)
}

/// Datastructure for merging subgraphs while maintaining topological sort order.
///
/// Maintains a global topo-sorted Vec of all operators. Each subgraph (merged group)
/// occupies a contiguous range in this Vec. Merging two groups combines their ranges
/// and re-sorts the affected window so groups remain contiguous and correctly ordered.
pub struct SubgraphMerge<K>
where
    K: Key,
{
    /// Predecessor edges in the contracted DAG (per representative).
    subgraph_preds: SecondaryMap<K, Vec<K>>,
    /// All operators in global topo-sort order (fixed length, reshuffled in windows).
    toposort_node: Vec<K>,
    /// Reverse index: node → (start, len).
    /// Non-representatives: len=1, start=their position.
    /// Representatives: start=first operator in group, len=number of operators in group.
    node_toposort: SecondaryMap<K, (usize, usize)>,
    /// Union-find for subgraph membership.
    subgraph_unionfind: crate::union_find::UnionFind<K>,
}

impl<K> SubgraphMerge<K>
where
    K: Key,
{
    pub fn new<PredsIter>(
        keys: impl IntoIterator<Item = K>,
        mut preds_fn: impl FnMut(K) -> PredsIter,
    ) -> Result<Self, Vec<K>>
    where
        PredsIter: IntoIterator<Item = K>,
    {
        let subgraph_preds = keys
            .into_iter()
            .map(|k| (k, (preds_fn)(k).into_iter().collect()))
            .collect::<SecondaryMap<K, Vec<K>>>();
        let toposort_node =
            topo_sort(subgraph_preds.keys(), |k| subgraph_preds[k].iter().copied())?;
        let node_toposort = toposort_node
            .iter()
            .enumerate()
            .map(|(i, &k)| (k, (i, 1)))
            .collect::<SecondaryMap<K, (usize, usize)>>();
        let subgraph_unionfind =
            crate::union_find::UnionFind::with_capacity(toposort_node.len());
        Ok(Self {
            subgraph_preds,
            toposort_node,
            node_toposort,
            subgraph_unionfind,
        })
    }

    pub fn find(&mut self, k: K) -> K {
        self.subgraph_unionfind.find(k)
    }

    pub fn same_set(&mut self, u: K, v: K) -> bool {
        self.subgraph_unionfind.same_set(u, v)
    }

    /// Returns the topo-sorted operators for a given subgraph representative.
    pub fn subgraph_nodes(&mut self, k: K) -> &[K] {
        let rep = self.subgraph_unionfind.find(k);
        let (start, len) = self.node_toposort[rep];
        &self.toposort_node[start..start + len]
    }

    /// Iterates all subgraph representatives with their topo-sorted operator slices.
    pub fn subgraphs(&self) -> impl Iterator<Item = &[K]> {
        self.node_toposort
            .values()
            .filter(|(_, len)| *len > 0)
            .map(|&(start, len)| &self.toposort_node[start..start + len])
    }

    pub fn try_merge(&mut self, u: K, v: K) -> bool {
        let u = self.subgraph_unionfind.find(u);
        let v = self.subgraph_unionfind.find(v);

        if u == v {
            return true;
        }

        // Ensure u is "before" v in topo order.
        let (u, v) = {
            let (u_start, _) = self.node_toposort[u];
            let (v_start, _) = self.node_toposort[v];
            if u_start <= v_start { (u, v) } else { (v, u) }
        };
        let (u_start, _u_len) = self.node_toposort[u];
        let (v_start, v_len) = self.node_toposort[v];
        let window_lo = u_start;
        let window_hi = v_start + v_len - 1;

        // ------------------------------------------------------------
        // 1. Cycle check: can v reach u via predecessor edges?
        // ------------------------------------------------------------
        // Only groups whose range overlaps [window_lo, window_hi] can be on such a path.
        // Direct predecessor edges from v to u become self-loops after
        // merge and are not real cycles, so we skip u as a direct pred.

        let mut visited = std::collections::HashSet::<K>::new();
        visited.insert(v);
        let mut stack = vec![v];

        while let Some(x) = stack.pop() {
            for &p in self.subgraph_preds[x].iter() {
                let root_p = self.subgraph_unionfind.find(p);

                // Direct pred edge from v to u is not a real cycle.
                if root_p == u && x == v {
                    continue;
                }
                if root_p == u {
                    return false;
                }

                let (p_start, p_len) = self.node_toposort[root_p];
                // Prune: group range [p_start, p_start+p_len-1] must overlap [window_lo, window_hi].
                if p_start + p_len > window_lo && p_start <= window_hi && visited.insert(root_p) {
                    stack.push(root_p);
                }
            }
        }

        // ------------------------------------------------------------
        // 2. Perform merge in union-find and rewire predecessors
        // ------------------------------------------------------------

        let new_root = self.subgraph_unionfind.union(u, v);

        let mut preds = Vec::new();
        preds.append(&mut self.subgraph_preds.remove(u).unwrap());
        preds.append(&mut self.subgraph_preds.remove(v).unwrap());
        preds.retain(|x| self.subgraph_unionfind.find(*x) != new_root);
        self.subgraph_preds.insert(new_root, preds);

        // Update the merged group's range temporarily for group identification.
        let subsumed = if new_root == u { v } else { u };
        self.node_toposort[subsumed] = (0, 0);

        // ------------------------------------------------------------
        // 3. Re-sort groups in window [window_lo..=window_hi]
        // ------------------------------------------------------------

        // Identify distinct groups in the window and collect their operator slices.
        // Groups' operators are contiguous within the window (except new_root which
        // has u's and v's operators at separate positions — but both within the window).
        let mut group_order: Vec<K> = Vec::new();
        let mut group_nodes_map: std::collections::HashMap<K, Vec<K>> =
            std::collections::HashMap::new();
        for &node in &self.toposort_node[window_lo..=window_hi] {
            let rep = self.subgraph_unionfind.find(node);
            if !group_nodes_map.contains_key(&rep) {
                group_order.push(rep);
            }
            group_nodes_map.entry(rep).or_default().push(node);
        }

        // Topo-sort groups in the window by their pred edges (filtered to window).
        let group_set: std::collections::HashSet<K> =
            group_order.iter().copied().collect();
        let subgraph_preds = &self.subgraph_preds;
        let subgraph_unionfind = &mut self.subgraph_unionfind;
        let sorted_groups = topo_sort(group_order.iter().copied(), |k| {
            subgraph_preds[k]
                .iter()
                .map(|&p| subgraph_unionfind.find(p))
                .filter(|p| group_set.contains(p))
                .collect::<Vec<_>>()
                .into_iter()
        })
        .expect("cycle check passed but re-toposort found cycle");

        // Rebuild the window: lay out each group's operators in sorted group order.
        let mut buf: Vec<K> = Vec::with_capacity(window_hi - window_lo + 1);
        for &group in &sorted_groups {
            buf.extend_from_slice(&group_nodes_map[&group]);
        }
        self.toposort_node[window_lo..=window_hi].copy_from_slice(&buf);

        // Update reverse index: positions and group ranges.
        let mut pos = window_lo;
        for &group in &sorted_groups {
            let g_len = group_nodes_map[&group].len();
            self.node_toposort[group] = (pos, g_len);
            pos += g_len;
        }

        true
    }
}

#[cfg(test)]
mod test {
    use std::collections::{BTreeMap, BTreeSet};

    use itertools::Itertools;
    use slotmap::SlotMap;

    use super::*;

    #[test]
    pub fn test_toposort() {
        let edges = [
            (5, 11),
            (11, 2),
            (11, 9),
            (11, 10),
            (7, 11),
            (7, 8),
            (8, 9),
            (3, 8),
            (3, 10),
        ];

        // https://commons.wikimedia.org/wiki/File:Directed_acyclic_graph_2.svg
        let sort = topo_sort([2, 3, 5, 7, 8, 9, 10, 11], |v| {
            edges
                .iter()
                .filter(move |&&(_, dst)| v == dst)
                .map(|&(src, _)| src)
        });
        assert!(
            sort.is_ok(),
            "Did not expect cycle: {:?}",
            sort.unwrap_err()
        );

        let sort = sort.unwrap();
        println!("{:?}", sort);

        let position: BTreeMap<_, _> = sort.iter().enumerate().map(|(i, &x)| (x, i)).collect();
        for (src, dst) in edges.iter() {
            assert!(position[src] < position[dst]);
        }
    }

    #[test]
    pub fn test_toposort_cycle() {
        // https://commons.wikimedia.org/wiki/File:Directed_graph,_cyclic.svg
        //          ┌────►C──────┐
        //          │            │
        //          │            ▼
        // A───────►B            E ─────►F
        //          ▲            │
        //          │            │
        //          └─────D◄─────┘
        let edges = [
            ('A', 'B'),
            ('B', 'C'),
            ('C', 'E'),
            ('D', 'B'),
            ('E', 'F'),
            ('E', 'D'),
        ];
        let ids = edges
            .iter()
            .flat_map(|&(a, b)| [a, b])
            .collect::<BTreeSet<_>>();
        let cycle_rotations = BTreeSet::from_iter([
            ['B', 'C', 'E', 'D'],
            ['C', 'E', 'D', 'B'],
            ['E', 'D', 'B', 'C'],
            ['D', 'B', 'C', 'E'],
        ]);

        let permutations = ids.iter().copied().permutations(ids.len());
        for permutation in permutations {
            let result = topo_sort(permutation.iter().copied(), |v| {
                edges
                    .iter()
                    .filter(move |&&(_, dst)| v == dst)
                    .map(|&(src, _)| src)
            });
            assert!(result.is_err());
            let cycle = result.unwrap_err();
            assert!(
                cycle_rotations.contains(&*cycle),
                "cycle: {:?}, vertex order: {:?}",
                cycle,
                permutation
            );
        }
    }

    #[test]
    pub fn test_subgraph_merge_basic() {
        let mut preds = SlotMap::new();

        let a = preds.insert(vec![]);
        let b = preds.insert(vec![]);
        let c = preds.insert(vec![]);
        let d = preds.insert(vec![]);
        let e = preds.insert(vec![]);
        let f = preds.insert(vec![]);

        preds[b].push(a);
        preds[c].push(b);
        preds[d].push(b);
        preds[e].push(c);
        preds[e].push(d);
        preds[f].push(e);

        let mut merge = SubgraphMerge::new(preds.keys(), |v| preds[v].iter().copied()).unwrap();

        assert!(merge.try_merge(a, a)); // No-op.
        //        ┌──► C ──┐
        //        │        ▼
        // A ───► B        E ───► F
        //        │        ▲
        //        └──► D ──┘
        assert!(merge.try_merge(b, c));
        assert!(merge.try_merge(b, c)); // No-op.
        // A ───► BC ────► E ───► F
        //        │        ▲
        //        └──► D ──┘
        assert!(!merge.try_merge(c, e)); // Rejected due to `D` self-edge.

        assert!(merge.try_merge(d, e));
        assert!(merge.try_merge(c, e));
    }
}
