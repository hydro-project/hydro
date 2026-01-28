//! COLT from Wang/Willsey/Suciu

use std::hash::Hash;

use variadics::variadic_collections::VariadicCollection;
use variadics::{PartialEqVariadic, SplitBySuffix, VariadicExt, var_expr, var_type};

use crate::ght::{GeneralizedHashTrieNode, GhtGet, GhtInner, GhtLeaf};

/// Data structure design for our COLT is unique.
///
/// In the paper, the COLT is an unbalanced trie that "grows upward" from leaves lazily
/// on access via the `force` method.
/// Unfortunately, unbalanced tries break our types: a node's type to be defined via the
/// type of its children, recursively -- meaning all paths need to be the same type (and length)!
///
/// To work around this, our COLT is a variadic *list* GHTs (a forest) of increasing height,
/// starting with a trie of height 0 and continuing until a trie of height |key| - 1.
/// Our `force` method does not add a node above a leaf L as in the paper. Instead
/// it `take`s L from the current trie and merges it into the next trie to the right which is 1 taller.
//
/// The following trait provides the behavior we need from the nodes in a COLT forest. Every
/// `ColtForestNode` is a `GeneralizedHashTrieNode` with some extra methods.
pub trait ColtForestNode: GeneralizedHashTrieNode {
    /// result of `force`ing a node
    type Force: GeneralizedHashTrieNode;

    /// Force the generation of a parent node, as in the Wang/Willsey/Suciu COLT structure,
    /// to be merged into the next trie to the right.
    fn force(self) -> Option<Self::Force>;

    /// Force the generation of a parent node but retain ref to this node
    fn force_drain(&mut self) -> Option<Self::Force>;
}

// Force only acts on leaves
impl<Head, Node> ColtForestNode for GhtInner<Head, Node>
where
    Head: 'static + Hash + Eq + Clone,
    Node: 'static + ColtForestNode,
    <Node as GeneralizedHashTrieNode>::Schema:
        SplitBySuffix<var_type!(Head,  ...<Node as GeneralizedHashTrieNode>::SuffixSchema)>,
{
    type Force = Node; // where Node:GeneralizedHashTrieNode;
    fn force(self) -> Option<Self::Force> {
        None
    }

    fn force_drain(&mut self) -> Option<Self::Force> {
        None
    }
}

// Leaf case
impl<Schema, Head, Rest, Storage> ColtForestNode
    for GhtLeaf<Schema, var_type!(Head, ...Rest), Storage>
where
    Head: 'static + Clone + Hash + Eq,
    Rest: 'static + Clone + Hash + Eq + VariadicExt,
    Schema: 'static + Hash + Eq + Clone + VariadicExt + PartialEqVariadic,
    Rest: PartialEqVariadic,
    Schema: SplitBySuffix<var_type!(Head, ...Rest)>,
    Schema: SplitBySuffix<Rest>,
    <Schema as SplitBySuffix<(Head, Rest)>>::Prefix: Eq + Hash + Clone,
    <Schema as SplitBySuffix<Rest>>::Prefix: Eq + Hash + Clone,
    Storage: VariadicCollection<Schema = Schema> + Default + IntoIterator<Item = Schema>,
    GhtLeaf<Schema, Rest, Storage>: GeneralizedHashTrieNode<Schema = Schema, Storage = Storage>,
    GhtInner<Head, GhtLeaf<Schema, Rest, Storage>>:
        GeneralizedHashTrieNode<Schema = Schema, Storage = Storage>,
{
    type Force = GhtInner<Head, GhtLeaf<Schema, Rest, Storage>>;
    fn force(mut self) -> Option<Self::Force> {
        let mut retval = Self::Force::default();
        self.forced = true;
        for row in self.into_iter().unwrap() {
            retval.insert(row);
        }
        Some(retval)
    }

    fn force_drain(&mut self) -> Option<GhtInner<Head, GhtLeaf<Schema, Rest, Storage>>> {
        let mut retval = Self::Force::default();
        self.forced = true;
        for row in self.elements.drain() {
            retval.insert(row);
        }
        Some(retval)
    }
}

