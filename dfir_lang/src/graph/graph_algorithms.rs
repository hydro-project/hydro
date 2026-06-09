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
pub struct SubgraphMerge<K>
where
    K: Key,
{
    subgraph_preds: SecondaryMap<K, Vec<K>>,
    /// Ordered list of subgraph representatives: position → node.
    toposort_subgraph: Vec<K>,
    /// Reverse index: node → position in `toposort_subgraph`.
    subgraph_toposort: SecondaryMap<K, usize>,
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
        let toposort_subgraph =
            topo_sort(subgraph_preds.keys(), |k| subgraph_preds[k].iter().copied())?;
        let subgraph_toposort = toposort_subgraph
            .iter()
            .enumerate()
            .map(|(i, &k)| (k, i))
            .collect::<SecondaryMap<K, usize>>();
        let subgraph_unionfind =
            crate::union_find::UnionFind::with_capacity(toposort_subgraph.len());
        Ok(Self {
            subgraph_preds,
            toposort_subgraph,
            subgraph_toposort,
            subgraph_unionfind,
        })
    }

    pub fn find(&mut self, k: K) -> K {
        self.subgraph_unionfind.find(k)
    }

    pub fn same_set(&mut self, u: K, v: K) -> bool {
        self.subgraph_unionfind.same_set(u, v)
    }

    pub fn try_merge(&mut self, u: K, v: K) -> bool {
        let u = self.subgraph_unionfind.find(u);
        let v = self.subgraph_unionfind.find(v);

        if u == v {
            return true;
        }

        // Ensure u is "before" v in topo order.
        let (u, v) = {
            let pu = self.subgraph_toposort[u];
            let pv = self.subgraph_toposort[v];
            if pu <= pv { (u, v) } else { (v, u) }
        };
        let p_u = self.subgraph_toposort[u];
        let p_v = self.subgraph_toposort[v];

        // ------------------------------------------------------------
        // 1. Cycle check: can v reach u via predecessor edges?
        // ------------------------------------------------------------
        // Only nodes in positions [p_u, p_v] can be on such a path.
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

                let pos = self.subgraph_toposort[root_p];
                if pos >= p_u && pos <= p_v && visited.insert(root_p) {
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

        // ------------------------------------------------------------
        // 3. Re-toposort the range [p_u..=p_v] and splice
        // ------------------------------------------------------------

        // Collect the nodes in the affected range (excluding u and v, replaced by new_root).
        let range_set: std::collections::HashSet<K> = self.toposort_subgraph[p_u..=p_v]
            .iter()
            .copied()
            .filter(|&k| k != u && k != v)
            .chain(std::iter::once(new_root))
            .collect();

        // Topo-sort the range (new_root + other nodes in between).
        // Collect local preds for the range to avoid borrow conflicts.
        let range_preds: Vec<(K, Vec<K>)> = range_set
            .iter()
            .map(|&k| {
                let preds: Vec<K> = self.subgraph_preds[k]
                    .iter()
                    .map(|&p| self.subgraph_unionfind.find(p))
                    .filter(|p| range_set.contains(p))
                    .collect();
                (k, preds)
            })
            .collect();
        let range_preds_map: std::collections::HashMap<K, &Vec<K>> =
            range_preds.iter().map(|(k, v)| (*k, v)).collect();

        let sorted_range = topo_sort(range_set.iter().copied(), |k| {
            range_preds_map[&k].iter().copied()
        })
        .expect("cycle check passed but re-toposort found cycle");

        // Splice the new sorted range into toposort_subgraph.
        self.toposort_subgraph
            .splice(p_u..=p_v, sorted_range.iter().copied());

        // Update reverse index for all affected positions.
        for (i, &k) in self.toposort_subgraph[p_u..].iter().enumerate() {
            self.subgraph_toposort[k] = p_u + i;
        }

        // Remove stale entries for u and v (they're now new_root).
        if new_root != u {
            self.subgraph_toposort.remove(u);
        }
        if new_root != v {
            self.subgraph_toposort.remove(v);
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
