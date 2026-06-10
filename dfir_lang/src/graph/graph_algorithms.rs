//! General graph algorithm utility functions

use std::collections::{BTreeSet, HashMap, HashSet};
use std::hash::Hash;

use slotmap::{Key, SecondaryMap, SparseSecondaryMap};

/// Topologically sorts a set of nodes. Returns a list where the order of `Id`s will agree with
/// the order of any path through the graph.
///
/// This succeeds if the input is a directed acyclic graph (DAG).
///
/// If the input has a cycle, an `Err` will be returned containing the cycle. Each node in the
/// cycle will be listed exactly once.
///
/// <https://en.wikipedia.org/wiki/Topological_sorting#Depth-first_search>
pub fn topo_sort<Id, PredsIter>(
    node_ids: impl IntoIterator<Item = Id>,
    mut preds_fn: impl FnMut(Id) -> PredsIter,
) -> Result<Vec<Id>, Vec<Id>>
where
    Id: Copy + Eq + Hash,
    PredsIter: IntoIterator<Item = Id>,
{
    let (mut marked, mut order) = Default::default();

    fn pred_dfs_postorder<Id, PredsIter>(
        node_id: Id,
        preds_fn: &mut impl FnMut(Id) -> PredsIter,
        marked: &mut HashMap<Id, bool>, // `false` => temporary, `true` => permanent.
        order: &mut Vec<Id>,
    ) -> Result<(), ()>
    where
        Id: Copy + Eq + Hash,
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
    /// Predecessor edges in the quotient DAG (per representative).
    subgraph_preds: SecondaryMap<K, Vec<K>>,
    /// All operators in global topo-sort order (fixed length, reshuffled in windows).
    /// Invariant: subgraphs are contiguous & non-overlapping ranges in this vec.
    toposort_node: Vec<K>,
    /// Reverse index: SG representative node -> index (in toposort_node).
    /// Invariant: `K` is both the representative node and the first node in the SG.
    sg_idx: SparseSecondaryMap<K, usize>,
    /// SG representative node -> SG len.
    /// The subgraph's nodes are `toposort_node[index..index+len]`.
    /// Invariant: the subgraph ranges are complete and non-overlapping.
    sg_len: SparseSecondaryMap<K, usize>,

    /// Union-find for subgraph membership.
    subgraph_unionfind: crate::union_find::UnionFind<K>,
}

