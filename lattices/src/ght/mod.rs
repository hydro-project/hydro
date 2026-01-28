//! GHT from the Wang/Willsey/Suciu Freejoin work
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

use variadics::variadic_collections::VariadicCollection;
use variadics::{
    PartialEqVariadic, RefVariadic, Split, SplitBySuffix, VariadicExt, var_args, var_type,
};

pub mod colt;
pub mod lattice;
pub mod macros;
pub mod test;

/// The GeneralizedHashTrieNode trait captures the properties of nodes in a Ght.
///
/// The Ght, defined by Wang/Willsey/Suciu, is a hash-based trie for storing tuples.
/// It is parameterized by an ordered schema [`VariadicExt`] of the relation stored in the trie.
/// It is a tree of [`GhtInner`] nodes, with [`GhtLeaf`] nodes at the leaves.
/// The trie is keyed on a prefix of the schema [`Self::KeyType`],
/// and the remaining columns [`Self::ValType`] are stored in the leaf.
/// All leaf nodes use the same `[Self::Storage]` type to store the data.
pub trait GeneralizedHashTrieNode: Default {
    // types that are the same in all nodes of the trie
    /// Schema variadic: the schema of the relation stored in this trie.
    type Schema: VariadicExt + Eq + Hash + Clone + SplitBySuffix<Self::SuffixSchema>;
    /// The prefix of columns in [`Self::Schema`] that the trie is keyed on
    type KeyType: VariadicExt + Eq + Hash + Clone;
    /// The suffix of columns in [`Self::Schema`] that are not part of the trie keys
    type ValType: VariadicExt + Eq + Hash + Clone;
    /// The type that holds the data in the leaves
    type Storage: VariadicCollection<Schema = Self::Schema>
        + Default
        + IntoIterator<Item = Self::Schema>;

    // types that vary per node
    /// SuffixSchema variadic: the suffix of [`Self::Schema`] from this node of the trie
    /// downward. The first entry in this variadic is of type [`Self::Head`].
    type SuffixSchema: VariadicExt + Eq + Hash + Clone;
    /// The first field in [`Self::SuffixSchema`], and the key for the next node in the trie.
    type Head: Eq + Hash + Clone;

    /// Create a new Ght from the iterator.
    fn new_from(input: impl IntoIterator<Item = Self::Schema>) -> Self;

    /// Merge a matching Ght node into this node
    fn merge_node(&mut self, other: Self) -> bool;

    /// Report the height of this node. This is the length of path from this node to a leaf - 1.
    /// E.g. if we have GhtInner<GhtInner<GhtLeaf...>> the height is 2
    /// This is a static property of the type of this node, so simply invokes the static method.
    fn height(&self) -> usize {
        Self::HEIGHT
    }

    /// The height of this node in the GhT. Leaf = 0.
    const HEIGHT: usize;

    /// Inserts an item into the hash trie.
    fn insert(&mut self, row: Self::Schema) -> bool;

    /// Returns `true` if the (entire) row is found below in the trie, `false` otherwise.
    /// See [`GhtGet::get`] to look just for "head" keys in this node
    fn contains<'a>(&'a self, row: <Self::Schema as VariadicExt>::AsRefVar<'a>) -> bool;

    /// Iterate through (entire) rows stored in this HashTrie.
    fn recursive_iter(&self) -> impl Iterator<Item = <Self::Schema as VariadicExt>::AsRefVar<'_>>;

    /// return the leaf below that contains this row, or `None` if not found.
    fn find_containing_leaf(
        &self,
        row: <Self::Schema as VariadicExt>::AsRefVar<'_>,
    ) -> Option<&'_ GhtLeaf<Self::Schema, Self::ValType, Self::Storage>>;

    /// into_iter for leaf elements, or None for inner nodes
    fn into_iter(self) -> Option<impl Iterator<Item = Self::Schema>>;

    /// pull all the data out of this trie node but retain the reference
    fn drain(&mut self) -> Option<impl Iterator<Item = Self::Schema>>;
}

/// internal node of a HashTrie
#[derive(Debug, Clone)]
pub struct GhtInner<Head, Node>
where
    Head: Clone,
    Node: GeneralizedHashTrieNode,
{
    pub(crate) children: HashMap<Head, Node>,
}

