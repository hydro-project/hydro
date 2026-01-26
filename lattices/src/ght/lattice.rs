//! Lattice traits for GHT

use core::cmp::Ordering::{Equal, Greater, Less};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::hash::Hash;

use variadics::variadic_collections::VariadicSet;
use variadics::{CloneVariadic, PartialEqVariadic, SplitBySuffix, VariadicExt, var_expr, var_type};

use crate::ght::{GeneralizedHashTrieNode, GhtGet, GhtInner, GhtLeaf};
use crate::{IsBot, IsTop, LatticeBimorphism, LatticeOrd, Merge};

impl<Head, Node> Merge<GhtInner<Head, Node>> for GhtInner<Head, Node>
where
    Node: GeneralizedHashTrieNode + Merge<Node>,
    Node::Storage: VariadicSet<Schema = Node::Schema>, // multiset is not a lattice!
    Self: GeneralizedHashTrieNode,
    Head: Hash + Eq + Clone,
{
    fn merge(&mut self, other: GhtInner<Head, Node>) -> bool {
        let mut changed = false;

        for (k, v) in other.children {
            match self.children.entry(k) {
                std::collections::hash_map::Entry::Occupied(mut occupied) => {
                    changed |= occupied.get_mut().merge_node(v);
                }
                std::collections::hash_map::Entry::Vacant(vacant) => {
                    vacant.insert(v);
                    changed = true;
                }
            }
        }
        changed
    }
}

impl<Schema, ValType, Storage> Merge<GhtLeaf<Schema, ValType, Storage>>
    for GhtLeaf<Schema, ValType, Storage>
where
    Schema: Eq + Hash,
    Storage: VariadicSet<Schema = Schema> + Extend<Schema> + IntoIterator<Item = Schema>,
{
    fn merge(&mut self, other: GhtLeaf<Schema, ValType, Storage>) -> bool {
        let old_len = self.elements.len();
        self.elements.extend(other.elements);
        self.elements.len() > old_len
    }
}

impl<Head, Node> PartialEq<GhtInner<Head, Node>> for GhtInner<Head, Node>
where
    Head: Hash + Eq + 'static + Clone,
    Node: GeneralizedHashTrieNode + 'static + PartialEq,
    Node::Storage: VariadicSet<Schema = Node::Schema>, // multiset is not a lattice!
    Node::Schema: SplitBySuffix<var_type!(Head, ...Node::SuffixSchema)>,
    GhtInner<Head, Node>: GhtGet,
    <GhtInner<Head, Node> as GhtGet>::Get: PartialEq,
{
    fn eq(&self, other: &GhtInner<Head, Node>) -> bool {
        if self.children.len() != other.children.len() {
            return false;
        }

        for head in self.iter() {
            let other_node = other.get(&head);
            if other_node.is_none() {
                return false;
            }
            let this_node = self.get(&head);
            if this_node.is_none() {
                return false;
            }
            if this_node.unwrap() != other_node.unwrap() {
                return false;
            }
        }
        true
    }
}

impl<Head, Node> PartialOrd<GhtInner<Head, Node>> for GhtInner<Head, Node>
where
    Head: Hash + Eq + 'static + Clone,
    Node: 'static + GeneralizedHashTrieNode + PartialEq + PartialOrd,
    Node::Storage: VariadicSet<Schema = Node::Schema>, // multiset is not a lattice!
    Node::Schema: SplitBySuffix<var_type!(Head, ...Node::SuffixSchema)>,
{
    fn partial_cmp(&self, other: &GhtInner<Head, Node>) -> Option<Ordering> {
        let mut self_any_greater = false;
        let mut other_any_greater = false;
        if self.children.is_empty() && other.children.is_empty() {
            Some(Equal)
        } else {
            #[expect(
                clippy::disallowed_methods,
                reason = "nondeterministic iteration order, TODO(mingwei)"
            )]
            for k in self.children.keys().chain(other.children.keys()) {
                match (self.children.get(k), other.children.get(k)) {
                    (Some(self_value), Some(other_value)) => {
                        match self_value.partial_cmp(other_value)? {
                            Greater => {
                                self_any_greater = true;
                            }
                            Less => {
                                other_any_greater = true;
                            }
                            Equal => {}
                        }
                    }
                    (Some(_), None) => {
                        self_any_greater = true;
                    }
                    (None, Some(_)) => {
                        other_any_greater = true;
                    }
                    (None, None) => unreachable!(),
                }
            }
            match (self_any_greater, other_any_greater) {
                (true, false) => Some(Greater),
                (false, true) => Some(Less),
                (false, false) => Some(Equal),
                (true, true) => unreachable!(),
            }
        }
    }
}

impl<Schema, SuffixSchema, Storage> PartialOrd<GhtLeaf<Schema, SuffixSchema, Storage>>
    for GhtLeaf<Schema, SuffixSchema, Storage>