/// Emulate the `get` and iter` functions for a single Ght node
/// [`GhtGet`] across a forest of ColtForestNodes.
///
/// The "current" ColtGet node (corresponding to the "current" GhtGet node) at depth
/// d from the root is a variadic list of nodes, each at depth d in its their
/// respective trie in the forest, Tries of height d or smaller are omitted,
/// hence the first element in any ColtGet is a GhtLeaf.
pub trait ColtGet {
    /// Schema variadic: the schema of the relation stored in this COLT.
    /// This type is the same in all Tries and nodes of the COLT.
    type Schema: VariadicExt + Eq + Hash + Clone;
    /// The type of Storage
    /// This type is the same in all Tries and nodes of the COLT
    type Storage: VariadicCollection;
    /// SuffixSchema variadic: the suffix of the schema *from this node of the trie
    /// downward*. The first entry in this variadic is of type Head.
    /// This type is the same in all Tries of the COLT (but changes as we traverse downward)
    type SuffixSchema: VariadicExt + Eq + Hash + Clone;
    /// The type of the first column in the SuffixSchema
    /// This type is the same in all Tries of the COLT (but changes as we traverse downward)
    type Head: Eq + Hash;

    /// Type returned by [`Self::get`].
    type Get;

    /// Following the spec in Wang/Willsey/Suciu, on an Inner node this retrieves the value
    /// (child) associated with the given "head" key. It returns an `Option` containing a
    /// reference to the value if found, or `None` if not found.
    /// On a Leaf node, returns None.
    fn get(self, head: &Self::Head) -> Self::Get;

    /// Iterator for the "head" keys (from inner nodes) or nothing (from leaf nodes).
    fn iter(&self) -> impl Iterator<Item = Self::Head>;
}

/// `ColtGet` without the first (head) trie.
pub trait ColtGetTail<InnerToMerge>: ColtGet {
    /// merge an inner node into the head of this tail of the forest
    fn merge(&mut self, inner_to_merge: InnerToMerge);
}

impl<'a, Rest, Schema, SuffixSchema, Storage> ColtGet for var_type!(&'a mut GhtLeaf<Schema, SuffixSchema, Storage>, ...Rest)
where
    Rest: ColtGetTail<
            <GhtLeaf<Schema, SuffixSchema, Storage> as ColtForestNode>::Force,
            Storage = Storage,
        >,
    <Rest as ColtGet>::SuffixSchema: 'a,
    GhtLeaf<Schema, SuffixSchema, Storage>: ColtForestNode,
    Schema: Clone + Hash + Eq + VariadicExt,
    SuffixSchema: Clone + Hash + Eq + VariadicExt,
    Storage: VariadicCollection<Schema = Schema>,
{
    type Schema = Schema;
    type Head = Rest::Head;
    type SuffixSchema = SuffixSchema;
    type Get = Rest::Get;
    type Storage = Rest::Storage;

    fn get(self, head: &Self::Head) -> Self::Get {
        let (first, mut rest) = self;
        let forced = first.force_drain().unwrap();
        ColtGetTail::merge(&mut rest, forced);
        Rest::get(rest, head)
    }

    fn iter(&self) -> impl Iterator<Item = Self::Head> {
        std::iter::empty()
    }
}

// we only merge in GhtInner<Head, GhtLeaf<_>> nodes, so this
// should never be called.
impl<'a, Rest, Schema, SuffixSchema, T, Storage> ColtGetTail<T> for var_type!(&'a mut GhtLeaf<Schema, SuffixSchema, Storage>, ...Rest)
where
    Rest: ColtGetTail<
            <GhtLeaf<Schema, SuffixSchema, Storage> as ColtForestNode>::Force,
            Storage = Storage,
        >,
    <Rest as ColtGet>::SuffixSchema: 'a,
    GhtLeaf<Schema, SuffixSchema, Storage>: ColtForestNode,
    Schema: Clone + Hash + Eq + VariadicExt,
    SuffixSchema: Clone + Hash + Eq + VariadicExt,
    Storage: VariadicCollection<Schema = Schema>,
{
    fn merge(&mut self, _inner_to_merge: T) {
        panic!();
    }
}