impl<Head, Node: GeneralizedHashTrieNode> Default for GhtInner<Head, Node>
where
    Head: Clone,
    Node: GeneralizedHashTrieNode,
{
    fn default() -> Self {
        let children = Default::default();
        Self { children }
    }
}

impl<Head, Node> GeneralizedHashTrieNode for GhtInner<Head, Node>
where
    Head: 'static + Hash + Eq + Clone,
    Node: 'static + GeneralizedHashTrieNode,
    Node::Schema: SplitBySuffix<var_type!(Head, ...Node::SuffixSchema)>,
{
    type Schema = Node::Schema;
    type KeyType = Node::KeyType;
    type ValType = Node::ValType;
    type Storage = Node::Storage;
    type SuffixSchema = var_type!(Head, ...Node::SuffixSchema);
    type Head = Head;

    fn new_from(input: impl IntoIterator<Item = Self::Schema>) -> Self {
        let mut retval: Self = Default::default();
        for row in input {
            retval.insert(row);
        }
        retval
    }

    fn merge_node(&mut self, other: Self) -> bool {
        let mut changed = false;

        for (k, v) in other.children {
            match self.children.entry(k) {
                std::collections::hash_map::Entry::Occupied(mut occupied) => {
                    changed |= occupied.get_mut().merge_node(v)
                }
                std::collections::hash_map::Entry::Vacant(vacant) => {
                    vacant.insert(v);
                    changed = true
                }
            }
        }
        changed
    }

    const HEIGHT: usize = Node::HEIGHT + 1;

    fn insert(&mut self, row: Self::Schema) -> bool {
        let (_prefix, var_args!(head, ..._rest)) =
            Self::Schema::split_by_suffix_ref(row.as_ref_var());
        self.children.entry(head.clone()).or_default().insert(row)
    }

    fn contains<'a>(&'a self, row: <Self::Schema as VariadicExt>::AsRefVar<'a>) -> bool {
        let (_prefix, var_args!(head, ..._rest)) = Self::Schema::split_by_suffix_ref(row);
        if let Some(node) = self.children.get(head) {
            node.contains(row)
        } else {
            false
        }
    }

    fn recursive_iter(&self) -> impl Iterator<Item = <Self::Schema as VariadicExt>::AsRefVar<'_>> {
        #[expect(
            clippy::disallowed_methods,
            reason = "nondeterministic iteration order, TODO(mingwei)"
        )]
        self.children
            .iter()
            .flat_map(|(_k, vs)| vs.recursive_iter())
    }

    fn find_containing_leaf(
        &self,
        row: <Self::Schema as VariadicExt>::AsRefVar<'_>,
    ) -> Option<&'_ GhtLeaf<Self::Schema, Self::ValType, Self::Storage>> {
        let (_prefix, var_args!(head, ..._rest)) = Self::Schema::split_by_suffix_ref(row);
        self.children
            .get(head)
            .and_then(|child| child.find_containing_leaf(row))
    }

    fn into_iter(self) -> Option<impl Iterator<Item = Self::Schema>> {
        None::<Box<dyn Iterator<Item = Self::Schema>>>
    }

    fn drain(&mut self) -> Option<impl Iterator<Item = Self::Schema>> {
        None::<Box<dyn Iterator<Item = Self::Schema>>>
    }
}

impl<Head, Node> FromIterator<Node::Schema> for GhtInner<Head, Node>
where
    Head: 'static + Hash + Eq + Clone,
    Node: 'static + GeneralizedHashTrieNode + Clone,
    Node::Schema: SplitBySuffix<var_type!(Head, ...Node::SuffixSchema)>,
{
    fn from_iter<Iter: IntoIterator<Item = Node::Schema>>(iter: Iter) -> Self {
        let mut out = Self::default();
        for row in iter {
            out.insert(row);
        }
        out
    }
}

/// leaf node of a HashTrie
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct GhtLeaf<Schema, ValType, Storage>
where
    Schema: Eq + Hash,
    Storage: VariadicCollection<Schema = Schema>,
{
    pub(crate) elements: Storage,
    pub(crate) forced: bool,
    /// defines ValType for the parents, recursively
    pub(crate) _suffix_schema: PhantomData<ValType>,
}
impl<Schema, ValType, Storage> Default for GhtLeaf<Schema, ValType, Storage>
where
    Schema: Eq + Hash,
    Storage: VariadicCollection<Schema = Schema> + Default,
{
    fn default() -> Self {
        let elements = Storage::default();
        Self {
            elements,
            forced: false,
            _suffix_schema: PhantomData,
        }
    }
}

