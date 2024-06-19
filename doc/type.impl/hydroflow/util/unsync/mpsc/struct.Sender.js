(function() {var type_impls = {
"hydroflow":[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Clone-for-Sender%3CT%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/hydroflow/util/unsync/mpsc.rs.html#82-88\">source</a><a href=\"#impl-Clone-for-Sender%3CT%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;T&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> for <a class=\"struct\" href=\"hydroflow/util/unsync/mpsc/struct.Sender.html\" title=\"struct hydroflow::util::unsync::mpsc::Sender\">Sender</a>&lt;T&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hydroflow/util/unsync/mpsc.rs.html#83-87\">source</a><a href=\"#method.clone\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html#tymethod.clone\" class=\"fn\">clone</a>(&amp;self) -&gt; Self</h4></section></summary><div class='docblock'>Returns a copy of the value. <a href=\"https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html#tymethod.clone\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone_from\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/nightly/src/core/clone.rs.html#169\">source</a></span><a href=\"#method.clone_from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html#method.clone_from\" class=\"fn\">clone_from</a>(&amp;mut self, source: <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.reference.html\">&amp;Self</a>)</h4></section></summary><div class='docblock'>Performs copy-assignment from <code>source</code>. <a href=\"https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html#method.clone_from\">Read more</a></div></details></div></details>","Clone","hydroflow::util::tcp::TcpFramedSink"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Drop-for-Sender%3CT%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/hydroflow/util/unsync/mpsc.rs.html#89-97\">source</a><a href=\"#impl-Drop-for-Sender%3CT%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;T&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/ops/drop/trait.Drop.html\" title=\"trait core::ops::drop::Drop\">Drop</a> for <a class=\"struct\" href=\"hydroflow/util/unsync/mpsc/struct.Sender.html\" title=\"struct hydroflow::util::unsync::mpsc::Sender\">Sender</a>&lt;T&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.drop\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hydroflow/util/unsync/mpsc.rs.html#90-96\">source</a><a href=\"#method.drop\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/nightly/core/ops/drop/trait.Drop.html#tymethod.drop\" class=\"fn\">drop</a>(&amp;mut self)</h4></section></summary><div class='docblock'>Executes the destructor for this type. <a href=\"https://doc.rust-lang.org/nightly/core/ops/drop/trait.Drop.html#tymethod.drop\">Read more</a></div></details></div></details>","Drop","hydroflow::util::tcp::TcpFramedSink"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Sender%3CT%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/hydroflow/util/unsync/mpsc.rs.html#19-81\">source</a><a href=\"#impl-Sender%3CT%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;T&gt; <a class=\"struct\" href=\"hydroflow/util/unsync/mpsc/struct.Sender.html\" title=\"struct hydroflow::util::unsync::mpsc::Sender\">Sender</a>&lt;T&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.send\" class=\"method\"><a class=\"src rightside\" href=\"src/hydroflow/util/unsync/mpsc.rs.html#21-44\">source</a><h4 class=\"code-header\">pub async fn <a href=\"hydroflow/util/unsync/mpsc/struct.Sender.html#tymethod.send\" class=\"fn\">send</a>(&amp;self, item: T) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/nightly/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.unit.html\">()</a>, <a class=\"struct\" href=\"hydroflow/util/unsync/mpsc/struct.SendError.html\" title=\"struct hydroflow::util::unsync::mpsc::SendError\">SendError</a>&lt;T&gt;&gt;</h4></section></summary><div class=\"docblock\"><p>Asynchronously sends value to the receiver.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.try_send\" class=\"method\"><a class=\"src rightside\" href=\"src/hydroflow/util/unsync/mpsc.rs.html#51-67\">source</a><h4 class=\"code-header\">pub fn <a href=\"hydroflow/util/unsync/mpsc/struct.Sender.html#tymethod.try_send\" class=\"fn\">try_send</a>(&amp;self, item: T) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/nightly/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.unit.html\">()</a>, <a class=\"enum\" href=\"hydroflow/util/unsync/mpsc/enum.TrySendError.html\" title=\"enum hydroflow::util::unsync::mpsc::TrySendError\">TrySendError</a>&lt;T&gt;&gt;</h4></section></summary><div class=\"docblock\"><p>Tries to send the value to the receiver without blocking.</p>\n<p>Returns an error if the destination is closed or if the buffer is at capacity.</p>\n<p><a href=\"hydroflow/util/unsync/mpsc/enum.TrySendError.html#variant.Full\" title=\"variant hydroflow::util::unsync::mpsc::TrySendError::Full\"><code>TrySendError::Full</code></a> will never be returned if this is an unbounded channel.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.close_this_sender\" class=\"method\"><a class=\"src rightside\" href=\"src/hydroflow/util/unsync/mpsc.rs.html#73-75\">source</a><h4 class=\"code-header\">pub fn <a href=\"hydroflow/util/unsync/mpsc/struct.Sender.html#tymethod.close_this_sender\" class=\"fn\">close_this_sender</a>(&amp;mut self)</h4></section></summary><div class=\"docblock\"><p>Close this sender. No more messages can be sent from this sender.</p>\n<p>Note that this only closes the channel from the view-point of this sender. The channel\nremains open until all senders have gone away, or until the <a href=\"hydroflow/util/unsync/mpsc/struct.Receiver.html\" title=\"struct hydroflow::util::unsync::mpsc::Receiver\"><code>Receiver</code></a> closes the channel.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.is_closed\" class=\"method\"><a class=\"src rightside\" href=\"src/hydroflow/util/unsync/mpsc.rs.html#78-80\">source</a><h4 class=\"code-header\">pub fn <a href=\"hydroflow/util/unsync/mpsc/struct.Sender.html#tymethod.is_closed\" class=\"fn\">is_closed</a>(&amp;self) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.bool.html\">bool</a></h4></section></summary><div class=\"docblock\"><p>If this sender or the corresponding <a href=\"hydroflow/util/unsync/mpsc/struct.Receiver.html\" title=\"struct hydroflow::util::unsync::mpsc::Receiver\"><code>Receiver</code></a> is closed.</p>\n</div></details></div></details>",0,"hydroflow::util::tcp::TcpFramedSink"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Sink%3CT%3E-for-Sender%3CT%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/hydroflow/util/unsync/mpsc.rs.html#99-141\">source</a><a href=\"#impl-Sink%3CT%3E-for-Sender%3CT%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;T&gt; Sink&lt;T&gt; for <a class=\"struct\" href=\"hydroflow/util/unsync/mpsc/struct.Sender.html\" title=\"struct hydroflow::util::unsync::mpsc::Sender\">Sender</a>&lt;T&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle\" open><summary><section id=\"associatedtype.Error\" class=\"associatedtype trait-impl\"><a href=\"#associatedtype.Error\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a class=\"associatedtype\">Error</a> = <a class=\"enum\" href=\"hydroflow/util/unsync/mpsc/enum.TrySendError.html\" title=\"enum hydroflow::util::unsync::mpsc::TrySendError\">TrySendError</a>&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/nightly/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;T&gt;&gt;</h4></section></summary><div class='docblock'>The type of value produced by the sink when an error occurs.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.poll_ready\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hydroflow/util/unsync/mpsc.rs.html#102-120\">source</a><a href=\"#method.poll_ready\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">poll_ready</a>(\n    self: <a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.reference.html\">&amp;mut Self</a>&gt;,\n    ctx: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/core/task/wake/struct.Context.html\" title=\"struct core::task::wake::Context\">Context</a>&lt;'_&gt;,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/nightly/core/task/poll/enum.Poll.html\" title=\"enum core::task::poll::Poll\">Poll</a>&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/nightly/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.unit.html\">()</a>, Self::Error&gt;&gt;</h4></section></summary><div class='docblock'>Attempts to prepare the <code>Sink</code> to receive a value. <a>Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.start_send\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hydroflow/util/unsync/mpsc.rs.html#122-127\">source</a><a href=\"#method.start_send\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">start_send</a>(self: <a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.reference.html\">&amp;mut Self</a>&gt;, item: T) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/nightly/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.unit.html\">()</a>, Self::Error&gt;</h4></section></summary><div class='docblock'>Begin the process of sending a value to the sink.\nEach call to this function must be preceded by a successful call to\n<code>poll_ready</code> which returned <code>Poll::Ready(Ok(()))</code>. <a>Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.poll_flush\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hydroflow/util/unsync/mpsc.rs.html#129-131\">source</a><a href=\"#method.poll_flush\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">poll_flush</a>(\n    self: <a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.reference.html\">&amp;mut Self</a>&gt;,\n    _ctx: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/core/task/wake/struct.Context.html\" title=\"struct core::task::wake::Context\">Context</a>&lt;'_&gt;,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/nightly/core/task/poll/enum.Poll.html\" title=\"enum core::task::poll::Poll\">Poll</a>&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/nightly/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.unit.html\">()</a>, Self::Error&gt;&gt;</h4></section></summary><div class='docblock'>Flush any remaining output from this sink. <a>Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.poll_close\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hydroflow/util/unsync/mpsc.rs.html#133-140\">source</a><a href=\"#method.poll_close\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">poll_close</a>(\n    self: <a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.reference.html\">&amp;mut Self</a>&gt;,\n    ctx: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/core/task/wake/struct.Context.html\" title=\"struct core::task::wake::Context\">Context</a>&lt;'_&gt;,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/nightly/core/task/poll/enum.Poll.html\" title=\"enum core::task::poll::Poll\">Poll</a>&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/nightly/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.unit.html\">()</a>, Self::Error&gt;&gt;</h4></section></summary><div class='docblock'>Flush any remaining output and close this sink, if necessary. <a>Read more</a></div></details></div></details>","Sink<T>","hydroflow::util::tcp::TcpFramedSink"]]
};if (window.register_type_impls) {window.register_type_impls(type_impls);} else {window.pending_type_impls = type_impls;}})()