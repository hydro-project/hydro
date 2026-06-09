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
    /// (min_order, max_order) for each merged group.
    subgraph_topo_order: SecondaryMap<K, (usize, usize)>,
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
        let topo_order =
            topo_sort(subgraph_preds.keys(), |k| subgraph_preds[k].iter().copied())?;
        let subgraph_topo_order = topo_order
            .iter()
            .enumerate()
            .map(|(i, &k)| (k, (i, i)))
            .collect::<SecondaryMap<K, (usize, usize)>>();
        let subgraph_unionfind =
            crate::union_find::UnionFind::with_capacity(subgraph_topo_order.len());
        Ok(Self {
            subgraph_preds,
            subgraph_topo_order,
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

        // Ensure u is "before" v in topo order (by min_order)
        let (u, v) = {
            let ou = self.subgraph_topo_order[u].0;
            let ov = self.subgraph_topo_order[v].0;
            if ou <= ov { (u, v) } else { (v, u) }
        };

        // ------------------------------------------------------------
        // 1. Cycle check: can v reach u via predecessor edges?
        // ------------------------------------------------------------
        // Pruning: a merged node with max_order < u_min cannot reach u.
        //          a merged node with min_order > v_max is past v.

        let mut visited = std::collections::HashSet::<K>::from_iter(
            self.subgraph_preds[v]
                .iter()
                .map(|&p| self.subgraph_unionfind.find(p)),
        );
        visited.remove(&u);
        let mut stack = visited.iter().copied().collect::<Vec<_>>();
        let u_min_ord = self.subgraph_topo_order[u].0;
        let v_max_ord = self.subgraph_topo_order[v].1;

        while let Some(x) = stack.pop() {
            if x == u {
                return false;
            }

            for &p in self.subgraph_preds[x].iter() {
                let root_p = self.subgraph_unionfind.find(p);

                let (p_min, p_max) = self.subgraph_topo_order[root_p];
                if p_max >= u_min_ord && p_min <= v_max_ord && visited.insert(root_p) {
                    stack.push(root_p);
                }
            }
        }

        // ------------------------------------------------------------
        // 2. Perform merge
        // ------------------------------------------------------------

        let new_root = self.subgraph_unionfind.union(u, v);

        // ------------------------------------------------------------
        // 3. Update topo order (track min and max)
        // ------------------------------------------------------------

        let (u_min, u_max) = self.subgraph_topo_order.remove(u).unwrap();
        let (v_min, v_max) = self.subgraph_topo_order.remove(v).unwrap();
        self.subgraph_topo_order
            .insert(new_root, (u_min.min(v_min), u_max.max(v_max)));

        // ------------------------------------------------------------
        // 4. Rewire predecessor lists
        // ------------------------------------------------------------

        let mut preds = Vec::new();
        preds.append(&mut self.subgraph_preds.remove(u).unwrap());
        preds.append(&mut self.subgraph_preds.remove(v).unwrap());
        preds.retain(|x| self.subgraph_unionfind.find(*x) != new_root);
        self.subgraph_preds.insert(new_root, preds);

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