impl<Schema, ValHead, ValRest, Storage> GeneralizedHashTrieNode
    for GhtLeaf<Schema, var_type!(ValHead, ...ValRest), Storage>
where
    Schema: 'static
        + Eq
        + VariadicExt
        + Hash
        + Clone
        + SplitBySuffix<var_type!(ValHead, ...ValRest)>
        + PartialEqVariadic,
    ValHead: Clone + Eq + Hash,
    var_type!(ValHead, ...ValRest): Clone + Eq + Hash + PartialEqVariadic,
    <Schema as SplitBySuffix<var_type!(ValHead, ...ValRest)>>::Prefix: Eq + Hash + Clone,
    Storage: VariadicCollection<Schema = Schema> + Default + IntoIterator<Item = Schema>,
{
    type Schema = Schema;
    type SuffixSchema = var_type!(ValHead, ...ValRest);
    type ValType = var_type!(ValHead, ...ValRest);
    type KeyType = <Schema as SplitBySuffix<var_type!(ValHead, ...ValRest)>>::Prefix;
    type Head = ValHead;
    type Storage = Storage;

    fn new_from(input: impl IntoIterator<Item = Self::Schema>) -> Self {
        let mut retval: Self = Default::default();
        for i in input {
            retval.insert(i);
        }
        retval
    }

    fn merge_node(&mut self, other: Self) -> bool {
        let old_len = self.elements.len();
        self.elements.extend(other.elements);
        self.elements.len() > old_len
    }

    const HEIGHT: usize = 0;

    fn insert(&mut self, row: Self::Schema) -> bool {
        self.elements.insert(row);
        true
    }

    fn contains<'a>(&'a self, row: <Self::Schema as VariadicExt>::AsRefVar<'a>) -> bool {
        self.elements.iter().any(|r| Schema::eq_ref(r, row))
    }

    fn recursive_iter(&self) -> impl Iterator<Item = <Self::Schema as VariadicExt>::AsRefVar<'_>> {
        self.elements.iter()
    }

    fn find_containing_leaf(
        &self,
        row: <Self::Schema as VariadicExt>::AsRefVar<'_>,
    ) -> Option<&'_ GhtLeaf<<Self as GeneralizedHashTrieNode>::Schema, Self::ValType, Self::Storage>>
    {
        // TODO(mingwei): actually use the hash set as a hash set
        if self
            .elements
            .iter()
            .any(|x| <Schema as PartialEqVariadic>::eq_ref(row, x))
        {
            Some(self)
        } else {
            None
        }
    }

    fn into_iter(self) -> Option<impl Iterator<Item = Self::Schema>> {
        Some(self.elements.into_iter())
    }

    fn drain(&mut self) -> Option<impl Iterator<Item = Self::Schema>> {
        Some(self.elements.drain())
    }
}

impl<Schema, Storage> GeneralizedHashTrieNode for GhtLeaf<Schema, (), Storage>
where
    Schema: 'static + Eq + VariadicExt + Hash + Clone + PartialEqVariadic,
    Storage: VariadicCollection<Schema = Schema> + Default + IntoIterator<Item = Schema>,
{
    type Schema = Schema;
    type SuffixSchema = ();
    type ValType = ();
    type KeyType = Schema;
    type Head = ();
    type Storage = Storage;

    fn new_from(input: impl IntoIterator<Item = Self::Schema>) -> Self {
        let mut retval: Self = Default::default();
        for i in input {
            retval.insert(i);
        }
        retval
    }

    fn merge_node(&mut self, other: Self) -> bool {
        let old_len = self.elements.len();
        self.elements.extend(other.elements);
        self.elements.len() > old_len
    }

    const HEIGHT: usize = 0;

    fn insert(&mut self, row: Self::Schema) -> bool {
        self.elements.insert(row);
        true
    }

    fn contains<'a>(&'a self, row: <Self::Schema as VariadicExt>::AsRefVar<'a>) -> bool {
        self.elements.iter().any(|r| Schema::eq_ref(r, row))
    }

    fn recursive_iter(&self) -> impl Iterator<Item = <Self::Schema as VariadicExt>::AsRefVar<'_>> {
        self.elements.iter()
    }

    fn find_containing_leaf(
        &self,
        row: <Self::Schema as VariadicExt>::AsRefVar<'_>,
    ) -> Option<&'_ GhtLeaf<<Self as GeneralizedHashTrieNode>::Schema, Self::ValType, Self::Storage>>
    {
        // TODO(mingwei): actually use the hash set as a hash set
        if self
            .elements
            .iter()
            .any(|x| <Schema as PartialEqVariadic>::eq_ref(row, x))
        {
            Some(self)
        } else {
            None
        }
    }

    fn into_iter(self) -> Option<impl Iterator<Item = Self::Schema>> {
        Some(self.elements.into_iter())
    }

    fn drain(&mut self) -> Option<impl Iterator<Item = Self::Schema>> {
        Some(self.elements.drain())
    }
}

