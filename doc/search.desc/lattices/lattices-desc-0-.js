searchState.loadedDescShard("lattices", 0, "The <code>lattices</code> crate provides ergonomic and compsable …\nThe type of atoms for this lattice.\nThe iter type iterating the antichain atoms.\nTrait to atomize a lattice into individual elements. For …\nA <code>Conflict</code> lattice, stores a single instance of <code>T</code> and goes …\nTrait for recursively revealing the underlying types …\nDominating pair compound lattice.\nTrait to check if a lattice instance is bottom (⊥).\nTrait to check if a lattice instance is top (⊤) and …\nAlias trait for lattice types.\nSemilattice bimorphism. Lattice merge must distribute over …\nSame as <code>From</code> but for lattices.\nSemilattice morphism. Lattice merge must distribute over …\nTrait for lattice partial order comparison PartialOrd is …\nA totally ordered max lattice. Merging returns the larger …\nTrait for lattice merge (AKA “join” or “least upper …\nA totally ordered min lattice. Merging returns the smaller …\nNaive lattice compare, based on the <code>Merge::merge</code> function.\nThe output lattice type.\nThe output lattice type.\nPair compound lattice.\nA <code>Point</code> lattice, corresponding to a single instance of <code>T</code>.\nThe underlying type when revealed.\nVec-union compound lattice.\nWraps a lattice in <code>Option</code>, treating <code>None</code> as a new bottom …\nWraps a lattice in <code>Option</code>, treating <code>None</code> as a new top …\nModule for definiting algebraic structures and properties.\nReveal the inner value as an exclusive reference.\nReveal the inner value as an exclusive reference.\nReveal the inner value as an exclusive reference.\nReveal the inner value as an exclusive reference.\nReveal the inner value as an exclusive reference.\nReveal the inner value as an exclusive reference.\nReveal the inner value as an exclusive reference.\nReveal the inner value as an exclusive reference.\nReveal the inner value as a shared reference.\nReveal the inner value as a shared reference.\nReveal the inner value as a shared reference.\nReveal the inner value as a shared reference.\nReveal the inner value as a shared reference.\nReveal the inner value as a shared reference.\nReveal the inner value as a shared reference.\nReveal the inner value as a shared reference.\nAtomize self: convert into an iter of atoms.\nExecutes the function.\nExecutes the function.\nConverts a closure to a bimorphism. Does not check for …\nConverts a closure to a morphism. Does not check for …\nSimple singleton or array collection with <code>cc_traits</code> …\nReveals the underlying lattice types recursively.\nReturns the argument unchanged.\nReturns the argument unchanged.\nCreate a new <code>Max</code> lattice instance from an <code>Into&lt;T&gt;</code> value.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nGets the inner by value, consuming self.\nGets the inner by value, consuming self.\nGets the inner by value, consuming self.\nGets the inner by value, consuming self.\nGets the inner by value, consuming self.\nGets the inner by value, consuming self.\nGets the inner by value, consuming self.\nGets the inner by value, consuming self.\nReturns if <code>self</code> is lattice bottom (⊥).\nReturns if <code>self</code> is lattice top (⊤).\nThe <code>Key</code> of the  dominating pair lattice, usually a …\nConvert from the <code>Other</code> lattice into <code>Self</code>.\nModule containing the <code>MapUnion</code> lattice and aliases for …\nModule containing the <code>MapUnionWithTombstones</code> lattice and …\nMerge <code>other</code> into the <code>self</code> lattice.\nMerge <code>this</code> and <code>delta</code> together, returning the new value.\nNaive compare based on the <code>Merge::merge</code> method. This …\nCreate a new <code>Conflict</code> lattice instance from a value.\nCreate a <code>DomPair</code> from the given <code>Key</code> and <code>Val</code>.\nCreate a new <code>Max</code> lattice instance from a <code>T</code>.\nCreate a new <code>Min</code> lattice instance from a <code>T</code>.\nCreate a <code>Pair</code> from the given values.\nCreate a new <code>Point</code> lattice instance from a value.\nCreate a new <code>VecUnion</code> from a <code>Vec</code> of <code>Lat</code> instances.\nCreate a new <code>WithBot</code> lattice instance from a value.\nCreate a new <code>WithTop</code> lattice instance from a value.\nCreate a new <code>Conflict</code> lattice instance from a value using …\nCreate a <code>DomPair</code> from the given <code>Into&lt;Key&gt;</code> and <code>Into&lt;Val&gt;</code>.\nCreate a new <code>Min</code> lattice instance from an <code>Into&lt;T&gt;</code> value.\nCreate a <code>Pair</code> from the given values, using <code>Into</code>.\nCreate a new <code>Point</code> lattice instance from a value using <code>Into</code>…\nCreate a new <code>VecUnion</code> from an <code>Into&lt;Vec&lt;Lat&gt;&gt;</code>.\nCreate a new <code>WithBot</code> lattice instance from a value using …\nCreate a new <code>WithTop</code> lattice instance from a value using …\nModule containing the <code>SetUnion</code> lattice and aliases for …\nModule containing the <code>SetUnionWithTombstones</code> lattice and …\nHelper test utils to test lattice implementation …\nModule containing the <code>UnionFind</code> lattice and aliases for …\nThe value stored inside. This should not be mutated.\nDefines an abelian group structure. An abelian group is a …\nDefines the absorbing_element property. An element z is …\nDefines the associativity property. a(bc) = (ab)c\nDefines a commutative monoid structure. A commutative …\nDefines a commutative ring structure. A commutative ring …\nDefines the commutativity property. xy = yx\nDefines the distributive property a(b+c) = ab + ac and …\nDefines a field structure. A field is a commutative ring …\nDefines a group structure. A group is a set of items along …\nDefines the idempotency property. xx = x\nDefines the identity property. An element e is the …\nDefines an integral domain structure. An integral domain …\nDefines the inverse property. An element b is the inverse …\nDefines the left distributive property a(b+c) = ab + ac\nDefines a monoid structure. A monoid is a set of items …\nDefines a no-nonzero-zero-divisors property. x is a …\nDefines the non_zero inverse property. Every element …\nDefines the right distributive property. (b+c)a = ba + ca\nDefines a ring structure. A ring is a semiring with an …\nDefines a semigroup structure. A semigroup is a set of …\nDefines a semiring structure. A semiring is a set of items …\nAn array wrapper representing a fixed-size map.\nAn array wrapper representing a fixed-size set (modulo …\nA key-value entry wrapper representing a singleton map.\nA type that will always be an empty set.\nTrait for transforming the values of a map without …\nOutput type, should be <code>Self</code> but with <code>OldVal</code> replaced with …\nA key-value entry wrapper around <code>Option&lt;(K, V)&gt;</code> …\nA wrapper around <code>Option</code>, representing either a singleton …\nA key-value entry wrapper representing a singleton map.\nA wrapper around an item, representing a singleton set.\nA <code>Vec</code>-wrapper representing a naively implemented map.\nA <code>Vec</code>-wrapper representing a naively-implemented set.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nKeys, should be the same length as and correspond 1:1 to …\nKeys, corresponding 1:1 with <code>vals</code>.\nMap the values into using the <code>map_fn</code>.\nCreate a new <code>VecMap</code> from the separate <code>keys</code> and <code>vals</code> vecs.\nVals, should be the same length as and correspond 1:1 to …\nValues, corresponding 1:1 with <code>keys</code>.\nComposable bimorphism, wraps an existing morphism by …\nMap-union compound lattice.\nArray-backed <code>MapUnion</code> lattice.\n<code>std::collections::BTreeMap</code>-backed <code>MapUnion</code> lattice.\n<code>std::collections::HashMap</code>-backed <code>MapUnion</code> lattice.\n<code>Option</code>-backed <code>MapUnion</code> lattice.\n<code>crate::collections::SingletonMap</code>-backed <code>MapUnion</code> lattice.\n<code>Vec</code>-backed <code>MapUnion</code> lattice.\nReveal the inner value as an exclusive reference.\nReveal the inner value as a shared reference.\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nGets the inner by value, consuming self.\nCreate a new <code>MapUnion</code> from a <code>Map</code>.\nCreate a new <code>MapUnion</code> from an <code>Into&lt;Map&gt;</code>.\n<code>std::collections::HashMap</code>-backed <code>MapUnionWithTombstones</code> …\nMap-union-with-tombstones compound lattice.\n<code>crate::collections::SingletonMap</code>-backed …\n<code>crate::collections::SingletonSet</code>-backed …\nReveal the inner value as an exclusive reference.\nReveal the inner value as a shared reference.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nGets the inner by value, consuming self.\nCreate a new <code>MapUnionWithTombstones</code> from a <code>Map</code> and a …\nCreate a new <code>MapUnionWithTombstones</code> from an <code>Into&lt;Map&gt;</code> and …\nBimorphism for the cartesian product of two sets. Output …\nSet-union lattice.\n<code>crate::collections::ArraySet</code>-backed <code>SetUnion</code> lattice.\n<code>std::collections::BTreeSet</code>-backed <code>SetUnion</code> lattice.\n<code>std::collections::HashSet</code>-backed <code>SetUnion</code> lattice.\n<code>Option</code>-backed <code>SetUnion</code> lattice.\n<code>crate::collections::SingletonSet</code>-backed <code>SetUnion</code> lattice.\n<code>Vec</code>-backed <code>SetUnion</code> lattice.\nReveal the inner value as an exclusive reference.\nReveal the inner value as a shared reference.\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nGets the inner by value, consuming self.\nCreate a new <code>SetUnion</code> from a <code>Set</code>.\nCreate a new <code>SetUnion</code> from an <code>Into&lt;Set&gt;</code>.\nSet-union lattice with tombstones.\n<code>crate::collections::ArraySet</code>-backed <code>SetUnionWithTombstones</code> …\n<code>std::collections::BTreeSet</code>-backed <code>SetUnionWithTombstones</code> …\n<code>std::collections::HashSet</code>-backed <code>SetUnionWithTombstones</code> …\n<code>Option</code>-backed <code>SetUnionWithTombstones</code> lattice.\n<code>crate::collections::SingletonSet</code>-backed …\n<code>crate::collections::SingletonSet</code>-backed …\n<code>Vec</code>-backed <code>SetUnionWithTombstones</code> lattice.\nReveal the inner value as an exclusive reference.\nReveal the inner value as a shared reference.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nGets the inner by value, consuming self.\nCreate a new <code>SetUnionWithTombstones</code> from a <code>Set</code> and …\nCreate a new <code>SetUnionWithTombstones</code> from an <code>Into&lt;Set&gt;</code> and …\nReturns an iterator of <code>N</code>-length arrays containing all …\nHelper which calls many other <code>check_*</code> functions in this …\nCheck that the atomized lattice points re-merge to form …\nChecks that the <code>LatticeBimorphism</code> is valid, i.e. that …\nAsserts that <code>IsBot</code> is true for <code>Default::default()</code>.\nChecks that the item which is bot is less than (or equal …\nChecks that the item which is top is greater than (or …\nChecks that the <code>LatticeMorphism</code> is valid, i.e. that merge …\nCheck that the lattice’s <code>PartialOrd</code> implementation …\nCheck lattice associativity, commutativity, and …\nChecks <code>PartialOrd</code>, <code>PartialEq</code>, and <code>Eq</code>’s reflexivity, …\nUnion-find lattice.\nArray-backed <code>UnionFind</code> lattice.\n<code>std::collections::BTreeMap</code>-backed <code>UnionFind</code> lattice.\n<code>std::collections::HashMap</code>-backed <code>UnionFind</code> lattice.\n<code>Option</code>-backed <code>UnionFind</code> lattice.\n<code>crate::collections::SingletonMap</code>-backed <code>UnionFind</code> lattice.\n<code>Vec</code>-backed <code>UnionFind</code> lattice.\nReveal the inner value as an exclusive reference.\nReveal the inner value as a shared reference.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nGets the inner by value, consuming self.\nCreate a new <code>UnionFind</code> from a <code>Map</code>.\nCreate a new <code>UnionFind</code> from an <code>Into&lt;Map&gt;</code>.\nReturns if <code>a</code> and <code>b</code> are in the same set.\nUnion the sets containg <code>a</code> and <code>b</code>.")