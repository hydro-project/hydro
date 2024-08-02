use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;

use sealed::sealed;
use variadics::{var_args, var_expr, var_type, PartialEqVariadic, Split, VariadicExt};

use crate::ght_lattice::DeepJoinLatticeBimorphism;

/// GeneralizedHashTrie wraps up a root GeneralizedHashTrieNode with metadata
/// for the key and value types associated with the full trie.
pub trait GeneralizedHashTrie {
    //+ for<'a> HtPrefixIter<var_type!(&'a Self::Head)> {
    /// Schema variadic: the type of rows we're storing
    type Schema: VariadicExt;

    /// the prefix of the Schema representing the Key type
    type KeyType: VariadicExt;
    /// the last column of the Schema, i.e. the Value type
    type ValType: VariadicExt + Eq + Hash;
    // /// The type of the first column in the Schema
    // type Head: Eq + Hash;
    // /// The type of the Node in the root
    // type Node: GeneralizedHashTrieNode;
    /// The underlying root Trie Node
    type Trie: GeneralizedHashTrieNode;

    /// Create a new Ght from the iterator.
    fn new_from(input: impl IntoIterator<Item = Self::Schema>) -> Self;

    /// Report the height of the tree if its not empty. This is the length of a root to leaf path -1.
    /// E.g. if we have GhtInner<GhtInner<GhtLeaf...>> the height is 2
    fn height(&self) -> Option<usize>;

    /// Inserts items into the hash trie.
    fn insert(&mut self, row: Self::Schema) -> bool;

    /// Returns `true` if the (entire) row is found in the trie, `false` otherwise.
    fn contains<'a>(&'a self, row: <Self::Schema as VariadicExt>::AsRefVar<'a>) -> bool;

    // /// walk to the leaf from any inner node
    // fn walk_to_leaf<Node, Head, Rest>(
    //     node: Node,
    //     search_key: var_type!(Head, ...Rest),
    // ) -> Option<GhtLeaf<Self::ValType>>
    // where
    //     Node: GeneralizedHashTrieNode + GhtHasChildren,
    //     Rest: VariadicExt;

    /// Iterate through the (entire) rows stored in this HashTrie.
    /// Returns Variadics, not tuples.
    fn recursive_iter(&self) -> impl Iterator<Item = <Self::Schema as VariadicExt>::AsRefVar<'_>>;

    /// Iterate through the (entire) rows stored in this HashTrie, but with the leaf
    /// values stubbed out ((), ()).
    fn recursive_iter_keys(
        &self,
    ) -> impl Iterator<Item = <Self::Schema as VariadicExt>::AsRefVar<'_>>;

    // /// Extract Key and Value from a by-value tuple
    // fn split_key_val_no_refs(tup: Self::Schema) -> (Self::KeyType, Self::ValType)
    // where
    //     Self::Schema: Split<Self::KeyType, Suffix = Self::ValType>,
    // {
    //     tup.split()
    // }

    /// Extract Key and Value from a returned tuple
    fn split_key_val<'a>(
        tup: <Self::Schema as VariadicExt>::AsRefVar<'a>,
    ) -> (
        <Self::KeyType as VariadicExt>::AsRefVar<'a>,
        <Self::ValType as VariadicExt>::AsRefVar<'a>,
    )
    where
        <Self::Schema as VariadicExt>::AsRefVar<'a>: Split<
            <Self::KeyType as VariadicExt>::AsRefVar<'a>,
            Suffix = <Self::ValType as VariadicExt>::AsRefVar<'a>,
        >,
    {
        tup.split()
    }

    /// get a ref to the underlying root note of the trie
    fn get_trie(&self) -> &Self::Trie;

    /// get a mutable ref to the underlying root note of the trie
    fn get_mut_trie(&mut self) -> &mut Self::Trie;
}

/// GeneralizedHashTrie is a metadata node pointing to a root GeneralizedHashTrieNode.
#[derive(Debug, Clone)]
pub struct GHT<KeyType, ValType, TrieRoot>
where
    KeyType: VariadicExt, // + AsRefVariadicPartialEq
    ValType: VariadicExt, // + AsRefVariadicPartialEq
    TrieRoot: GeneralizedHashTrieNode,
{
    pub(crate) trie: TrieRoot,
    pub(crate) _key: std::marker::PhantomData<KeyType>,
    pub(crate) _val: std::marker::PhantomData<ValType>,
}

