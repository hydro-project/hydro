(function() {
    var type_impls = Object.fromEntries([["dfir_rs",[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Clone-for-Key%3CTag%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/dfir_rs/util/slot_vec.rs.html#24-28\">Source</a><a href=\"#impl-Clone-for-Key%3CTag%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Tag: ?<a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> for <a class=\"struct\" href=\"dfir_rs/util/slot_vec/struct.Key.html\" title=\"struct dfir_rs::util::slot_vec::Key\">Key</a>&lt;Tag&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/dfir_rs/util/slot_vec.rs.html#25-27\">Source</a><a href=\"#method.clone\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html#tymethod.clone\" class=\"fn\">clone</a>(&amp;self) -&gt; Self</h4></section></summary><div class='docblock'>Returns a copy of the value. <a href=\"https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html#tymethod.clone\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone_from\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/nightly/src/core/clone.rs.html#174\">Source</a></span><a href=\"#method.clone_from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html#method.clone_from\" class=\"fn\">clone_from</a>(&amp;mut self, source: &amp;Self)</h4></section></summary><div class='docblock'>Performs copy-assignment from <code>source</code>. <a href=\"https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html#method.clone_from\">Read more</a></div></details></div></details>","Clone","dfir_rs::scheduled::SubgraphId","dfir_rs::scheduled::HandoffId","dfir_rs::scheduled::StateId","dfir_rs::scheduled::LoopId"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Debug-for-Key%3CTag%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/dfir_rs/util/slot_vec.rs.html#51-55\">Source</a><a href=\"#impl-Debug-for-Key%3CTag%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Tag: ?<a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> for <a class=\"struct\" href=\"dfir_rs/util/slot_vec/struct.Key.html\" title=\"struct dfir_rs::util::slot_vec::Key\">Key</a>&lt;Tag&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fmt\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/dfir_rs/util/slot_vec.rs.html#52-54\">Source</a><a href=\"#method.fmt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/fmt/trait.Debug.html#tymethod.fmt\" class=\"fn\">fmt</a>(&amp;self, f: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/core/fmt/struct.Formatter.html\" title=\"struct core::fmt::Formatter\">Formatter</a>&lt;'_&gt;) -&gt; <a class=\"type\" href=\"https://doc.rust-lang.org/nightly/core/fmt/type.Result.html\" title=\"type core::fmt::Result\">Result</a></h4></section></summary><div class='docblock'>Formats the value using the given formatter. <a href=\"https://doc.rust-lang.org/nightly/core/fmt/trait.Debug.html#tymethod.fmt\">Read more</a></div></details></div></details>","Debug","dfir_rs::scheduled::SubgraphId","dfir_rs::scheduled::HandoffId","dfir_rs::scheduled::StateId","dfir_rs::scheduled::LoopId"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Display-for-Key%3CTag%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/dfir_rs/util/slot_vec.rs.html#56-60\">Source</a><a href=\"#impl-Display-for-Key%3CTag%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Tag: ?<a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/fmt/trait.Display.html\" title=\"trait core::fmt::Display\">Display</a> for <a class=\"struct\" href=\"dfir_rs/util/slot_vec/struct.Key.html\" title=\"struct dfir_rs::util::slot_vec::Key\">Key</a>&lt;Tag&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fmt\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/dfir_rs/util/slot_vec.rs.html#57-59\">Source</a><a href=\"#method.fmt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/fmt/trait.Display.html#tymethod.fmt\" class=\"fn\">fmt</a>(&amp;self, f: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/core/fmt/struct.Formatter.html\" title=\"struct core::fmt::Formatter\">Formatter</a>&lt;'_&gt;) -&gt; <a class=\"type\" href=\"https://doc.rust-lang.org/nightly/core/fmt/type.Result.html\" title=\"type core::fmt::Result\">Result</a></h4></section></summary><div class='docblock'>Formats the value using the given formatter. <a href=\"https://doc.rust-lang.org/nightly/core/fmt/trait.Display.html#tymethod.fmt\">Read more</a></div></details></div></details>","Display","dfir_rs::scheduled::SubgraphId","dfir_rs::scheduled::HandoffId","dfir_rs::scheduled::StateId","dfir_rs::scheduled::LoopId"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Hash-for-Key%3CTag%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/dfir_rs/util/slot_vec.rs.html#46-50\">Source</a><a href=\"#impl-Hash-for-Key%3CTag%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Tag: ?<a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/hash/trait.Hash.html\" title=\"trait core::hash::Hash\">Hash</a> for <a class=\"struct\" href=\"dfir_rs/util/slot_vec/struct.Key.html\" title=\"struct dfir_rs::util::slot_vec::Key\">Key</a>&lt;Tag&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.hash\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/dfir_rs/util/slot_vec.rs.html#47-49\">Source</a><a href=\"#method.hash\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/hash/trait.Hash.html#tymethod.hash\" class=\"fn\">hash</a>&lt;H: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/hash/trait.Hasher.html\" title=\"trait core::hash::Hasher\">Hasher</a>&gt;(&amp;self, state: <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.reference.html\">&amp;mut H</a>)</h4></section></summary><div class='docblock'>Feeds this value into the given <a href=\"https://doc.rust-lang.org/nightly/core/hash/trait.Hasher.html\" title=\"trait core::hash::Hasher\"><code>Hasher</code></a>. <a href=\"https://doc.rust-lang.org/nightly/core/hash/trait.Hash.html#tymethod.hash\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.hash_slice\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.3.0\">1.3.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/nightly/src/core/hash/mod.rs.html#235-237\">Source</a></span><a href=\"#method.hash_slice\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/hash/trait.Hash.html#method.hash_slice\" class=\"fn\">hash_slice</a>&lt;H&gt;(data: &amp;[Self], state: <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.reference.html\">&amp;mut H</a>)<div class=\"where\">where\n    H: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/hash/trait.Hasher.html\" title=\"trait core::hash::Hasher\">Hasher</a>,\n    Self: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>,</div></h4></section></summary><div class='docblock'>Feeds a slice of this type into the given <a href=\"https://doc.rust-lang.org/nightly/core/hash/trait.Hasher.html\" title=\"trait core::hash::Hasher\"><code>Hasher</code></a>. <a href=\"https://doc.rust-lang.org/nightly/core/hash/trait.Hash.html#method.hash_slice\">Read more</a></div></details></div></details>","Hash","dfir_rs::scheduled::SubgraphId","dfir_rs::scheduled::HandoffId","dfir_rs::scheduled::StateId","dfir_rs::scheduled::LoopId"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Key%3CTag%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/dfir_rs/util/slot_vec.rs.html#15-23\">Source</a><a href=\"#impl-Key%3CTag%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Tag: ?<a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>&gt; <a class=\"struct\" href=\"dfir_rs/util/slot_vec/struct.Key.html\" title=\"struct dfir_rs::util::slot_vec::Key\">Key</a>&lt;Tag&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.from_raw\" class=\"method\"><a class=\"src rightside\" href=\"src/dfir_rs/util/slot_vec.rs.html#17-22\">Source</a><h4 class=\"code-header\">pub fn <a href=\"dfir_rs/util/slot_vec/struct.Key.html#tymethod.from_raw\" class=\"fn\">from_raw</a>(index: <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.usize.html\">usize</a>) -&gt; Self</h4></section></summary><div class=\"docblock\"><p>Creates a Key from a raw index. Avoid using this function directly.</p>\n</div></details></div></details>",0,"dfir_rs::scheduled::SubgraphId","dfir_rs::scheduled::HandoffId","dfir_rs::scheduled::StateId","dfir_rs::scheduled::LoopId"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Ord-for-Key%3CTag%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/dfir_rs/util/slot_vec.rs.html#35-39\">Source</a><a href=\"#impl-Ord-for-Key%3CTag%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Tag: ?<a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.Ord.html\" title=\"trait core::cmp::Ord\">Ord</a> for <a class=\"struct\" href=\"dfir_rs/util/slot_vec/struct.Key.html\" title=\"struct dfir_rs::util::slot_vec::Key\">Key</a>&lt;Tag&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.cmp\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/dfir_rs/util/slot_vec.rs.html#36-38\">Source</a><a href=\"#method.cmp\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.Ord.html#tymethod.cmp\" class=\"fn\">cmp</a>(&amp;self, other: &amp;Self) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/nightly/core/cmp/enum.Ordering.html\" title=\"enum core::cmp::Ordering\">Ordering</a></h4></section></summary><div class='docblock'>This method returns an <a href=\"https://doc.rust-lang.org/nightly/core/cmp/enum.Ordering.html\" title=\"enum core::cmp::Ordering\"><code>Ordering</code></a> between <code>self</code> and <code>other</code>. <a href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.Ord.html#tymethod.cmp\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.max\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.21.0\">1.21.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/nightly/src/core/cmp.rs.html#1008-1010\">Source</a></span><a href=\"#method.max\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.Ord.html#method.max\" class=\"fn\">max</a>(self, other: Self) -&gt; Self<div class=\"where\">where\n    Self: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>,</div></h4></section></summary><div class='docblock'>Compares and returns the maximum of two values. <a href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.Ord.html#method.max\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.min\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.21.0\">1.21.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/nightly/src/core/cmp.rs.html#1047-1049\">Source</a></span><a href=\"#method.min\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.Ord.html#method.min\" class=\"fn\">min</a>(self, other: Self) -&gt; Self<div class=\"where\">where\n    Self: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>,</div></h4></section></summary><div class='docblock'>Compares and returns the minimum of two values. <a href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.Ord.html#method.min\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.clamp\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.50.0\">1.50.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/nightly/src/core/cmp.rs.html#1073-1075\">Source</a></span><a href=\"#method.clamp\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.Ord.html#method.clamp\" class=\"fn\">clamp</a>(self, min: Self, max: Self) -&gt; Self<div class=\"where\">where\n    Self: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>,</div></h4></section></summary><div class='docblock'>Restrict a value to a certain interval. <a href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.Ord.html#method.clamp\">Read more</a></div></details></div></details>","Ord","dfir_rs::scheduled::SubgraphId","dfir_rs::scheduled::HandoffId","dfir_rs::scheduled::StateId","dfir_rs::scheduled::LoopId"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-PartialEq-for-Key%3CTag%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/dfir_rs/util/slot_vec.rs.html#40-44\">Source</a><a href=\"#impl-PartialEq-for-Key%3CTag%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Tag: ?<a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialEq.html\" title=\"trait core::cmp::PartialEq\">PartialEq</a> for <a class=\"struct\" href=\"dfir_rs/util/slot_vec/struct.Key.html\" title=\"struct dfir_rs::util::slot_vec::Key\">Key</a>&lt;Tag&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.eq\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/dfir_rs/util/slot_vec.rs.html#41-43\">Source</a><a href=\"#method.eq\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialEq.html#tymethod.eq\" class=\"fn\">eq</a>(&amp;self, other: &amp;Self) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>Tests for <code>self</code> and <code>other</code> values to be equal, and is used by <code>==</code>.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.ne\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/nightly/src/core/cmp.rs.html#261\">Source</a></span><a href=\"#method.ne\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialEq.html#method.ne\" class=\"fn\">ne</a>(&amp;self, other: <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.reference.html\">&amp;Rhs</a>) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>Tests for <code>!=</code>. The default implementation is almost always sufficient,\nand should not be overridden without very good reason.</div></details></div></details>","PartialEq","dfir_rs::scheduled::SubgraphId","dfir_rs::scheduled::HandoffId","dfir_rs::scheduled::StateId","dfir_rs::scheduled::LoopId"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-PartialOrd-for-Key%3CTag%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/dfir_rs/util/slot_vec.rs.html#30-34\">Source</a><a href=\"#impl-PartialOrd-for-Key%3CTag%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Tag: ?<a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialOrd.html\" title=\"trait core::cmp::PartialOrd\">PartialOrd</a> for <a class=\"struct\" href=\"dfir_rs/util/slot_vec/struct.Key.html\" title=\"struct dfir_rs::util::slot_vec::Key\">Key</a>&lt;Tag&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.partial_cmp\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/dfir_rs/util/slot_vec.rs.html#31-33\">Source</a><a href=\"#method.partial_cmp\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialOrd.html#tymethod.partial_cmp\" class=\"fn\">partial_cmp</a>(&amp;self, other: &amp;Self) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/nightly/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/nightly/core/cmp/enum.Ordering.html\" title=\"enum core::cmp::Ordering\">Ordering</a>&gt;</h4></section></summary><div class='docblock'>This method returns an ordering between <code>self</code> and <code>other</code> values if one exists. <a href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialOrd.html#tymethod.partial_cmp\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.lt\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/nightly/src/core/cmp.rs.html#1381\">Source</a></span><a href=\"#method.lt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialOrd.html#method.lt\" class=\"fn\">lt</a>(&amp;self, other: <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.reference.html\">&amp;Rhs</a>) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>Tests less than (for <code>self</code> and <code>other</code>) and is used by the <code>&lt;</code> operator. <a href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialOrd.html#method.lt\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.le\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/nightly/src/core/cmp.rs.html#1399\">Source</a></span><a href=\"#method.le\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialOrd.html#method.le\" class=\"fn\">le</a>(&amp;self, other: <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.reference.html\">&amp;Rhs</a>) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>Tests less than or equal to (for <code>self</code> and <code>other</code>) and is used by the\n<code>&lt;=</code> operator. <a href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialOrd.html#method.le\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.gt\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/nightly/src/core/cmp.rs.html#1417\">Source</a></span><a href=\"#method.gt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialOrd.html#method.gt\" class=\"fn\">gt</a>(&amp;self, other: <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.reference.html\">&amp;Rhs</a>) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>Tests greater than (for <code>self</code> and <code>other</code>) and is used by the <code>&gt;</code>\noperator. <a href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialOrd.html#method.gt\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.ge\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/nightly/src/core/cmp.rs.html#1435\">Source</a></span><a href=\"#method.ge\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialOrd.html#method.ge\" class=\"fn\">ge</a>(&amp;self, other: <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.reference.html\">&amp;Rhs</a>) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>Tests greater than or equal to (for <code>self</code> and <code>other</code>) and is used by\nthe <code>&gt;=</code> operator. <a href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialOrd.html#method.ge\">Read more</a></div></details></div></details>","PartialOrd","dfir_rs::scheduled::SubgraphId","dfir_rs::scheduled::HandoffId","dfir_rs::scheduled::StateId","dfir_rs::scheduled::LoopId"],["<section id=\"impl-Copy-for-Key%3CTag%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/dfir_rs/util/slot_vec.rs.html#29\">Source</a><a href=\"#impl-Copy-for-Key%3CTag%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Tag: ?<a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Copy.html\" title=\"trait core::marker::Copy\">Copy</a> for <a class=\"struct\" href=\"dfir_rs/util/slot_vec/struct.Key.html\" title=\"struct dfir_rs::util::slot_vec::Key\">Key</a>&lt;Tag&gt;</h3></section>","Copy","dfir_rs::scheduled::SubgraphId","dfir_rs::scheduled::HandoffId","dfir_rs::scheduled::StateId","dfir_rs::scheduled::LoopId"],["<section id=\"impl-Eq-for-Key%3CTag%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/dfir_rs/util/slot_vec.rs.html#45\">Source</a><a href=\"#impl-Eq-for-Key%3CTag%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Tag: ?<a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.Eq.html\" title=\"trait core::cmp::Eq\">Eq</a> for <a class=\"struct\" href=\"dfir_rs/util/slot_vec/struct.Key.html\" title=\"struct dfir_rs::util::slot_vec::Key\">Key</a>&lt;Tag&gt;</h3></section>","Eq","dfir_rs::scheduled::SubgraphId","dfir_rs::scheduled::HandoffId","dfir_rs::scheduled::StateId","dfir_rs::scheduled::LoopId"]]]]);
    if (window.register_type_impls) {
        window.register_type_impls(type_impls);
    } else {
        window.pending_type_impls = type_impls;
    }
})()
//{"start":55,"fragment_lengths":[25284]}