where
    Schema: Eq + Hash + PartialEqVariadic,
    SuffixSchema: Eq + Hash,
    Storage: VariadicSet<Schema = Schema> + PartialEq,
{
    fn partial_cmp(&self, other: &GhtLeaf<Schema, SuffixSchema, Storage>) -> Option<Ordering> {
        match self.elements.len().cmp(&other.elements.len()) {
            Greater => {
                if other.elements.iter().all(|tup| self.elements.contains(tup)) {
                    Some(Greater)
                } else {
                    None
                }
            }
            Equal => {
                if self
                    .elements
                    .iter()
                    .all(|head| other.elements.contains(head))
                {
                    Some(Equal)
                } else {
                    None
                }
            }
            Less => {
                if self
                    .elements
                    .iter()
                    .all(|head| other.elements.contains(head))
                {
                    Some(Less)
                } else {
                    None
                }
            }
        }
    }
}

impl<Head, Node> LatticeOrd<GhtInner<Head, Node>> for GhtInner<Head, Node>
where
    Self: PartialOrd<GhtInner<Head, Node>>,
    Head: Clone,
    Node: GeneralizedHashTrieNode,
    Node::Storage: VariadicSet<Schema = Node::Schema>, // multiset is not a lattice!
{
}
impl<Schema, SuffixSchema, Storage> LatticeOrd<GhtLeaf<Schema, SuffixSchema, Storage>>
    for GhtLeaf<Schema, SuffixSchema, Storage>
where
    Schema: Eq + Hash + PartialEqVariadic,
    SuffixSchema: Eq + Hash,
    Storage: VariadicSet<Schema = Schema> + PartialEq,
{
}

impl<Head, Node> IsBot for GhtInner<Head, Node>
where
    Head: Clone,
    Node: GeneralizedHashTrieNode + IsBot,
{
    fn is_bot(&self) -> bool {
        #[expect(
            clippy::disallowed_methods,
            reason = "nondeterministic iteration order, TODO(mingwei)"
        )]
        self.children.iter().all(|(_, v)| v.is_bot())
    }
}

impl<Schema, SuffixSchema, Storage> IsBot for GhtLeaf<Schema, SuffixSchema, Storage>
where
    Schema: Eq + Hash,
    SuffixSchema: Eq + Hash,
    Storage: VariadicSet<Schema = Schema>,
{
    fn is_bot(&self) -> bool {
        self.elements.is_empty()
    }
}

impl<Head, Node> IsTop for GhtInner<Head, Node>
where
    Head: Clone,
    Node: GeneralizedHashTrieNode,
    Node::Storage: VariadicSet<Schema = Node::Schema>, // multiset is not a lattice!
{
    fn is_top(&self) -> bool {
        false
    }
}

impl<Schema, SuffixSchema, Storage> IsTop for GhtLeaf<Schema, SuffixSchema, Storage>
where
    Schema: Eq + Hash,
    SuffixSchema: Eq + Hash,
    Storage: VariadicSet<Schema = Schema>,
{
    fn is_top(&self) -> bool {
        false
    }
}

//////////////////////////
// BiMorphisms for GHT
//

/// Bimorphism for the cartesian product of two GHT *subtries*.
///
/// Output is a set of all possible pairs of
/// *suffixes* from the two subtries. If you use this at the root of a GHT, it's a full cross-product.
/// If you use this at an internal node, it provides a 'factorized' representation with only the suffix
/// cross-products expanded.
pub struct GhtCartesianProductBimorphism<GhtOut> {
    _phantom: std::marker::PhantomData<fn() -> GhtOut>,
}
impl<GhtOut> Default for GhtCartesianProductBimorphism<GhtOut> {
    fn default() -> Self {
        Self {
            _phantom: Default::default(),
        }
    }
}
impl<'a, 'b, GhtA, GhtB, GhtOut> LatticeBimorphism<&'a GhtA, &'b GhtB>
    for GhtCartesianProductBimorphism<GhtOut>
where
    GhtA: GeneralizedHashTrieNode,
    GhtA::Storage: VariadicSet<Schema = GhtA::Schema>, // multiset is not a lattice!
    GhtB: GeneralizedHashTrieNode,
    GhtB::Storage: VariadicSet<Schema = GhtB::Schema>, // multiset is not a lattice!
    GhtOut: FromIterator<var_type!(...GhtA::SuffixSchema, ...GhtB::SuffixSchema)>,
    GhtA::SuffixSchema: CloneVariadic,
    GhtB::SuffixSchema: CloneVariadic,
{
    type Output = GhtOut;

    fn call(&mut self, ght_a: &'a GhtA, ght_b: &'b GhtB) -> Self::Output {
        ght_a.recursive_iter().flat_map(|a| {
            let (_a_prefix, a_suffix) = <GhtA::Schema as SplitBySuffix<GhtA::SuffixSchema>>::split_by_suffix_ref(a);
            ght_b
                .recursive_iter()
                .map(move |b| {
                    let (_b_prefix, b_suffix) = <GhtB::Schema as SplitBySuffix<GhtB::SuffixSchema>>::split_by_suffix_ref(b);
                    var_expr!(...<GhtA::SuffixSchema as CloneVariadic>::clone_ref_var(a_suffix), ...<GhtB::SuffixSchema as CloneVariadic>::clone_ref_var(b_suffix))
                })
        }).collect()
    }
}