impl<K, V, TrieRoot> GHT<K, V, TrieRoot>
where
    K: VariadicExt, // + AsRefVariadicPartialEq
    V: VariadicExt, // + AsRefVariadicPartialEq
    TrieRoot: GeneralizedHashTrieNode,
{
    /// Just calls `prefix_iter` on the underlying trie.
    pub fn prefix_iter<'a, Prefix>(
        &'a self,
        prefix: Prefix,
    ) -> impl Iterator<Item = <TrieRoot::Suffix as VariadicExt>::AsRefVar<'a>>
    where
        TrieRoot: HtPrefixIter<Prefix>,
        Prefix: 'a,
    {
        self.trie.prefix_iter(prefix)
    }
}

impl<K, V, TrieRoot> GeneralizedHashTrie for GHT<K, V, TrieRoot>
where
    K: VariadicExt,             // + AsRefVariadicPartialEq
    V: VariadicExt + Hash + Eq, // + AsRefVariadicPartialEq
    TrieRoot: GeneralizedHashTrieNode,
{
    type KeyType = K;
    type ValType = V;
    type Schema = TrieRoot::Schema;
    type Trie = TrieRoot;

    fn new_from(input: impl IntoIterator<Item = Self::Schema>) -> Self {
        let trie = GeneralizedHashTrieNode::new_from(input);
        GHT {
            trie,
            _key: Default::default(),
            _val: Default::default(),
        }
    }

    fn height(&self) -> Option<usize> {
        self.trie.height()
    }

    fn insert(&mut self, row: Self::Schema) -> bool {
        self.trie.insert(row)
    }

    fn contains<'a>(&'a self, row: <Self::Schema as VariadicExt>::AsRefVar<'a>) -> bool {
        self.trie.contains(row)
    }

    /// Iterate through the (entire) rows stored in this HashTrie.
    /// Returns Variadics, not tuples.
    fn recursive_iter(&self) -> impl Iterator<Item = <Self::Schema as VariadicExt>::AsRefVar<'_>> {
        self.trie.recursive_iter()
    }

    /// Iterate through the (entire) rows stored in this HashTrie, but with the leaf
    /// values stubbed out ((), ()).
    fn recursive_iter_keys(
        &self,
    ) -> impl Iterator<Item = <Self::Schema as VariadicExt>::AsRefVar<'_>> {
        self.trie.recursive_iter_keys()
    }

    fn get_trie(&self) -> &TrieRoot {
        &self.trie
    }

    /// get a mutable ref to the underlying root note of the trie
    fn get_mut_trie(&mut self) -> &mut Self::Trie {
        &mut self.trie
    }
}

impl<K, V, TrieRoot> Default for GHT<K, V, TrieRoot>
where
    K: VariadicExt, // + AsRefVariadicPartialEq
    V: VariadicExt, // + AsRefVariadicPartialEq
    TrieRoot: GeneralizedHashTrieNode,
{
    fn default() -> Self {
        let tree = TrieRoot::default();
        let _key: std::marker::PhantomData<K> = Default::default();
        let _val: std::marker::PhantomData<V> = Default::default();
        Self {
            trie: tree,
            _key,
            _val,
        }
    }
}

/// GeneralizedHashTrieNode trait
#[sealed]
pub trait GeneralizedHashTrieNode: Default // + for<'a> HtPrefixIter<var_type!(&'a Self::Head)>
{
    /// Schema variadic: the type of rows we're storing in this subtrie
    type Schema: VariadicExt;
    /// The type of the first column in the Schema
    type Head: Eq + Hash;
    /// The type of the leaves
    // type Leaf: Eq + Hash;

    /// Create a new Ght from the iterator.
    fn new_from(input: impl IntoIterator<Item = Self::Schema>) -> Self;

    /// Report the height of the tree if its not empty. This is the length of a root to leaf path -1.
    /// E.g. if we have GhtInner<GhtInner<GhtLeaf...>> the height is 2
    fn height(&self) -> Option<usize>;