impl<Schema, ValType, Storage> FromIterator<Schema> for GhtLeaf<Schema, ValType, Storage>
where
    Schema: Eq + Hash,
    Storage: VariadicCollection<Schema = Schema> + Default + FromIterator<Schema>,
{
    fn from_iter<Iter: IntoIterator<Item = Schema>>(iter: Iter) -> Self {
        let elements = iter.into_iter().collect();
        Self {
            elements,
            forced: false,
            _suffix_schema: PhantomData,
        }
    }
}

/// A trait for the get and iter methods from Wang/Willsey/Suciu, which
/// work differently on leaves than internal nodes
pub trait GhtGet: GeneralizedHashTrieNode {
    /// Type returned by [`Self::get`].
    type Get: GeneralizedHashTrieNode<Schema = Self::Schema>;

    /// On an Inner node, retrieves the value (child) associated with the given "head" key.
    /// returns an `Option` containing a reference to the value if found, or `None` if not found.
    /// On a Leaf node, returns None.
    fn get<'a>(&'a self, head: &Self::Head) -> Option<&'a Self::Get>;

    /// get, but mutable output
    fn get_mut<'a>(&'a mut self, head: &Self::Head) -> Option<&'a mut Self::Get>;

    /// Iterator for the "head" keys (from inner nodes) or nothing (from leaf nodes).
    fn iter(&self) -> impl Iterator<Item = Self::Head>;

    /// Iterator for the tuples (from leaf nodes) or nothing (from inner nodes).
    fn iter_tuples(&self) -> impl Iterator<Item = <Self::Schema as VariadicExt>::AsRefVar<'_>>;
}

impl<Head, Node> GhtGet for GhtInner<Head, Node>
where
    Head: 'static + Eq + Hash + Clone,
    Node: 'static + GeneralizedHashTrieNode,
    Node::Schema: SplitBySuffix<var_type!(Head, ...Node::SuffixSchema)>,
{
    /// Type returned by [`Self::get`].
    type Get = Node;

    /// On an Inner node, retrieves the value (child) associated with the given "head" key.
    /// returns an `Option` containing a reference to the value if found, or `None` if not found.
    /// On a Leaf node, returns None.
    fn get<'a>(&'a self, head: &Self::Head) -> Option<&'a Self::Get> {
        self.children.get(head)
    }

    fn get_mut<'a>(&'a mut self, head: &Self::Head) -> Option<&'a mut Self::Get> {
        self.children.get_mut(head)
    }

    fn iter(&self) -> impl Iterator<Item = Self::Head> {
        #[expect(
            clippy::disallowed_methods,
            reason = "nondeterministic iteration order, TODO(mingwei)"
        )]
        self.children.keys().cloned()
    }

    fn iter_tuples(&self) -> impl Iterator<Item = <Self::Schema as VariadicExt>::AsRefVar<'_>> {
        std::iter::empty()
    }
}