/// Forms the cartesian product of the ValTypes only
/// Used on GhtLeaf nodes to implement DeepJoinLatticeBimorphism
pub struct GhtValTypeProductBimorphism<GhtOut> {
    _phantom: std::marker::PhantomData<fn() -> GhtOut>,
}
impl<GhtOut> Default for GhtValTypeProductBimorphism<GhtOut> {
    fn default() -> Self {
        Self {
            _phantom: Default::default(),
        }
    }
}
impl<'a, 'b, GhtA, GhtB, GhtOut> LatticeBimorphism<&'a GhtA, &'b GhtB>
    for GhtValTypeProductBimorphism<GhtOut>
where
    GhtA: GeneralizedHashTrieNode,
    GhtA::Storage: VariadicSet<Schema = GhtA::Schema>, // multiset is not a lattice!
    GhtB: GeneralizedHashTrieNode,
    GhtB::Storage: VariadicSet<Schema = GhtB::Schema>, // multiset is not a lattice!
    GhtOut: FromIterator<var_type!(...GhtA::Schema, ...GhtB::ValType)>,
    GhtA::Schema: Eq + Hash + CloneVariadic,
    GhtB::Schema: Eq + Hash + SplitBySuffix<GhtB::ValType>,
    GhtB::ValType: CloneVariadic,
{
    type Output = GhtOut;

    fn call(&mut self, ght_a: &'a GhtA, ght_b: &'b GhtB) -> Self::Output {
        ght_a.recursive_iter().flat_map(|a| {
            ght_b
                .recursive_iter()
                .map(move |b| {
                    let (_prefix_b, suffix_b)
                        = <GhtB::Schema as SplitBySuffix<GhtB::ValType>>::split_by_suffix_ref(b);
                    var_expr!(...<GhtA::Schema as CloneVariadic>::clone_ref_var(a), ...<GhtB::ValType as CloneVariadic>::clone_ref_var(suffix_b))
                }
            )
        }).collect()
    }
}

/// Composable bimorphism, wraps an existing morphism by partitioning it per key.
///
/// For example, `GhtKeyedBimorphism<..., GhtCartesianProduct<...>>` is a join.
#[derive(Default)]
pub struct GhtBimorphism<Bimorphism> {
    bimorphism: Bimorphism,
    // _phantom: std::marker::PhantomData<fn() -> MapOut>,
}
impl<Bimorphism> GhtBimorphism<Bimorphism> {
    /// Create a `KeyedBimorphism` using `bimorphism` for handling values.
    pub fn new(bimorphism: Bimorphism) -> Self {
        Self {
            bimorphism,
            // _phantom: std::marker::PhantomData,
        }
    }
}

impl<GhtA, GhtB, ValFunc, GhtOut> LatticeBimorphism<GhtA, GhtB> for GhtBimorphism<ValFunc>
where
    GhtA: GeneralizedHashTrieNode,
    GhtA::Storage: VariadicSet<Schema = GhtA::Schema>, // multiset is not a lattice!
    GhtB: GeneralizedHashTrieNode,
    GhtB::Storage: VariadicSet<Schema = GhtB::Schema>, // multiset is not a lattice!
    GhtOut: GeneralizedHashTrieNode, // FromIterator<var_type!(...GhtA::Schema, ...GhtB::ValType)>,
    for<'a, 'b> ValFunc: LatticeBimorphism<&'a GhtA, &'b GhtB, Output = GhtOut>,
{
    type Output = GhtOut;

    fn call(&mut self, ght_a: GhtA, ght_b: GhtB) -> Self::Output {
        let node_bim = &mut self.bimorphism; // GhtNodeKeyedBimorphism::<ValFunc>::new(self.bimorphism);
        node_bim.call(&ght_a, &ght_b)
    }
}

#[derive(Default)]
/// bimorphism trait for equijoining Ght Nodes
pub struct GhtNodeKeyedBimorphism<Bimorphism> {
    bimorphism: Bimorphism,
}
/// bimorphism implementation for equijoining Ght Nodes
impl<Bimorphism> GhtNodeKeyedBimorphism<Bimorphism> {
    /// initialize bimorphism
    pub fn new(bimorphism: Bimorphism) -> Self {
        Self { bimorphism }
    }
}
/// bimorphism implementation for equijoining Ght Nodes
impl<'a, 'b, Head, GhtA, GhtB, ValFunc> LatticeBimorphism<&'a GhtA, &'b GhtB>
    for GhtNodeKeyedBimorphism<ValFunc>