    /// report whether node is a leaf node; else an inner node
    fn is_leaf(&self) -> bool;

    /// Inserts items into the hash trie.
    fn insert(&mut self, row: Self::Schema) -> bool;

    /// Returns `true` if the (entire) row is found in the trie, `false` otherwise.
    /// See `get()` below to look just for "head" keys in this node
    fn contains<'a>(&'a self, row: <Self::Schema as VariadicExt>::AsRefVar<'a>) -> bool;

    /// Iterator for the "head" keys (from inner nodes) or elements (from leaf nodes).
    fn iter(&self) -> impl Iterator<Item = &'_ Self::Head>;

    /// Type returned by [`Self::get`].
    type Get: GeneralizedHashTrieNode;

    /// On an Inner node, retrieves the value (child) associated with the given "head" key.
    /// returns an `Option` containing a reference to the value if found, or `None` if not found.
    /// On a Leaf node, returns None.
    fn get(&self, head: &Self::Head) -> Option<&'_ Self::Get>;

    /// Iterate through the (entire) rows stored in this HashTrie.
    /// Returns Variadics, not tuples.
    fn recursive_iter(&self) -> impl Iterator<Item = <Self::Schema as VariadicExt>::AsRefVar<'_>>;

    /// Iterate through the (entire) rows stored in this HashTrie, but with the leaf
    /// values stubbed out ((), ()).
    fn recursive_iter_keys(
        &self,
    ) -> impl Iterator<Item = <Self::Schema as VariadicExt>::AsRefVar<'_>>;

    /// Bimorphism for joining on full tuple keys (all GhtInner keys) in the trie
    type DeepJoin<Other>
    where
        Other: GeneralizedHashTrieNode,
        (Self, Other): DeepJoinLatticeBimorphism;

    // /// For Inner nodes only, this is the type of the Child node
    // type ChildNode: GeneralizedHashTrieNode;

    // /// Cast as Some<&GhtInner> if this is an inner node, else return None
    // fn cast_as_inner(&self) -> Option<&Self>;

    // /// Cast as Some<&mut GhtInner> if this is an inner node, else return None
    // fn cast_as_inner_mut(&mut self) -> Option<&mut Self>;
}

/// A trait for internal nodes of a GHT
pub trait GhtHasChildren: GeneralizedHashTrieNode {
    /// The child node's type
    type Node: GeneralizedHashTrieNode;
    /// return the hash map of children, mutable
    fn children(&mut self) -> &mut HashMap<Self::Head, Self::Node>;
}