impl<Schema, ValType, Storage> GhtGet for GhtLeaf<Schema, ValType, Storage>
where
    Schema: 'static + Eq + Hash + Clone + PartialEqVariadic + SplitBySuffix<ValType>,
    ValType: Eq + Hash + Clone + PartialEqVariadic,
    <Schema as SplitBySuffix<ValType>>::Prefix: Eq + Hash + Clone,
    GhtLeaf<Schema, ValType, Storage>: GeneralizedHashTrieNode<Schema = Schema>,
    Storage: VariadicCollection<Schema = Schema>,
{
    /// Type returned by [`Self::get`].
    type Get = GhtLeaf<Schema, ValType, Storage>;

    /// On an Inner node, retrieves the value (child) associated with the given "head" key.
    /// returns an `Option` containing a reference to the value if found, or `None` if not found.
    /// On a Leaf node, returns None.
    fn get<'a>(&'a self, _head: &Self::Head) -> Option<&'a Self::Get> {
        None
    }
    fn get_mut<'a>(&'a mut self, _head: &Self::Head) -> Option<&'a mut Self::Get> {
        None
    }

    fn iter(&self) -> impl Iterator<Item = Self::Head> {
        std::iter::empty()
    }

    fn iter_tuples(&self) -> impl Iterator<Item = <Self::Schema as VariadicExt>::AsRefVar<'_>> {
        self.elements.iter()
    }
}

/// A trait to iterate through the items in a Ght based on a prefix of the schema.
pub trait GhtPrefixIter<KeyPrefix> {
    /// the schema output
    type Item: VariadicExt;
    /// given a prefix, return an iterator through the items below
    fn prefix_iter<'a>(
        &'a self,
        prefix: KeyPrefix,
    ) -> impl Iterator<Item = <Self::Item as VariadicExt>::AsRefVar<'a>>
    where
        Self::Item: 'a;
}

impl<'k, Head, Node, PrefixRest> GhtPrefixIter<var_type!(&'k Head, ...PrefixRest)>
    for GhtInner<Head, Node>
where
    Head: Eq + Hash + Clone,
    Node: GeneralizedHashTrieNode + GhtPrefixIter<PrefixRest>,
{
    type Item = <Node as GhtPrefixIter<PrefixRest>>::Item;
    fn prefix_iter<'a>(
        &'a self,
        prefix: var_type!(&'k Head, ...PrefixRest),
    ) -> impl Iterator<Item = <Self::Item as VariadicExt>::AsRefVar<'a>>
    where
        Self::Item: 'a,
    {
        let var_args!(head, ...rest) = prefix;
        self.children
            .get(head)
            .map(|node| node.prefix_iter(rest))
            .into_iter()
            .flatten()
    }
}
impl<Head, Node> GhtPrefixIter<var_type!()> for GhtInner<Head, Node>
where
    Self: GeneralizedHashTrieNode,
    Head: Eq + Hash + Clone,
    Node: GeneralizedHashTrieNode,
{
    type Item = <Self as GeneralizedHashTrieNode>::Schema;
    fn prefix_iter<'a>(
        &'a self,
        _prefix: var_type!(),
    ) -> impl Iterator<Item = <Self::Item as VariadicExt>::AsRefVar<'a>>
    where
        Self::Item: 'a,
    {
        self.recursive_iter()
    }
}

impl<KeyPrefixRef, Schema, ValType, Storage> GhtPrefixIter<KeyPrefixRef>
    for GhtLeaf<Schema, ValType, Storage>
where
    KeyPrefixRef: 'static + RefVariadic,
    Schema: 'static + VariadicExt + Hash + Eq + SplitBySuffix<ValType>,
    ValType: VariadicExt,
    ValType: Split<KeyPrefixRef::UnRefVar>,
    KeyPrefixRef::UnRefVar: PartialEqVariadic,
    Storage: 'static + VariadicCollection<Schema = Schema>,
{
    type Item = Schema;
    fn prefix_iter<'a>(
        &'a self,
        prefix: KeyPrefixRef,
    ) -> impl Iterator<Item = <Self::Item as VariadicExt>::AsRefVar<'a>>
    where
        Self::Item: 'a,
    {
        self.elements.iter().filter(move |&row| {
            let (_row_prefix, row_mid_suffix) =
                <Schema as SplitBySuffix<ValType>>::split_by_suffix_ref(row);
            let (row_mid, _row_suffix): (<KeyPrefixRef::UnRefVar as VariadicExt>::AsRefVar<'_>, _) =
                <ValType as Split<KeyPrefixRef::UnRefVar>>::split_ref(row_mid_suffix);
            <KeyPrefixRef::UnRefVar as PartialEqVariadic>::eq_ref(prefix.unref_ref(), row_mid)
        })
    }
}