impl<K> SubgraphMerge<K>
where
    K: Key,
{
    /// Creates a new `SubgraphMerge` from nodes and their predecessor edges.
    /// Returns `Err` with a cycle if the input graph is not a DAG.
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
        let sg_idx = toposort_node
            .iter()
            .enumerate()
            .map(|(i, &k)| (k, i))
            .collect();
        let sg_len = toposort_node.iter().map(|&k| (k, 1)).collect();
        let subgraph_unionfind = crate::union_find::UnionFind::with_capacity(toposort_node.len());
        Ok(Self {
            subgraph_preds,
            toposort_node,
            sg_idx,
            sg_len,
            subgraph_unionfind,
        })
    }

    /// Find the representative of the subgraph containing `k`.
    pub fn find(&mut self, k: K) -> K {
        self.subgraph_unionfind.find(k)
    }

    /// Returns true if `u` and `v` are in the same subgraph.
    pub fn same_set(&mut self, u: K, v: K) -> bool {
        self.subgraph_unionfind.same_set(u, v)
    }

    /// Iterates all subgraph representatives with their topo-sorted operator slices,
    /// in topological order (by position in `toposort_node`).
    pub fn subgraphs(&self) -> impl Iterator<Item = &[K]> {
        let mut i = 0;
        std::iter::from_fn(move || {
            let Some(&sg_node) = self.toposort_node.get(i) else {
                debug_assert_eq!(i, self.toposort_node.len());
                return None;
            };
            debug_assert_eq!(i, self.sg_idx[sg_node]);
            let sg_len = self.sg_len[sg_node];
            let sg_slice = &self.toposort_node[i..i + sg_len];
            i += sg_len;
            Some(sg_slice)
        })
    }

    /// Attempts to merge the subgraphs containing `u` and `v`.
    /// Returns `false` if merging would create a cycle in the subgraph DAG.
    pub fn try_merge(&mut self, u: K, v: K) -> bool {
        // 0. Set up `u` and `v` to be in order, and subgraph representatives.

        // Ensure `u` and `v` are subgraph representatives.
        let u = self.subgraph_unionfind.find(u);
        let v = self.subgraph_unionfind.find(v);
        if u == v {
            // Short circuit no-op case. Guards against weird `u == v` aliasing.
            return true;
        }

        // Ensure `u` is before `v` in topo order.
        let (u, v) = if self.sg_idx[u] < self.sg_idx[v] {
            (u, v)
        } else {
            (v, u)
        };
        // Get the member nodes of `u` and `v`, and the `window`. Pulling references here does ensure that
        // `toposort_node` remains unchanged until we properly merge `u_nodes` and `v_nodes`.
        let (u_nodes, v_nodes, window) = {
            let (u_idx, u_len) = (self.sg_idx[u], self.sg_len[u]);
            let (v_idx, v_len) = (self.sg_idx[v], self.sg_len[v]);
            (
                &self.toposort_node[u_idx..u_idx + u_len],
                &self.toposort_node[v_idx..v_idx + v_len],
                u_idx..v_idx + v_len,
            )
        };

        // 1. Cycle check: can `v` reach `u` via predecessor edges?
        // Only groups within `window` can be on such a path. Direct predecessor edges from `v` to `u` become
        // self-loops after merge and are not real cycles, so we skip direct `u -> v` edges.

        let mut stack = vec![v];
        let mut visited = HashSet::<_>::from_iter([v]);

        while let Some(x) = stack.pop() {
            for &p in self.subgraph_preds[x].iter() {
                let root_p = self.subgraph_unionfind.find(p);

                if root_p == u {
                    if x == v {
                        // Ignore `u -> v` direct edge, not a real cycle.
                        continue;
                    }
                    // Cycle found, return false.
                    return false;
                }

                // Prune: group must be within the `window`.
                if window.contains(&self.sg_idx[root_p]) && visited.insert(root_p) {
                    stack.push(root_p);
                }
            }
        }

        // 2. Perform merge in union-find and append predecessors.
        // `u` will be the new representative.
        {
            // `UnionFind::union` ensures the first arg's representative will represent the new merged group. `u` is before
            // `v` in the topo order, and `u` is already its own representative. This ensures that `u` stays at the *start*
            // of its subgraph group, so the `idx..idx+len` slice is the whole subgraph.
            let _new_root = self.subgraph_unionfind.union(u, v);
            debug_assert_eq!(u, _new_root);
            let v_preds = &mut self.subgraph_preds.remove(v).unwrap();
            let u_preds = &mut self.subgraph_preds[u];
            u_preds.append(v_preds);
            // Update all preds to be representatives (from past unioning). Delete any self-edges.
            u_preds.retain_mut(|x| {
                *x = self.subgraph_unionfind.find(*x);
                *x != u // Retain only non-self edges.
            });
            // Remove any duplicates (may have be created from past unioning).
            u_preds.sort_unstable();
            u_preds.dedup();
        }
        // Remove subsumed `v` and grow `u`'s length.
        {
            self.sg_idx.remove(v).unwrap();
            let v_len = self.sg_len.remove(v).unwrap();
            // Set `u`'s len to the combined size. (Note: `sg_idx[u]` still needs updating, below after re-sort).
            self.sg_len[u] += v_len;
        }

        // 3. Re-sort groups in `window`.
        // Topo-sort groups in the window by their quotient edges.
        {
            let sorted_groups = {
                let reps_in_window = self.toposort_node[window.clone()]
                    .iter()
                    .map(|&k| self.subgraph_unionfind.find(k))
                    .collect::<BTreeSet<_>>();

                // We borrow fields separately to allow the closure to call `find()` (which needs `&mut`) while also reading
                // `subgraph_preds` and `sg_idx` (via `&`).
                // Only predecessor groups whose range overlaps the window are included - groups entirely outside the window
                // have their ordering already satisfied.
                let subgraph_preds = &self.subgraph_preds;
                let subgraph_unionfind = &mut self.subgraph_unionfind;
                let sg_idx = &self.sg_idx;
                topo_sort(reps_in_window, |k| {
                    subgraph_preds[k]
                    .iter()
                    .map(|&p| subgraph_unionfind.find(p))
                    .filter(|&p| window.contains(&sg_idx[p])) // Prune to window.
                    .collect::<Vec<_>>()
                    .into_iter()
                })
                .expect("bug: cycle check passed but re-toposort found cycle")
            };

            // Rebuild the window: lay out each group's operators in sorted group order.
            // All groups except `u` (new root) have contiguous operators at their current range. `u`'s operators will be
            // `u_nodes` *and* `v_nodes`.
            let mut buf = Vec::with_capacity(window.len());
            for &group in &sorted_groups {
                if group == u {
                    buf.extend_from_slice(u_nodes);
                    buf.extend_from_slice(v_nodes);
                } else {
                    let g_idx = self.sg_idx[group];
                    let g_len = self.sg_len[group];
                    buf.extend_from_slice(&self.toposort_node[g_idx..g_idx + g_len]);
                }
            }
            self.toposort_node[window.clone()].copy_from_slice(&buf);

            // Update reverse index `sg_idx` start positions (`sg_len` already correct).
            let mut pos = window.start;
            for &group in &sorted_groups {
                self.sg_idx[group] = pos;
                pos += self.sg_len[group];
            }
            debug_assert_eq!(window.end, pos);
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
        assert!(!merge.try_merge(c, e)); // Rejected due to `D` outside-cycle.

        assert!(merge.try_merge(d, e));
        assert!(merge.try_merge(c, e)); // Now valid since `D` is no longer outside.
    }
}