impl<'a, Head, Head2, Rest, Node> ColtGet for var_type!(&'a mut GhtInner<Head, GhtInner<Head2, Node>>, ...Rest)
where
    Rest: ColtGet<Head = Head>,
    Head: Eq + Hash + Clone,
    Head2: Eq + Hash + Clone,
    Node: GeneralizedHashTrieNode,
    GhtInner<Head, GhtInner<Head2, Node>>: GeneralizedHashTrieNode<
            Head = Rest::Head,
            SuffixSchema = Rest::SuffixSchema,
            Schema = Rest::Schema,
            Storage = Rest::Storage,
        >,
    GhtInner<Head2, Node>: GeneralizedHashTrieNode<Schema = Rest::Schema, Storage = Rest::Storage>,
{
    type Schema = Rest::Schema;
    type Head = Rest::Head;
    type SuffixSchema = Rest::SuffixSchema;
    type Get = var_type!(&'a mut GhtInner<Head2, Node>, ...Rest::Get);
    type Storage = Rest::Storage;

    fn get(self, head: &Self::Head) -> Self::Get {
        let (first, rest) = self;
        // create a child entry here for this get, to absorb future forces
        // TODO(mingwei): extra clone here if entry already exists.
        let child = first.children.entry(head.clone()).or_default();
        var_expr!(child, ...Rest::get(rest, head))
    }

    fn iter(&self) -> impl Iterator<Item = Self::Head> {
        #[expect(
            clippy::disallowed_methods,
            reason = "nondeterministic iteration order, TODO(mingwei)"
        )]
        self.0.children.keys().cloned().chain(Rest::iter(&self.1))
    }
}

impl<'a, Head, Rest, Schema, ValType, Storage> ColtGet for var_type!(&'a mut GhtInner<Head, GhtLeaf<Schema, ValType, Storage>>, ...Rest)
where
    Rest: ColtGet<Head = Head>,
    Head: Eq + Hash + Clone,
    Schema: Eq + Hash + Clone + PartialEqVariadic,
    ValType: Eq + Hash + Clone + PartialEqVariadic,
    Storage: VariadicCollection<Schema = Schema>,
    GhtLeaf<Schema, ValType, Storage>: GeneralizedHashTrieNode,
    Schema: 'static + Eq + VariadicExt + Hash + Clone + SplitBySuffix<ValType> + PartialEqVariadic,
    <Schema as SplitBySuffix<ValType>>::Prefix: Eq + Hash + Clone,
    GhtInner<Head, GhtLeaf<Schema, ValType, Storage>>:
        GeneralizedHashTrieNode<Head = Head> + GhtGet,
    GhtInner<Head, GhtLeaf<Schema, ValType, Storage>>:
        GeneralizedHashTrieNode<Head = Rest::Head, Schema = Rest::Schema, Storage = Rest::Storage>,
    GhtLeaf<Schema, ValType, Storage>:
        GeneralizedHashTrieNode<Schema = Rest::Schema, Storage = Rest::Storage> + GhtGet,
{
    type Schema = Rest::Schema;
    type Head = Rest::Head;
    type SuffixSchema = Rest::SuffixSchema;
    type Get = var_type!(&'a mut GhtLeaf<Schema, ValType, Storage>, ...Rest::Get);
    type Storage = Rest::Storage;

    fn get(self, head: &Self::Head) -> Self::Get {
        let (first, rest) = self;
        let child = first.children.entry(head.clone()).or_default();
        var_expr!(child, ...Rest::get(rest, head))
    }

    fn iter(&self) -> impl Iterator<Item = Self::Head> {
        #[expect(
            clippy::disallowed_methods,
            reason = "nondeterministic iteration order, TODO(mingwei)"
        )]
        self.0.children.keys().cloned().chain(Rest::iter(&self.1))
    }
}