/// internal node of a HashTrie
#[derive(Debug, Clone)]
pub struct GhtInner<Head, Node>
where
    Node: GeneralizedHashTrieNode,
{
    pub(crate) children: HashMap<Head, Node>,
    // pub(crate) _leaf: std::marker::PhantomData<Leaf>,
}
impl<Head, Node: GeneralizedHashTrieNode> Default for GhtInner<Head, Node>
where
    Node: GeneralizedHashTrieNode,
{
    fn default() -> Self {
        let children = Default::default();
        Self {
            children,
            // _leaf: Default::default(),
        }
    }
}
#[sealed]
impl<Head, Node> GeneralizedHashTrieNode for GhtInner<Head, Node>
where
    Head: 'static + Hash + Eq,
    Node: 'static + GeneralizedHashTrieNode,
{
    type Schema = var_type!(Head, ...Node::Schema);
    type Head = Head;

    fn new_from(input: impl IntoIterator<Item = Self::Schema>) -> Self {
        let mut retval: Self = Default::default();
        for i in input {
            retval.insert(i);
        }
        retval
    }

    fn height(&self) -> Option<usize> {
        if let Some((_k, v)) = self.children.iter().next() {
            Some(v.height().unwrap() + 1)
        } else {
            None
        }
    }

    fn is_leaf(&self) -> bool {
        false
    }

    fn insert(&mut self, row: Self::Schema) -> bool {
        let var_args!(head, ...rest) = row;
        self.children.entry(head).or_default().insert(rest)
    }

    fn contains<'a>(&'a self, row: <Self::Schema as VariadicExt>::AsRefVar<'a>) -> bool {
        let var_args!(head, ...rest) = row;
        if let Some(node) = self.children.get(head) {
            node.contains(rest)
        } else {
            false
        }
    }

    fn iter(&self) -> impl Iterator<Item = &'_ Self::Head> {
        self.children.keys()
    }

    type Get = Node;
    fn get(&self, head: &Self::Head) -> Option<&'_ Self::Get> {
        self.children.get(head)
    }

    fn recursive_iter(&self) -> impl Iterator<Item = <Self::Schema as VariadicExt>::AsRefVar<'_>> {
        self.children
            .iter()
            .flat_map(|(k, vs)| vs.recursive_iter().map(move |v| var_expr!(k, ...v)))
    }

    fn recursive_iter_keys(
        &self,
    ) -> impl Iterator<Item = <Self::Schema as VariadicExt>::AsRefVar<'_>> {
        self.children
            .iter()
            .flat_map(|(k, vs)| vs.recursive_iter_keys().map(move |v| var_expr!(k, ...v)))
    }

    type DeepJoin<Other> = <(Self, Other) as DeepJoinLatticeBimorphism>::DeepJoinLatticeBimorphism
    where
        Other: GeneralizedHashTrieNode,
        (Self, Other): DeepJoinLatticeBimorphism;

    // type ChildNode = Node;
    // fn cast_as_inner(&self) -> Option<&Self> {
    //     Some(self)
    // }
    // fn cast_as_inner_mut(&mut self) -> Option<&mut Self> {
    //     Some(self)
    // }
}
impl<Head, Node> FromIterator<var_type!(Head, ...Node::Schema)> for GhtInner<Head, Node>
where
    Head: 'static + Hash + Eq,
    Node: 'static + GeneralizedHashTrieNode,
{
    fn from_iter<Iter: IntoIterator<Item = var_type!(Head, ...Node::Schema)>>(iter: Iter) -> Self {
        let mut out = Self::default();
        for row in iter {
            out.insert(row);
        }
        out
    }
}

impl<Head, Node> GhtHasChildren for GhtInner<Head, Node>
where
    Head: 'static + Hash + Eq,
    Node: 'static + GeneralizedHashTrieNode,
{
    type Node = Node;
    fn children(&mut self) -> &mut HashMap<Head, Node> {
        &mut self.children
    }
}

/// leaf node of a HashTrie
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct GhtLeaf<T>
where
    T: Eq + Hash,
{
    pub(crate) elements: HashSet<T>,
}
impl<T> Default for GhtLeaf<T>
where
    T: Eq + Hash,
{
    fn default() -> Self {
        let elements = Default::default();
        Self { elements }
    }
}
#[sealed]
impl<T> GeneralizedHashTrieNode for GhtLeaf<T>
where
    T: 'static + Eq + VariadicExt + Hash,
    for<'a> T::AsRefVar<'a>: PartialEq,
{
    type Schema = T;
    type Head = T;

    fn new_from(input: impl IntoIterator<Item = Self::Schema>) -> Self {
        let mut retval: Self = Default::default();
        for i in input {
            retval.insert(i);
        }
        retval
    }

    fn height(&self) -> Option<usize> {
        Some(0)
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn insert(&mut self, row: Self::Schema) -> bool {
        self.elements.insert(row)
    }

    fn contains<'a>(&'a self, row: <Self::Schema as VariadicExt>::AsRefVar<'a>) -> bool {
        self.elements.iter().any(|r| r.as_ref_var() == row)
    }

    fn iter(&self) -> impl Iterator<Item = &'_ Self::Head> {
        self.elements.iter()
    }

    type Get = Self;
    fn get(&self, _head: &Self::Head) -> Option<&'_ Self::Get> {
        Option::<&Self>::None
    }

    fn recursive_iter(&self) -> impl Iterator<Item = <Self::Schema as VariadicExt>::AsRefVar<'_>> {
        self.elements.iter().map(T::as_ref_var)
    }

    fn recursive_iter_keys(
        &self,
    ) -> impl Iterator<Item = <Self::Schema as VariadicExt>::AsRefVar<'_>> {
        let out = self.elements.iter().map(T::as_ref_var).next().unwrap();
        std::iter::once(out)
    }

    type DeepJoin<Other> = <(Self, Other) as DeepJoinLatticeBimorphism>::DeepJoinLatticeBimorphism
    where
        Other: GeneralizedHashTrieNode,
        (Self, Other): DeepJoinLatticeBimorphism;
}