where
    Head: Clone + Hash + Eq,
    ValFunc: LatticeBimorphism<&'a GhtA::Get, &'b GhtB::Get>,
    ValFunc::Output: GeneralizedHashTrieNode,
    GhtA: GeneralizedHashTrieNode<Head = Head> + GhtGet,
    GhtB: GeneralizedHashTrieNode<Head = Head, Schema = GhtA::Schema> + GhtGet,
    GhtA::Storage: VariadicSet<Schema = GhtA::Schema>, // multiset is not a lattice!
    GhtB::Storage: VariadicSet<Schema = GhtB::Schema>, // multiset is not a lattice!
    <GhtA::SuffixSchema as VariadicExt>::AsRefVar<'a>: CloneVariadic,
    <GhtB::SuffixSchema as VariadicExt>::AsRefVar<'b>: CloneVariadic,
{
    type Output = GhtInner<Head, ValFunc::Output>; // HashMap<Head, ValFunc::Output>; // GhtOut;

    fn call(&mut self, ght_a: &'a GhtA, ght_b: &'b GhtB) -> Self::Output {
        let mut children = HashMap::<Head, ValFunc::Output>::new();
        // for head in ght_b.iter_keys() {
        for head in ght_b.iter() {
            if let Some(get_a) = ght_a.get(&head) {
                let get_b = ght_b.get(&head).unwrap();
                let val = self.bimorphism.call(get_a, get_b);
                children.insert(head.clone(), val);
            }
        }
        GhtInner { children }
    }
}

/// bimorphism trait for equijoin on full tuple (keys in all GhtInner nodes)
pub trait DeepJoinLatticeBimorphism<Storage> {
    /// bimorphism type for equijoin on full tuple (keys in all GhtInner nodes)
    type DeepJoinLatticeBimorphism;
}
/// bimorphism implementation for equijoin on full tuple (keys in all GhtInner nodes)
impl<Head, NodeA, NodeB, Storage> DeepJoinLatticeBimorphism<Storage>
    for (GhtInner<Head, NodeA>, GhtInner<Head, NodeB>)
where
    Head: 'static + Hash + Eq + Clone,
    NodeA: 'static + GeneralizedHashTrieNode,
    NodeB: 'static + GeneralizedHashTrieNode,
    NodeA::Storage: VariadicSet<Schema = NodeA::Schema>, // multiset is not a lattice!
    NodeB::Storage: VariadicSet<Schema = NodeB::Schema>, // multiset is not a lattice!
    (NodeA, NodeB): DeepJoinLatticeBimorphism<Storage>,
    Storage: VariadicSet<Schema = var_type!(...NodeA::Schema, ...NodeB::ValType)>,
{
    type DeepJoinLatticeBimorphism = GhtNodeKeyedBimorphism<
        <(NodeA, NodeB) as DeepJoinLatticeBimorphism<Storage>>::DeepJoinLatticeBimorphism,
    >;
}
impl<SchemaA, ValTypeA, StorageA, SchemaB, ValTypeB, StorageB, StorageOut>
    DeepJoinLatticeBimorphism<StorageOut>
    for (
        GhtLeaf<SchemaA, ValTypeA, StorageA>,
        GhtLeaf<SchemaB, ValTypeB, StorageB>,
    )
where
    SchemaA: 'static + VariadicExt + Eq + Hash + SplitBySuffix<ValTypeA>, /* + AsRefVariadicPartialEq */
    ValTypeA: 'static + VariadicExt + Eq + Hash, // + AsRefVariadicPartialEq
    SchemaB: 'static + VariadicExt + Eq + Hash + SplitBySuffix<ValTypeB>, /* + AsRefVariadicPartialEq */
    ValTypeB: 'static + VariadicExt + Eq + Hash, // + AsRefVariadicPartialEq
    StorageA: VariadicSet<Schema = SchemaA>,
    StorageB: VariadicSet<Schema = SchemaB>,
    StorageOut: VariadicSet<Schema = var_type!(...SchemaA, ...ValTypeB)>,
    for<'x> SchemaA::AsRefVar<'x>: CloneVariadic,
    for<'x> SchemaB::AsRefVar<'x>: CloneVariadic,
    var_type!(...SchemaA, ...ValTypeB): Eq + Hash,
{
    type DeepJoinLatticeBimorphism = GhtValTypeProductBimorphism<
        GhtLeaf<
            var_type!(...SchemaA, ...ValTypeB),
            var_type!(...ValTypeA, ...ValTypeB),
            StorageOut,
        >,
    >;
}
