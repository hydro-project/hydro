(function() {
    var type_impls = Object.fromEntries([["gossip_kv",[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Clone-for-DomPair%3CKey,+Val%3E\" class=\"impl\"><a href=\"#impl-Clone-for-DomPair%3CKey,+Val%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Key, Val&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> for DomPair&lt;Key, Val&gt;<div class=\"where\">where\n    Key: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a>,\n    Val: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a>,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone\" class=\"method trait-impl\"><a href=\"#method.clone\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html#tymethod.clone\" class=\"fn\">clone</a>(&amp;self) -&gt; DomPair&lt;Key, Val&gt;</h4></section></summary><div class='docblock'>Returns a copy of the value. <a href=\"https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html#tymethod.clone\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone_from\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/nightly/src/core/clone.rs.html#174\">Source</a></span><a href=\"#method.clone_from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html#method.clone_from\" class=\"fn\">clone_from</a>(&amp;mut self, source: &amp;Self)</h4></section></summary><div class='docblock'>Performs copy-assignment from <code>source</code>. <a href=\"https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html#method.clone_from\">Read more</a></div></details></div></details>","Clone","gossip_kv::model::RowValue"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Debug-for-DomPair%3CKey,+Val%3E\" class=\"impl\"><a href=\"#impl-Debug-for-DomPair%3CKey,+Val%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Key, Val&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> for DomPair&lt;Key, Val&gt;<div class=\"where\">where\n    Key: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a>,\n    Val: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a>,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fmt\" class=\"method trait-impl\"><a href=\"#method.fmt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/fmt/trait.Debug.html#tymethod.fmt\" class=\"fn\">fmt</a>(&amp;self, f: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/core/fmt/struct.Formatter.html\" title=\"struct core::fmt::Formatter\">Formatter</a>&lt;'_&gt;) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/nightly/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.unit.html\">()</a>, <a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/core/fmt/struct.Error.html\" title=\"struct core::fmt::Error\">Error</a>&gt;</h4></section></summary><div class='docblock'>Formats the value using the given formatter. <a href=\"https://doc.rust-lang.org/nightly/core/fmt/trait.Debug.html#tymethod.fmt\">Read more</a></div></details></div></details>","Debug","gossip_kv::model::RowValue"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-DeepReveal-for-DomPair%3CKey,+Val%3E\" class=\"impl\"><a href=\"#impl-DeepReveal-for-DomPair%3CKey,+Val%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Key, Val&gt; DeepReveal for DomPair&lt;Key, Val&gt;<div class=\"where\">where\n    Key: DeepReveal,\n    Val: DeepReveal,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle\" open><summary><section id=\"associatedtype.Revealed\" class=\"associatedtype trait-impl\"><a href=\"#associatedtype.Revealed\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a class=\"associatedtype\">Revealed</a> = (&lt;Key as DeepReveal&gt;::Revealed, &lt;Val as DeepReveal&gt;::Revealed)</h4></section></summary><div class='docblock'>The underlying type when revealed.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.deep_reveal\" class=\"method trait-impl\"><a href=\"#method.deep_reveal\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">deep_reveal</a>(self) -&gt; &lt;DomPair&lt;Key, Val&gt; as DeepReveal&gt;::Revealed</h4></section></summary><div class='docblock'>Reveals the underlying lattice types recursively.</div></details></div></details>","DeepReveal","gossip_kv::model::RowValue"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Default-for-DomPair%3CKey,+Val%3E\" class=\"impl\"><a href=\"#impl-Default-for-DomPair%3CKey,+Val%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Key, Val&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a> for DomPair&lt;Key, Val&gt;<div class=\"where\">where\n    Key: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a>,\n    Val: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a>,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.default\" class=\"method trait-impl\"><a href=\"#method.default\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/default/trait.Default.html#tymethod.default\" class=\"fn\">default</a>() -&gt; DomPair&lt;Key, Val&gt;</h4></section></summary><div class='docblock'>Returns the “default value” for a type. <a href=\"https://doc.rust-lang.org/nightly/core/default/trait.Default.html#tymethod.default\">Read more</a></div></details></div></details>","Default","gossip_kv::model::RowValue"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Deserialize%3C'de%3E-for-DomPair%3CKey,+Val%3E\" class=\"impl\"><a href=\"#impl-Deserialize%3C'de%3E-for-DomPair%3CKey,+Val%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;'de, Key, Val&gt; <a class=\"trait\" href=\"https://docs.rs/serde/1.0.210/serde/de/trait.Deserialize.html\" title=\"trait serde::de::Deserialize\">Deserialize</a>&lt;'de&gt; for DomPair&lt;Key, Val&gt;<div class=\"where\">where\n    Key: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.210/serde/de/trait.Deserialize.html\" title=\"trait serde::de::Deserialize\">Deserialize</a>&lt;'de&gt;,\n    Val: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.210/serde/de/trait.Deserialize.html\" title=\"trait serde::de::Deserialize\">Deserialize</a>&lt;'de&gt;,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.deserialize\" class=\"method trait-impl\"><a href=\"#method.deserialize\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://docs.rs/serde/1.0.210/serde/de/trait.Deserialize.html#tymethod.deserialize\" class=\"fn\">deserialize</a>&lt;__D&gt;(\n    __deserializer: __D,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/nightly/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;DomPair&lt;Key, Val&gt;, &lt;__D as <a class=\"trait\" href=\"https://docs.rs/serde/1.0.210/serde/de/trait.Deserializer.html\" title=\"trait serde::de::Deserializer\">Deserializer</a>&lt;'de&gt;&gt;::<a class=\"associatedtype\" href=\"https://docs.rs/serde/1.0.210/serde/de/trait.Deserializer.html#associatedtype.Error\" title=\"type serde::de::Deserializer::Error\">Error</a>&gt;<div class=\"where\">where\n    __D: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.210/serde/de/trait.Deserializer.html\" title=\"trait serde::de::Deserializer\">Deserializer</a>&lt;'de&gt;,</div></h4></section></summary><div class='docblock'>Deserialize this value from the given Serde deserializer. <a href=\"https://docs.rs/serde/1.0.210/serde/de/trait.Deserialize.html#tymethod.deserialize\">Read more</a></div></details></div></details>","Deserialize<'de>","gossip_kv::model::RowValue"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-DomPair%3CKey,+Val%3E\" class=\"impl\"><a href=\"#impl-DomPair%3CKey,+Val%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Key, Val&gt; DomPair&lt;Key, Val&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.new\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">new</a>(key: Key, val: Val) -&gt; DomPair&lt;Key, Val&gt;</h4></section></summary><div class=\"docblock\"><p>Create a <code>DomPair</code> from the given <code>Key</code> and <code>Val</code>.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.new_from\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">new_from</a>(key: impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/convert/trait.Into.html\" title=\"trait core::convert::Into\">Into</a>&lt;Key&gt;, val: impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/convert/trait.Into.html\" title=\"trait core::convert::Into\">Into</a>&lt;Val&gt;) -&gt; DomPair&lt;Key, Val&gt;</h4></section></summary><div class=\"docblock\"><p>Create a <code>DomPair</code> from the given <code>Into&lt;Key&gt;</code> and <code>Into&lt;Val&gt;</code>.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.as_reveal_ref\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">as_reveal_ref</a>(&amp;self) -&gt; (<a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.reference.html\">&amp;Key</a>, <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.reference.html\">&amp;Val</a>)</h4></section></summary><div class=\"docblock\"><p>Reveal the inner value as a shared reference.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.as_reveal_mut\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">as_reveal_mut</a>(&amp;mut self) -&gt; (<a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.reference.html\">&amp;mut Key</a>, <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.reference.html\">&amp;mut Val</a>)</h4></section></summary><div class=\"docblock\"><p>Reveal the inner value as an exclusive reference.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.into_reveal\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">into_reveal</a>(self) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.tuple.html\">(Key, Val)</a></h4></section></summary><div class=\"docblock\"><p>Gets the inner by value, consuming self.</p>\n</div></details></div></details>",0,"gossip_kv::model::RowValue"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-IsBot-for-DomPair%3CKey,+Val%3E\" class=\"impl\"><a href=\"#impl-IsBot-for-DomPair%3CKey,+Val%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Key, Val&gt; IsBot for DomPair&lt;Key, Val&gt;<div class=\"where\">where\n    Key: IsBot,\n    Val: IsBot,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.is_bot\" class=\"method trait-impl\"><a href=\"#method.is_bot\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">is_bot</a>(&amp;self) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>Returns if <code>self</code> is lattice bottom (⊥). <a>Read more</a></div></details></div></details>","IsBot","gossip_kv::model::RowValue"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-IsTop-for-DomPair%3CKey,+Val%3E\" class=\"impl\"><a href=\"#impl-IsTop-for-DomPair%3CKey,+Val%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Key, Val&gt; IsTop for DomPair&lt;Key, Val&gt;<div class=\"where\">where\n    Key: IsTop,\n    Val: IsTop,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.is_top\" class=\"method trait-impl\"><a href=\"#method.is_top\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">is_top</a>(&amp;self) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>Returns if <code>self</code> is lattice top (⊤). <a>Read more</a></div></details></div></details>","IsTop","gossip_kv::model::RowValue"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-LatticeFrom%3CDomPair%3CKeyOther,+ValOther%3E%3E-for-DomPair%3CKeySelf,+ValSelf%3E\" class=\"impl\"><a href=\"#impl-LatticeFrom%3CDomPair%3CKeyOther,+ValOther%3E%3E-for-DomPair%3CKeySelf,+ValSelf%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;KeySelf, KeyOther, ValSelf, ValOther&gt; LatticeFrom&lt;DomPair&lt;KeyOther, ValOther&gt;&gt; for DomPair&lt;KeySelf, ValSelf&gt;<div class=\"where\">where\n    KeySelf: LatticeFrom&lt;KeyOther&gt;,\n    ValSelf: LatticeFrom&lt;ValOther&gt;,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.lattice_from\" class=\"method trait-impl\"><a href=\"#method.lattice_from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">lattice_from</a>(other: DomPair&lt;KeyOther, ValOther&gt;) -&gt; DomPair&lt;KeySelf, ValSelf&gt;</h4></section></summary><div class='docblock'>Convert from the <code>Other</code> lattice into <code>Self</code>.</div></details></div></details>","LatticeFrom<DomPair<KeyOther, ValOther>>","gossip_kv::model::RowValue"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Merge%3CDomPair%3CKeyOther,+ValOther%3E%3E-for-DomPair%3CKeySelf,+ValSelf%3E\" class=\"impl\"><a href=\"#impl-Merge%3CDomPair%3CKeyOther,+ValOther%3E%3E-for-DomPair%3CKeySelf,+ValSelf%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;KeySelf, KeyOther, ValSelf, ValOther&gt; Merge&lt;DomPair&lt;KeyOther, ValOther&gt;&gt; for DomPair&lt;KeySelf, ValSelf&gt;<div class=\"where\">where\n    KeySelf: Merge&lt;KeyOther&gt; + LatticeFrom&lt;KeyOther&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialOrd.html\" title=\"trait core::cmp::PartialOrd\">PartialOrd</a>&lt;KeyOther&gt;,\n    ValSelf: Merge&lt;ValOther&gt; + LatticeFrom&lt;ValOther&gt;,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.merge\" class=\"method trait-impl\"><a href=\"#method.merge\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">merge</a>(&amp;mut self, other: DomPair&lt;KeyOther, ValOther&gt;) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>Merge <code>other</code> into the <code>self</code> lattice. <a>Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.merge_owned\" class=\"method trait-impl\"><a href=\"#method.merge_owned\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">merge_owned</a>(this: Self, delta: Other) -&gt; Self<div class=\"where\">where\n    Self: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>,</div></h4></section></summary><div class='docblock'>Merge <code>this</code> and <code>delta</code> together, returning the new value.</div></details></div></details>","Merge<DomPair<KeyOther, ValOther>>","gossip_kv::model::RowValue"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-PartialEq%3CDomPair%3CKeyOther,+ValOther%3E%3E-for-DomPair%3CKeySelf,+ValSelf%3E\" class=\"impl\"><a href=\"#impl-PartialEq%3CDomPair%3CKeyOther,+ValOther%3E%3E-for-DomPair%3CKeySelf,+ValSelf%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;KeySelf, KeyOther, ValSelf, ValOther&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialEq.html\" title=\"trait core::cmp::PartialEq\">PartialEq</a>&lt;DomPair&lt;KeyOther, ValOther&gt;&gt; for DomPair&lt;KeySelf, ValSelf&gt;<div class=\"where\">where\n    KeySelf: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialEq.html\" title=\"trait core::cmp::PartialEq\">PartialEq</a>&lt;KeyOther&gt;,\n    ValSelf: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialEq.html\" title=\"trait core::cmp::PartialEq\">PartialEq</a>&lt;ValOther&gt;,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.eq\" class=\"method trait-impl\"><a href=\"#method.eq\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialEq.html#tymethod.eq\" class=\"fn\">eq</a>(&amp;self, other: &amp;DomPair&lt;KeyOther, ValOther&gt;) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>Tests for <code>self</code> and <code>other</code> values to be equal, and is used by <code>==</code>.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.ne\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/nightly/src/core/cmp.rs.html#261\">Source</a></span><a href=\"#method.ne\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialEq.html#method.ne\" class=\"fn\">ne</a>(&amp;self, other: <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.reference.html\">&amp;Rhs</a>) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>Tests for <code>!=</code>. The default implementation is almost always sufficient,\nand should not be overridden without very good reason.</div></details></div></details>","PartialEq<DomPair<KeyOther, ValOther>>","gossip_kv::model::RowValue"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-PartialOrd%3CDomPair%3CKeyOther,+ValOther%3E%3E-for-DomPair%3CKeySelf,+ValSelf%3E\" class=\"impl\"><a href=\"#impl-PartialOrd%3CDomPair%3CKeyOther,+ValOther%3E%3E-for-DomPair%3CKeySelf,+ValSelf%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;KeySelf, KeyOther, ValSelf, ValOther&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialOrd.html\" title=\"trait core::cmp::PartialOrd\">PartialOrd</a>&lt;DomPair&lt;KeyOther, ValOther&gt;&gt; for DomPair&lt;KeySelf, ValSelf&gt;<div class=\"where\">where\n    KeySelf: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialOrd.html\" title=\"trait core::cmp::PartialOrd\">PartialOrd</a>&lt;KeyOther&gt;,\n    ValSelf: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialOrd.html\" title=\"trait core::cmp::PartialOrd\">PartialOrd</a>&lt;ValOther&gt;,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.partial_cmp\" class=\"method trait-impl\"><a href=\"#method.partial_cmp\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialOrd.html#tymethod.partial_cmp\" class=\"fn\">partial_cmp</a>(&amp;self, other: &amp;DomPair&lt;KeyOther, ValOther&gt;) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/nightly/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/nightly/core/cmp/enum.Ordering.html\" title=\"enum core::cmp::Ordering\">Ordering</a>&gt;</h4></section></summary><div class='docblock'>This method returns an ordering between <code>self</code> and <code>other</code> values if one exists. <a href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialOrd.html#tymethod.partial_cmp\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.lt\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/nightly/src/core/cmp.rs.html#1381\">Source</a></span><a href=\"#method.lt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialOrd.html#method.lt\" class=\"fn\">lt</a>(&amp;self, other: <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.reference.html\">&amp;Rhs</a>) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>Tests less than (for <code>self</code> and <code>other</code>) and is used by the <code>&lt;</code> operator. <a href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialOrd.html#method.lt\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.le\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/nightly/src/core/cmp.rs.html#1399\">Source</a></span><a href=\"#method.le\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialOrd.html#method.le\" class=\"fn\">le</a>(&amp;self, other: <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.reference.html\">&amp;Rhs</a>) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>Tests less than or equal to (for <code>self</code> and <code>other</code>) and is used by the\n<code>&lt;=</code> operator. <a href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialOrd.html#method.le\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.gt\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/nightly/src/core/cmp.rs.html#1417\">Source</a></span><a href=\"#method.gt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialOrd.html#method.gt\" class=\"fn\">gt</a>(&amp;self, other: <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.reference.html\">&amp;Rhs</a>) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>Tests greater than (for <code>self</code> and <code>other</code>) and is used by the <code>&gt;</code>\noperator. <a href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialOrd.html#method.gt\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.ge\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/nightly/src/core/cmp.rs.html#1435\">Source</a></span><a href=\"#method.ge\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialOrd.html#method.ge\" class=\"fn\">ge</a>(&amp;self, other: <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.reference.html\">&amp;Rhs</a>) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>Tests greater than or equal to (for <code>self</code> and <code>other</code>) and is used by\nthe <code>&gt;=</code> operator. <a href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialOrd.html#method.ge\">Read more</a></div></details></div></details>","PartialOrd<DomPair<KeyOther, ValOther>>","gossip_kv::model::RowValue"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Serialize-for-DomPair%3CKey,+Val%3E\" class=\"impl\"><a href=\"#impl-Serialize-for-DomPair%3CKey,+Val%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Key, Val&gt; <a class=\"trait\" href=\"https://docs.rs/serde/1.0.210/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for DomPair&lt;Key, Val&gt;<div class=\"where\">where\n    Key: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.210/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a>,\n    Val: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.210/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a>,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.serialize\" class=\"method trait-impl\"><a href=\"#method.serialize\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://docs.rs/serde/1.0.210/serde/ser/trait.Serialize.html#tymethod.serialize\" class=\"fn\">serialize</a>&lt;__S&gt;(\n    &amp;self,\n    __serializer: __S,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/nightly/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;&lt;__S as <a class=\"trait\" href=\"https://docs.rs/serde/1.0.210/serde/ser/trait.Serializer.html\" title=\"trait serde::ser::Serializer\">Serializer</a>&gt;::<a class=\"associatedtype\" href=\"https://docs.rs/serde/1.0.210/serde/ser/trait.Serializer.html#associatedtype.Ok\" title=\"type serde::ser::Serializer::Ok\">Ok</a>, &lt;__S as <a class=\"trait\" href=\"https://docs.rs/serde/1.0.210/serde/ser/trait.Serializer.html\" title=\"trait serde::ser::Serializer\">Serializer</a>&gt;::<a class=\"associatedtype\" href=\"https://docs.rs/serde/1.0.210/serde/ser/trait.Serializer.html#associatedtype.Error\" title=\"type serde::ser::Serializer::Error\">Error</a>&gt;<div class=\"where\">where\n    __S: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.210/serde/ser/trait.Serializer.html\" title=\"trait serde::ser::Serializer\">Serializer</a>,</div></h4></section></summary><div class='docblock'>Serialize this value into the given Serde serializer. <a href=\"https://docs.rs/serde/1.0.210/serde/ser/trait.Serialize.html#tymethod.serialize\">Read more</a></div></details></div></details>","Serialize","gossip_kv::model::RowValue"],["<section id=\"impl-Copy-for-DomPair%3CKey,+Val%3E\" class=\"impl\"><a href=\"#impl-Copy-for-DomPair%3CKey,+Val%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Key, Val&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Copy.html\" title=\"trait core::marker::Copy\">Copy</a> for DomPair&lt;Key, Val&gt;<div class=\"where\">where\n    Key: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Copy.html\" title=\"trait core::marker::Copy\">Copy</a>,\n    Val: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Copy.html\" title=\"trait core::marker::Copy\">Copy</a>,</div></h3></section>","Copy","gossip_kv::model::RowValue"],["<section id=\"impl-Eq-for-DomPair%3CKey,+Val%3E\" class=\"impl\"><a href=\"#impl-Eq-for-DomPair%3CKey,+Val%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Key, Val&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.Eq.html\" title=\"trait core::cmp::Eq\">Eq</a> for DomPair&lt;Key, Val&gt;<div class=\"where\">where\n    Key: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.Eq.html\" title=\"trait core::cmp::Eq\">Eq</a>,\n    Val: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.Eq.html\" title=\"trait core::cmp::Eq\">Eq</a>,</div></h3></section>","Eq","gossip_kv::model::RowValue"],["<section id=\"impl-LatticeOrd%3CDomPair%3CKeyOther,+ValOther%3E%3E-for-DomPair%3CKeySelf,+ValSelf%3E\" class=\"impl\"><a href=\"#impl-LatticeOrd%3CDomPair%3CKeyOther,+ValOther%3E%3E-for-DomPair%3CKeySelf,+ValSelf%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;KeySelf, KeyOther, ValSelf, ValOther&gt; LatticeOrd&lt;DomPair&lt;KeyOther, ValOther&gt;&gt; for DomPair&lt;KeySelf, ValSelf&gt;<div class=\"where\">where\n    DomPair&lt;KeySelf, ValSelf&gt;: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialOrd.html\" title=\"trait core::cmp::PartialOrd\">PartialOrd</a>&lt;DomPair&lt;KeyOther, ValOther&gt;&gt;,</div></h3></section>","LatticeOrd<DomPair<KeyOther, ValOther>>","gossip_kv::model::RowValue"]]]]);
    if (window.register_type_impls) {
        window.register_type_impls(type_impls);
    } else {
        window.pending_type_impls = type_impls;
    }
})()
//{"start":55,"fragment_lengths":[30426]}