impl<T> FromIterator<T> for GhtLeaf<T>
where
    T: Eq + Hash,
{
    fn from_iter<Iter: IntoIterator<Item = T>>(iter: Iter) -> Self {
        let elements = iter.into_iter().collect();
        Self { elements }
    }
}

#[sealed]
/// iterators for HashTries based on a prefix search
pub trait FindLeaf<Schema, LeafType> {
    /// type of the suffix of this prefix
    type Suffix: VariadicExt;
    /// given a prefix, return an iterator through the items below
    fn find_containing_leaf<'a>(&'a self, row: Schema) -> Option<&'a LeafType>
    where
        Self::Suffix: 'a,
        LeafType: GeneralizedHashTrieNode;
}

#[sealed]
impl<KeyType, ValType, TrieRoot, Schema> FindLeaf<Schema, GhtLeaf<ValType>>
    for GHT<KeyType, ValType, TrieRoot>
where
    TrieRoot: FindLeaf<Schema, GhtLeaf<ValType>>,
    KeyType: VariadicExt,
    ValType: 'static + VariadicExt + Eq + Hash + PartialEqVariadic,
    TrieRoot: GeneralizedHashTrieNode,
    Schema: Eq + Hash + VariadicExt + PartialEqVariadic,
    for<'a> ValType::AsRefVar<'a>: PartialEq,
{
    // type Suffix = <TrieRoot as HtPrefixIter<Prefix>>::Suffix;
    type Suffix = <TrieRoot as FindLeaf<Schema, GhtLeaf<ValType>>>::Suffix;

    fn find_containing_leaf<'a>(&'a self, row: Schema) -> Option<&'a GhtLeaf<ValType>>
    where
        Self::Suffix: 'a,
    {
        self.trie.find_containing_leaf(row)
    }
}

#[sealed]
impl<'k, Head, Node, SchemaRest, LeafType> FindLeaf<var_type!(&'k Head, ...SchemaRest), LeafType>
    for GhtInner<Head, Node>
where
    Head: Eq + Hash,
    Node: GeneralizedHashTrieNode + FindLeaf<SchemaRest, LeafType>,
    SchemaRest: Eq + Hash + VariadicExt + PartialEqVariadic,
{
    type Suffix = <Node as FindLeaf<SchemaRest, LeafType>>::Suffix;
    fn find_containing_leaf<'a>(
        &'a self,
        row: var_type!(&'k Head, ...SchemaRest),
    ) -> Option<&'a LeafType>
    where
        Self::Suffix: 'a,
        LeafType: GeneralizedHashTrieNode,
    {
        let var_args!(head, ...rest) = row;
        self.children
            .get(head)
            .and_then(|child| child.find_containing_leaf(rest))
    }
}

// #[sealed]
// impl<'k, Head, PrefixRest> HtPrefixIter<var_type!(&'k Head, ...PrefixRest::AsRefVar<'k>)>
//     for GhtLeaf<var_type!(Head, ...PrefixRest)>
// where
//     Head: Eq + Hash,
//     PrefixRest: Eq + Hash + VariadicExt + PartialEqVariadic,

/// This case only splits HEAD and REST in order to prevent a conflict with the `HtPrefixIter<var_type!()>` impl.
/// If not for that, we could just use a single variadic type parameter.
#[sealed]
impl<'k, Head> FindLeaf<Head::AsRefVar<'k>, GhtLeaf<Head>> for GhtLeaf<Head>
where
    Head: Eq + Hash + VariadicExt + PartialEqVariadic,
    // Head: Eq + Hash + RefVariadic, /* TODO(mingwei): `Hash` actually use hash set contains instead of iterate. */
    // Head::UnRefVar: 'static + Eq + Hash,
    // for<'a> PrefixRest::AsRefVar<'a>: PartialEq<PrefixRest::AsRefVar<'a>>,
    // Head::UnRefVar: for<'a> VariadicExt<AsRefVar<'a> = Head>,
{
    type Suffix = var_expr!();
    fn find_containing_leaf<'a>(&'a self, row: Head::AsRefVar<'k>) -> Option<&'a GhtLeaf<Head>>
    where
        Self::Suffix: 'a,
    {
        // TODO(mingwei): actually use the hash set as a hash set
        if self
            .elements
            .iter()
            .any(|x| Head::eq_ref(row, x.as_ref_var()))
        {
            Some(self)
        } else {
            None
        }
        // let var_args!(head) = prefix;
        // self.elements.contains(head).then_some(()).into_iter()
    }
}