impl<'a, Head, Rest, Schema, ValType, Storage>
    ColtGetTail<GhtInner<Head, GhtLeaf<Schema, ValType, Storage>>> for var_type!(&'a mut GhtInner<Head, GhtLeaf<Schema, ValType, Storage>>, ...Rest)
where
    Rest: ColtGet<Head = Head, Schema = Schema, Storage = Storage>,
    Head: Eq + Hash + Clone,
    Schema: Eq + Hash + Clone + PartialEqVariadic,
    ValType: Eq + Hash + Clone + PartialEqVariadic,
    Storage: VariadicCollection<Schema = Schema>,
    var_type!(&'a mut GhtInner<Head, GhtLeaf<Schema, ValType, Storage>>, ...Rest):
        ColtGet<Head = Head, Schema = Schema, Storage = Storage>,
    GhtLeaf<Schema, ValType, Storage>: GeneralizedHashTrieNode<Schema = Schema>,
    Schema: 'static + Eq + VariadicExt + Hash + Clone + SplitBySuffix<ValType> + PartialEqVariadic,
    <Schema as SplitBySuffix<ValType>>::Prefix: Eq + Hash + Clone,
    GhtInner<Head, GhtLeaf<Schema, ValType, Storage>>:
        GeneralizedHashTrieNode<Head = Head, Schema = Schema, Storage = Storage> + GhtGet,
{
    fn merge(&mut self, inner_to_merge: GhtInner<Head, GhtLeaf<Schema, ValType, Storage>>) {
        let (head, _rest) = self;
        // can't use Merge with COLT bc columnstore is not a lattice!!
        head.merge_node(inner_to_merge);
    }
}

impl<'a, Head, Node> ColtGet for var_type!(&'a mut GhtInner<Head, Node>)
where
    GhtInner<Head, Node>: GeneralizedHashTrieNode,
    Head: Clone + Eq + Hash,
    Node: GeneralizedHashTrieNode,
{
    type Schema = <GhtInner<Head, Node> as GeneralizedHashTrieNode>::Schema;
    type SuffixSchema = <GhtInner<Head, Node> as GeneralizedHashTrieNode>::SuffixSchema;
    type Head = Head;
    type Get = var_type!(&'a mut Node);
    type Storage = Node::Storage;

    fn get(self, head: &Self::Head) -> Self::Get {
        let child = self.0.children.entry(head.clone()).or_default();
        var_expr!(child)
    }

    fn iter(&self) -> impl Iterator<Item = Self::Head> {
        #[expect(
            clippy::disallowed_methods,
            reason = "nondeterministic iteration order, TODO(mingwei)"
        )]
        self.0.children.keys().cloned()
    }
}
impl<Head, Schema, ValType, Storage> ColtGetTail<GhtInner<Head, GhtLeaf<Schema, ValType, Storage>>> for var_type!(&mut GhtInner<Head, GhtLeaf<Schema, ValType, Storage>>)
where
    GhtInner<Head, GhtLeaf<Schema, ValType, Storage>>:
        GeneralizedHashTrieNode<Head = Head> + GhtGet,
    GhtLeaf<Schema, ValType, Storage>: GeneralizedHashTrieNode<Schema = Schema, Storage = Storage>,
    Head: Clone + Eq + Hash,
    Schema: Clone + Eq + Hash + VariadicExt,
    Storage: VariadicCollection<Schema = Schema>,
{
    fn merge(&mut self, inner_to_merge: GhtInner<Head, GhtLeaf<Schema, ValType, Storage>>) {
        let (head, _rest) = self;
        // can't use Merge with COLT bc columnstore is not a lattice!!
        head.merge_node(inner_to_merge);
    }
}
