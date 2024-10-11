(function() {
    var type_impls = Object.fromEntries([["hydroflow_plus",[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Debug-for-SplitSink%3CS,+Item%3E\" class=\"impl\"><a href=\"#impl-Debug-for-SplitSink%3CS,+Item%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;S, Item&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> for <a class=\"struct\" href=\"hydroflow_plus/futures/prelude/stream/struct.SplitSink.html\" title=\"struct hydroflow_plus::futures::prelude::stream::SplitSink\">SplitSink</a>&lt;S, Item&gt;<div class=\"where\">where\n    S: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a>,\n    Item: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a>,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fmt\" class=\"method trait-impl\"><a href=\"#method.fmt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/fmt/trait.Debug.html#tymethod.fmt\" class=\"fn\">fmt</a>(&amp;self, f: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/core/fmt/struct.Formatter.html\" title=\"struct core::fmt::Formatter\">Formatter</a>&lt;'_&gt;) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/nightly/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.unit.html\">()</a>, <a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/core/fmt/struct.Error.html\" title=\"struct core::fmt::Error\">Error</a>&gt;</h4></section></summary><div class='docblock'>Formats the value using the given formatter. <a href=\"https://doc.rust-lang.org/nightly/core/fmt/trait.Debug.html#tymethod.fmt\">Read more</a></div></details></div></details>","Debug","hydroflow_plus::util::UdpFramedSink","hydroflow_plus::util::UdpSink","hydroflow_plus::util::UdpBytesSink","hydroflow_plus::util::UdpLinesSink"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Sink%3CItem%3E-for-SplitSink%3CS,+Item%3E\" class=\"impl\"><a href=\"#impl-Sink%3CItem%3E-for-SplitSink%3CS,+Item%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;S, Item&gt; <a class=\"trait\" href=\"hydroflow_plus/futures/trait.Sink.html\" title=\"trait hydroflow_plus::futures::Sink\">Sink</a>&lt;Item&gt; for <a class=\"struct\" href=\"hydroflow_plus/futures/prelude/stream/struct.SplitSink.html\" title=\"struct hydroflow_plus::futures::prelude::stream::SplitSink\">SplitSink</a>&lt;S, Item&gt;<div class=\"where\">where\n    S: <a class=\"trait\" href=\"hydroflow_plus/futures/trait.Sink.html\" title=\"trait hydroflow_plus::futures::Sink\">Sink</a>&lt;Item&gt;,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle\" open><summary><section id=\"associatedtype.Error\" class=\"associatedtype trait-impl\"><a href=\"#associatedtype.Error\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a href=\"hydroflow_plus/futures/trait.Sink.html#associatedtype.Error\" class=\"associatedtype\">Error</a> = &lt;S as <a class=\"trait\" href=\"hydroflow_plus/futures/trait.Sink.html\" title=\"trait hydroflow_plus::futures::Sink\">Sink</a>&lt;Item&gt;&gt;::<a class=\"associatedtype\" href=\"hydroflow_plus/futures/trait.Sink.html#associatedtype.Error\" title=\"type hydroflow_plus::futures::Sink::Error\">Error</a></h4></section></summary><div class='docblock'>The type of value produced by the sink when an error occurs.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.poll_ready\" class=\"method trait-impl\"><a href=\"#method.poll_ready\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"hydroflow_plus/futures/trait.Sink.html#tymethod.poll_ready\" class=\"fn\">poll_ready</a>(\n    self: <a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;&amp;mut <a class=\"struct\" href=\"hydroflow_plus/futures/prelude/stream/struct.SplitSink.html\" title=\"struct hydroflow_plus::futures::prelude::stream::SplitSink\">SplitSink</a>&lt;S, Item&gt;&gt;,\n    cx: &amp;mut <a class=\"struct\" href=\"hydroflow_plus/futures/task/struct.Context.html\" title=\"struct hydroflow_plus::futures::task::Context\">Context</a>&lt;'_&gt;,\n) -&gt; <a class=\"enum\" href=\"hydroflow_plus/futures/task/enum.Poll.html\" title=\"enum hydroflow_plus::futures::task::Poll\">Poll</a>&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/nightly/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.unit.html\">()</a>, &lt;S as <a class=\"trait\" href=\"hydroflow_plus/futures/trait.Sink.html\" title=\"trait hydroflow_plus::futures::Sink\">Sink</a>&lt;Item&gt;&gt;::<a class=\"associatedtype\" href=\"hydroflow_plus/futures/trait.Sink.html#associatedtype.Error\" title=\"type hydroflow_plus::futures::Sink::Error\">Error</a>&gt;&gt;</h4></section></summary><div class='docblock'>Attempts to prepare the <code>Sink</code> to receive a value. <a href=\"hydroflow_plus/futures/trait.Sink.html#tymethod.poll_ready\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.start_send\" class=\"method trait-impl\"><a href=\"#method.start_send\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"hydroflow_plus/futures/trait.Sink.html#tymethod.start_send\" class=\"fn\">start_send</a>(\n    self: <a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;&amp;mut <a class=\"struct\" href=\"hydroflow_plus/futures/prelude/stream/struct.SplitSink.html\" title=\"struct hydroflow_plus::futures::prelude::stream::SplitSink\">SplitSink</a>&lt;S, Item&gt;&gt;,\n    item: Item,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/nightly/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.unit.html\">()</a>, &lt;S as <a class=\"trait\" href=\"hydroflow_plus/futures/trait.Sink.html\" title=\"trait hydroflow_plus::futures::Sink\">Sink</a>&lt;Item&gt;&gt;::<a class=\"associatedtype\" href=\"hydroflow_plus/futures/trait.Sink.html#associatedtype.Error\" title=\"type hydroflow_plus::futures::Sink::Error\">Error</a>&gt;</h4></section></summary><div class='docblock'>Begin the process of sending a value to the sink.\nEach call to this function must be preceded by a successful call to\n<code>poll_ready</code> which returned <code>Poll::Ready(Ok(()))</code>. <a href=\"hydroflow_plus/futures/trait.Sink.html#tymethod.start_send\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.poll_flush\" class=\"method trait-impl\"><a href=\"#method.poll_flush\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"hydroflow_plus/futures/trait.Sink.html#tymethod.poll_flush\" class=\"fn\">poll_flush</a>(\n    self: <a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;&amp;mut <a class=\"struct\" href=\"hydroflow_plus/futures/prelude/stream/struct.SplitSink.html\" title=\"struct hydroflow_plus::futures::prelude::stream::SplitSink\">SplitSink</a>&lt;S, Item&gt;&gt;,\n    cx: &amp;mut <a class=\"struct\" href=\"hydroflow_plus/futures/task/struct.Context.html\" title=\"struct hydroflow_plus::futures::task::Context\">Context</a>&lt;'_&gt;,\n) -&gt; <a class=\"enum\" href=\"hydroflow_plus/futures/task/enum.Poll.html\" title=\"enum hydroflow_plus::futures::task::Poll\">Poll</a>&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/nightly/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.unit.html\">()</a>, &lt;S as <a class=\"trait\" href=\"hydroflow_plus/futures/trait.Sink.html\" title=\"trait hydroflow_plus::futures::Sink\">Sink</a>&lt;Item&gt;&gt;::<a class=\"associatedtype\" href=\"hydroflow_plus/futures/trait.Sink.html#associatedtype.Error\" title=\"type hydroflow_plus::futures::Sink::Error\">Error</a>&gt;&gt;</h4></section></summary><div class='docblock'>Flush any remaining output from this sink. <a href=\"hydroflow_plus/futures/trait.Sink.html#tymethod.poll_flush\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.poll_close\" class=\"method trait-impl\"><a href=\"#method.poll_close\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"hydroflow_plus/futures/trait.Sink.html#tymethod.poll_close\" class=\"fn\">poll_close</a>(\n    self: <a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;&amp;mut <a class=\"struct\" href=\"hydroflow_plus/futures/prelude/stream/struct.SplitSink.html\" title=\"struct hydroflow_plus::futures::prelude::stream::SplitSink\">SplitSink</a>&lt;S, Item&gt;&gt;,\n    cx: &amp;mut <a class=\"struct\" href=\"hydroflow_plus/futures/task/struct.Context.html\" title=\"struct hydroflow_plus::futures::task::Context\">Context</a>&lt;'_&gt;,\n) -&gt; <a class=\"enum\" href=\"hydroflow_plus/futures/task/enum.Poll.html\" title=\"enum hydroflow_plus::futures::task::Poll\">Poll</a>&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/nightly/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.unit.html\">()</a>, &lt;S as <a class=\"trait\" href=\"hydroflow_plus/futures/trait.Sink.html\" title=\"trait hydroflow_plus::futures::Sink\">Sink</a>&lt;Item&gt;&gt;::<a class=\"associatedtype\" href=\"hydroflow_plus/futures/trait.Sink.html#associatedtype.Error\" title=\"type hydroflow_plus::futures::Sink::Error\">Error</a>&gt;&gt;</h4></section></summary><div class='docblock'>Flush any remaining output and close this sink, if necessary. <a href=\"hydroflow_plus/futures/trait.Sink.html#tymethod.poll_close\">Read more</a></div></details></div></details>","Sink<Item>","hydroflow_plus::util::UdpFramedSink","hydroflow_plus::util::UdpSink","hydroflow_plus::util::UdpBytesSink","hydroflow_plus::util::UdpLinesSink"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-SplitSink%3CS,+Item%3E\" class=\"impl\"><a href=\"#impl-SplitSink%3CS,+Item%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;S, Item&gt; <a class=\"struct\" href=\"hydroflow_plus/futures/prelude/stream/struct.SplitSink.html\" title=\"struct hydroflow_plus::futures::prelude::stream::SplitSink\">SplitSink</a>&lt;S, Item&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.is_pair_of\" class=\"method\"><h4 class=\"code-header\">pub fn <a href=\"hydroflow_plus/futures/prelude/stream/struct.SplitSink.html#tymethod.is_pair_of\" class=\"fn\">is_pair_of</a>(&amp;self, other: &amp;<a class=\"struct\" href=\"hydroflow_plus/futures/prelude/stream/struct.SplitStream.html\" title=\"struct hydroflow_plus::futures::prelude::stream::SplitStream\">SplitStream</a>&lt;S&gt;) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.bool.html\">bool</a></h4></section></summary><div class=\"docblock\"><p>Returns <code>true</code> if the <code>SplitStream&lt;S&gt;</code> and <code>SplitSink&lt;S&gt;</code> originate from the same call to <code>StreamExt::split</code>.</p>\n</div></details></div></details>",0,"hydroflow_plus::util::UdpFramedSink","hydroflow_plus::util::UdpSink","hydroflow_plus::util::UdpBytesSink","hydroflow_plus::util::UdpLinesSink"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-SplitSink%3CS,+Item%3E\" class=\"impl\"><a href=\"#impl-SplitSink%3CS,+Item%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;S, Item&gt; <a class=\"struct\" href=\"hydroflow_plus/futures/prelude/stream/struct.SplitSink.html\" title=\"struct hydroflow_plus::futures::prelude::stream::SplitSink\">SplitSink</a>&lt;S, Item&gt;<div class=\"where\">where\n    S: <a class=\"trait\" href=\"hydroflow_plus/futures/trait.Sink.html\" title=\"trait hydroflow_plus::futures::Sink\">Sink</a>&lt;Item&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.reunite\" class=\"method\"><h4 class=\"code-header\">pub fn <a href=\"hydroflow_plus/futures/prelude/stream/struct.SplitSink.html#tymethod.reunite\" class=\"fn\">reunite</a>(self, other: <a class=\"struct\" href=\"hydroflow_plus/futures/prelude/stream/struct.SplitStream.html\" title=\"struct hydroflow_plus::futures::prelude::stream::SplitStream\">SplitStream</a>&lt;S&gt;) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/nightly/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;S, <a class=\"struct\" href=\"hydroflow_plus/futures/prelude/stream/struct.ReuniteError.html\" title=\"struct hydroflow_plus::futures::prelude::stream::ReuniteError\">ReuniteError</a>&lt;S, Item&gt;&gt;</h4></section></summary><div class=\"docblock\"><p>Attempts to put the two “halves” of a split <code>Stream + Sink</code> back\ntogether. Succeeds only if the <code>SplitStream&lt;S&gt;</code> and <code>SplitSink&lt;S&gt;</code> are\na matching pair originating from the same call to <code>StreamExt::split</code>.</p>\n</div></details></div></details>",0,"hydroflow_plus::util::UdpFramedSink","hydroflow_plus::util::UdpSink","hydroflow_plus::util::UdpBytesSink","hydroflow_plus::util::UdpLinesSink"],["<section id=\"impl-Unpin-for-SplitSink%3CS,+Item%3E\" class=\"impl\"><a href=\"#impl-Unpin-for-SplitSink%3CS,+Item%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;S, Item&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"hydroflow_plus/futures/prelude/stream/struct.SplitSink.html\" title=\"struct hydroflow_plus::futures::prelude::stream::SplitSink\">SplitSink</a>&lt;S, Item&gt;</h3></section>","Unpin","hydroflow_plus::util::UdpFramedSink","hydroflow_plus::util::UdpSink","hydroflow_plus::util::UdpBytesSink","hydroflow_plus::util::UdpLinesSink"]]]]);
    if (window.register_type_impls) {
        window.register_type_impls(type_impls);
    } else {
        window.pending_type_impls = type_impls;
    }
})()
//{"start":55,"fragment_lengths":[14834]}