#[sealed]
/// iterators for HashTries based on a prefix search
pub trait HtPrefixIter<Prefix> {
    /// type of the suffix of this prefix
    type Suffix: VariadicExt;
    /// given a prefix, return an iterator through the items below
    fn prefix_iter<'a>(
        &'a self,
        prefix: Prefix,
    ) -> impl Iterator<Item = <Self::Suffix as VariadicExt>::AsRefVar<'a>>
    where
        Self::Suffix: 'a;
}

#[sealed]
impl<Prefix, KeyType, ValType, TrieRoot> HtPrefixIter<Prefix> for GHT<KeyType, ValType, TrieRoot>
where
    TrieRoot: HtPrefixIter<Prefix>,
    KeyType: VariadicExt,
    ValType: VariadicExt,
    TrieRoot: GeneralizedHashTrieNode,
    // Head: Hash + Eq,
    // KeyType: VariadicExt,
    // ValType: VariadicExt,
    // TrieRoot: GeneralizedHashTrieNode + GhtHasChildren + HtPrefixIter<PrefixRest>,
    // PrefixRest: Copy,
    // <TrieRoot as GhtHasChildren>::Node: HtPrefixIter<PrefixRest>,
{
    type Suffix = <TrieRoot as HtPrefixIter<Prefix>>::Suffix;

    fn prefix_iter<'a>(
        &'a self,
        prefix: Prefix,
    ) -> impl Iterator<Item = <Self::Suffix as VariadicExt>::AsRefVar<'a>>
    where
        Self::Suffix: 'a,
    {
        self.trie.prefix_iter(prefix)
    }
}

#[sealed]
impl<'k, Head, Node, PrefixRest> HtPrefixIter<var_type!(&'k Head, ...PrefixRest)>
    for GhtInner<Head, Node>
where
    Head: Eq + Hash,
    Node: GeneralizedHashTrieNode + HtPrefixIter<PrefixRest>,
    PrefixRest: Copy,
{
    type Suffix = <Node as HtPrefixIter<PrefixRest>>::Suffix;
    fn prefix_iter<'a>(
        &'a self,
        prefix: var_type!(&'k Head, ...PrefixRest),
    ) -> impl Iterator<Item = <Self::Suffix as VariadicExt>::AsRefVar<'a>>
    where
        Self::Suffix: 'a,
    {
        let var_args!(head, ...rest) = prefix;
        self.children
            .get(head)
            .map(|node| node.prefix_iter(rest))
            .into_iter()
            .flatten()
    }
}
// #[sealed]
// impl<Head, Node> HtPrefixIter<var_type!()> for GhtInner<Head, Node>
// where
//     Head: 'static + Eq + Hash,
//     Node: 'static + GeneralizedHashTrieNode,
// {
//     type Suffix = <Self as GeneralizedHashTrieNode>::Schema;
//     fn prefix_iter<'a>(
//         &'a self,
//         _prefix: var_type!(),
//     ) -> impl Iterator<Item = <Self::Suffix as VariadicExt>::AsRefVar<'a>>
//     where
//         Self::Suffix: 'a,
//     {
//         self.recursive_iter()
//     }
// }

/// This case only splits HEAD and REST in order to prevent a conflict with the `HtPrefixIter<var_type!()>` impl.
/// If not for that, we could just use a single variadic type parameter.
#[sealed]
impl<'k, Head, PrefixRest> HtPrefixIter<var_type!(&'k Head, ...PrefixRest::AsRefVar<'k>)>
    for GhtLeaf<var_type!(Head, ...PrefixRest)>
where
    Head: Eq + Hash,
    PrefixRest: Eq + Hash + VariadicExt + PartialEqVariadic,
    // Head: Eq + Hash + RefVariadic, /* TODO(mingwei): `Hash` actually use hash set contains instead of iterate. */
    // Head::UnRefVar: 'static + Eq + Hash,
    // for<'a> PrefixRest::AsRefVar<'a>: PartialEq<PrefixRest::AsRefVar<'a>>,
    // Head::UnRefVar: for<'a> VariadicExt<AsRefVar<'a> = Head>,
{
    type Suffix = var_expr!();
    fn prefix_iter<'a>(
        &'a self,
        prefix: var_type!(&'k Head, ...PrefixRest::AsRefVar<'k>),
    ) -> impl Iterator<Item = <Self::Suffix as VariadicExt>::AsRefVar<'a>>
    where
        Self::Suffix: 'a,
    {
        // TODO(mingwei): actually use the hash set as a hash set
        let var_args!(prefix_head, ...prefix_rest) = prefix;
        self.elements
            .iter()
            .any(|var_args!(item_head, ...item_rest)| {
                prefix_head == item_head && PrefixRest::eq_ref(prefix_rest, item_rest.as_ref_var())
            })
            .then_some(())
            .into_iter()
        // let var_args!(head) = prefix;
        // self.elements.contains(head).then_some(()).into_iter()
    }
}
#[sealed]
impl<This> HtPrefixIter<var_type!()> for This
where
    This: 'static + GeneralizedHashTrieNode,
{
    type Suffix = <Self as GeneralizedHashTrieNode>::Schema;
    fn prefix_iter<'a>(
        &'a self,
        _prefix: var_type!(),
    ) -> impl Iterator<Item = <Self::Suffix as VariadicExt>::AsRefVar<'a>>
    where
        Self::Suffix: 'a,
    {
        self.recursive_iter()
    }
}

// trait ContainsKey {
//     fn contains_key(&self) -> bool;
// }

// impl<Head, Node> ContainsKey for GhtInner<Head, Node>
// where
//     Head: 'static + Hash + Eq,
//     Node: 'static + GeneralizedHashTrieNode,
// {
//     /// Returns `true` if the key is found in the inner nodes of the trie, `false` otherwise.
//     fn contains_key(&self, key: <Self::KeyType as VariadicExt>::AsRefVar<'_>) -> bool {
//         true
//     }
// }

/// Macro to construct a Ght node type from the constituent key and
/// dependent column types. You pass it:
///    - a list of key column types and dependent column type separated by a fat arrow,
///         a la (K1, K2, K3 => T1, T2, T3)
///
/// This macro generates a hierarchy of GHT node types where each key column is associated with an GhtInner
/// of the associated column type, and the remaining dependent columns are associated with a variadic HTleaf
/// a la var_expr!(T1, T2, T3)
#[macro_export]
macro_rules! GhtNodeType {
    // Empty key base case.
    (() => $( $z:ty ),*) => (
        $crate::ght::GhtLeaf::<$crate::variadics::var_type!($( $z ),* )>
    );
    // Singleton key base case.
    ($a:ty => $( $z:ty ),*) => (
        $crate::ght::GhtInner::<$a, $crate::ght::GhtLeaf::<$crate::variadics::var_type!($( $z ),*)>>
    );
    // Recursive case.
    ($a:ty, $( $b:ty ),* => $( $z:ty ),*) => (
        $crate::ght::GhtInner::<$a, $crate::GhtNodeType!($( $b ),* => $( $z ),*)>
    );
}

/// Macro to create a GHT with the appropriate KeyType, ValType, Schema,
/// and a pointer to the GhtInner at the root of the Trie
#[macro_export]
macro_rules! GhtType {
    ($a:ty => $( $z:ty ),* ) => (
        $crate::ght::GHT::<$crate::variadics::var_type!($a), $crate::variadics::var_type!($( $z ),*), $crate::GhtNodeType!($a => $( $z ),*)>
    );
    ($a:ty, $( $b:ty ),+  => $( $z:ty ),* ) => (
        $crate::ght::GHT::<$crate::variadics::var_type!( $a, $( $b ),+ ), $crate::variadics::var_type!($( $z ),*), $crate::GhtNodeType!($a, $( $b ),+ => $( $z ),*)>
    );
}
