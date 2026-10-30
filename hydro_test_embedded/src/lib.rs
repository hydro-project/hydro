#[cfg(feature = "test_embedded")]
#[expect(
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    reason = "generated code"
)]
#[allow(unused_imports, unused_qualifications, missing_docs, non_snake_case)]
pub mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded.rs"));
}

/// Same program as [`embedded`], generated with
/// [`EmbeddedCodegenOptions::with_flame_attribution`](hydro_lang::compile::embedded::EmbeddedCodegenOptions::with_flame_attribution).
#[cfg(feature = "test_embedded")]
#[expect(
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    reason = "generated code"
)]
#[allow(unused_imports, unused_qualifications, missing_docs, non_snake_case)]
pub mod embedded_flame_attribution {
    include!(concat!(env!("OUT_DIR"), "/embedded_flame_attribution.rs"));
}

#[cfg(feature = "test_embedded")]
#[expect(
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    reason = "generated code"
)]
#[allow(unused_imports, unused_qualifications, missing_docs, non_snake_case)]
pub mod unordered_output {
    include!(concat!(env!("OUT_DIR"), "/unordered_output.rs"));
}

#[cfg(feature = "test_embedded")]
#[expect(
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    reason = "generated code"
)]
#[allow(unused_imports, unused_qualifications, missing_docs, non_snake_case)]
pub mod at_least_once_input {
    include!(concat!(env!("OUT_DIR"), "/at_least_once_input.rs"));
}

#[cfg(feature = "test_embedded")]
#[expect(
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    reason = "generated code"
)]
#[allow(unused_imports, unused_qualifications, missing_docs, non_snake_case)]
pub mod weak_stream_channel {
    include!(concat!(env!("OUT_DIR"), "/weak_stream_channel.rs"));
}

#[cfg(feature = "test_embedded")]
#[expect(
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    reason = "generated code"
)]
#[allow(unused_imports, unused_qualifications, missing_docs, non_snake_case)]
pub mod weak_channel {
    include!(concat!(env!("OUT_DIR"), "/weak_channel.rs"));
}

#[cfg(feature = "test_embedded")]
#[expect(
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    reason = "generated code"
)]
#[allow(unused_imports, unused_qualifications, missing_docs, non_snake_case)]
pub mod singleton_input {
    include!(concat!(env!("OUT_DIR"), "/singleton_input.rs"));
}

#[cfg(feature = "test_embedded")]
#[expect(
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    reason = "generated code"
)]
#[allow(unused_imports, unused_qualifications, missing_docs, non_snake_case)]
pub mod echo_network {
    include!(concat!(env!("OUT_DIR"), "/echo_network.rs"));
}

#[cfg(feature = "test_embedded")]
#[expect(
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    reason = "generated code"
)]
#[allow(unused_imports, unused_qualifications, missing_docs, non_snake_case)]
pub mod echo_network_embedded {
    include!(concat!(env!("OUT_DIR"), "/echo_network_embedded.rs"));
}

#[cfg(feature = "test_embedded")]
#[expect(
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    reason = "generated code"
)]
#[allow(unused_imports, unused_qualifications, missing_docs, non_snake_case)]
pub mod o2m_broadcast {
    include!(concat!(env!("OUT_DIR"), "/o2m_broadcast.rs"));
}

#[cfg(feature = "test_embedded")]
#[expect(
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    reason = "generated code"
)]
#[allow(unused_imports, unused_qualifications, missing_docs, non_snake_case)]
pub mod o2m_demux_embedded {
    include!(concat!(env!("OUT_DIR"), "/o2m_demux_embedded.rs"));
}

#[cfg(feature = "test_embedded")]
#[expect(
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    reason = "generated code"
)]
#[allow(unused_imports, unused_qualifications, missing_docs, non_snake_case)]
pub mod m2o_send {
    include!(concat!(env!("OUT_DIR"), "/m2o_send.rs"));
}

#[cfg(feature = "test_embedded")]
#[expect(
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    reason = "generated code"
)]
#[allow(unused_imports, unused_qualifications, missing_docs, non_snake_case)]
pub mod m2m_broadcast {
    include!(concat!(env!("OUT_DIR"), "/m2m_broadcast.rs"));
}

#[cfg(all(test, feature = "test_embedded"))]
mod tests {
    use dfir_rs::bytes::{Bytes, BytesMut};
    use dfir_rs::futures::stream;
    use hydro_lang::compile::embedded_runtime::{
        InputChannel, OutputChannel, Promises, SatisfiedBy,
    };
    use hydro_lang::live_collections::stream::{ExactlyOnce, TotalOrder};
    use hydro_lang::location::MembershipEvent;
    use hydro_lang::location::member_id::TaglessMemberId;
    use hydro_lang::prelude::manual_proof;

    /// Every stream input and channel in these tests is an in-memory list or queue, which keeps
    /// its order and delivers each element once, so it serves any `C` needing at most that.
    fn in_memory<C, S>(inner: S) -> InputChannel<C, S>
    where
        C: SatisfiedBy<TotalOrder, ExactlyOnce>,
    {
        InputChannel::new(
            inner,
            Promises::new()
                .total_order(manual_proof!(
                    /// An in-memory list or queue keeps the order of its elements.
                ))
                .exactly_once(manual_proof!(
                    /// An in-memory list or queue holds each element once.
                )),
        )
    }

    /// Runs the generated `capitalize` dataflow from the given generated module and checks its
    /// output. The same Hydro program is generated into several modules with different codegen
    /// options (see `build.rs`), so this body is shared between them.
    macro_rules! check_capitalize {
        ($module:ident) => {{
            let input = stream::iter(vec![
                "hello".to_owned(),
                "world".to_owned(),
                "hydro".to_owned(),
            ]);
            let mut collected = vec![];
            let mut outputs = crate::$module::capitalize::EmbeddedOutputs {
                output: OutputChannel::new(|s: String| collected.push(s))
                    .require_total_order()
                    .require_exactly_once(),
            };
            let mut flow = crate::$module::capitalize(
                crate::$module::capitalize::EmbeddedInputs {
                    input: in_memory(input),
                },
                &mut outputs,
            );
            flow.run_tick().await;
            drop(flow);
            assert_eq!(collected, vec!["HELLO", "WORLD", "HYDRO"]);
        }};
    }

    // --- capitalize (no networking) ---
    // Order: (inputs, outputs)
    #[tokio::test]
    async fn test_embedded_capitalize() {
        check_capitalize!(embedded);
    }

    // --- capitalize, generated with flame attribution wrappers ---
    // Same program as above, but exercises the per-operator wrapper codegen path
    // (`require_flame_attribution`), which the default `embedded` module does not emit.
    #[tokio::test]
    async fn test_embedded_capitalize_flame_attribution() {
        check_capitalize!(embedded_flame_attribution);
    }

    // --- capitalize_unordered (an unordered output, no networking) ---
    // Order: (inputs, outputs)
    #[tokio::test]
    async fn test_embedded_unordered_output() {
        let input = stream::iter(vec![
            "hello".to_owned(),
            "world".to_owned(),
            "hydro".to_owned(),
        ]);
        let mut collected = vec![];
        let mut outputs = crate::unordered_output::capitalize_unordered::EmbeddedOutputs {
            output: OutputChannel::new(|s: String| collected.push(s)),
        };
        let mut flow = crate::unordered_output::capitalize_unordered(
            crate::unordered_output::capitalize_unordered::EmbeddedInputs {
                input: in_memory(input),
            },
            &mut outputs,
        );
        flow.run_tick().await;
        drop(flow);
        // The output is unordered, so compare contents only.
        collected.sort();
        assert_eq!(collected, vec!["HELLO", "HYDRO", "WORLD"]);
    }

    // --- unique_names (a `NoOrder`, `AtLeastOnce` input, no networking) ---
    // Order: (inputs, outputs)
    #[tokio::test]
    async fn test_embedded_at_least_once_input() {
        // The host's transport delivered "hello" twice, and out of order, which a `NoOrder`,
        // `AtLeastOnce` input allows.
        let input = stream::iter(vec![
            "world".to_owned(),
            "hello".to_owned(),
            "hello".to_owned(),
        ]);
        let mut collected = vec![];
        let mut outputs = crate::at_least_once_input::unique_names::EmbeddedOutputs {
            output: OutputChannel::new(|s: String| collected.push(s)),
        };
        let mut flow = crate::at_least_once_input::unique_names(
            crate::at_least_once_input::unique_names::EmbeddedInputs {
                // The dataflow relies on nothing about this input, so no promises are needed.
                input: InputChannel::new(input, Promises::new()),
            },
            &mut outputs,
        );
        flow.run_tick().await;
        drop(flow);
        // The output is unordered, so compare contents only.
        collected.sort();
        assert_eq!(collected, vec!["hello", "world"]);
    }

    // The tests below construct valid combinations of host code and channels, and check that
    // they compile.

    // The dataflow sends an unordered, at-least-once stream on `messages`, so its receiving end
    // needs no promises, even though the channel is declared `exactly_once().total_order()`.
    #[test]
    fn test_weak_stream_needs_no_channel_promises() {
        use crate::weak_stream_channel::weak_stream_receiver;

        let _net_in = weak_stream_receiver::EmbeddedNetworkIn {
            messages: InputChannel::new(stream::empty::<String>(), Promises::new()),
        };
    }

    // `messages` is declared `at_least_once().no_order()`, so its receiving end needs no
    // promises, even though the dataflow sends an ordered, exactly-once stream, which a
    // transport at its sending end may require.
    #[test]
    fn test_weak_channel_needs_no_promises() {
        use crate::weak_channel::{weak_channel_receiver, weak_channel_sender};

        let _net_in = weak_channel_receiver::EmbeddedNetworkIn {
            messages: InputChannel::new(stream::empty::<String>(), Promises::new()),
        };
        let _net_out = weak_channel_sender::EmbeddedNetworkOut {
            messages: OutputChannel::new(|_: String| {})
                .require_total_order()
                .require_exactly_once(),
        };
    }

    // --- singleton_input (singleton + stream, no networking) ---
    // Order: (singleton_inputs, inputs, outputs)
    #[tokio::test]
    async fn test_embedded_singleton_input() {
        let names = stream::iter(vec!["Alice".to_owned(), "Bob".to_owned()]);
        let mut collected = vec![];
        let mut outputs = crate::singleton_input::prefix_names::EmbeddedOutputs {
            output: OutputChannel::new(|s: String| collected.push(s)),
        };
        let mut flow = crate::singleton_input::prefix_names(
            crate::singleton_input::prefix_names::EmbeddedInputs {
                names: in_memory(names),
                prefix: "Hello".to_owned(),
            },
            &mut outputs,
        );
        flow.run_tick().await;
        drop(flow);
        assert_eq!(collected, vec!["Hello Alice", "Hello Bob"]);
    }

    // --- echo_network (o2o) ---
    // sender order: (inputs, network_out)
    // receiver order: (outputs, network_in)
    #[tokio::test]
    async fn test_echo_network() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Bytes>();

        // Sender: (input, net_out)
        let input = stream::iter(vec!["hello".to_owned(), "world".to_owned()]);
        let mut net_out = crate::echo_network::echo_sender::EmbeddedNetworkOut {
            messages: OutputChannel::new(move |bytes: Bytes| {
                tx.send(bytes).unwrap();
            }),
        };
        let mut flow_sender = crate::echo_network::echo_sender(
            crate::echo_network::echo_sender::EmbeddedInputs {
                input: in_memory(input),
            },
            &mut net_out,
        );
        flow_sender.run_tick().await;
        drop(flow_sender);

        let mut bytes_vec = vec![];
        while let Ok(b) = rx.try_recv() {
            bytes_vec.push(Ok(BytesMut::from(b.as_ref())));
        }
        assert_eq!(bytes_vec.len(), 2);

        // Receiver: (outputs, network_in)
        let net_in = crate::echo_network::echo_receiver::EmbeddedNetworkIn {
            messages: in_memory(stream::iter(bytes_vec)),
        };
        let mut received = vec![];
        let mut outputs = crate::echo_network::echo_receiver::EmbeddedOutputs {
            output: OutputChannel::new(|s: String| received.push(s)),
        };
        let mut flow_receiver = crate::echo_network::echo_receiver(&mut outputs, net_in);
        flow_receiver.run_tick().await;
        drop(flow_receiver);
        assert_eq!(received, vec!["HELLO", "WORLD"]);
    }

    // --- echo_network_embedded (o2o, an `EMBEDDED` channel) ---
    // Unlike `echo_network`, the network channel exposes the raw `String` payload (not `Bytes`),
    // so serialization is performed by the surrounding (test) code.
    #[tokio::test]
    async fn test_echo_network_embedded() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();

        // Sender: `messages` is now `FnMut(String)` (the raw payload, not `Bytes`).
        let input = stream::iter(vec!["hello".to_owned(), "world".to_owned()]);
        let mut net_out = crate::echo_network_embedded::echo_sender::EmbeddedNetworkOut {
            // The receiving end promises order and no duplicates; this transport (an in-order
            // queue) keeps them only if the dataflow sends them, so it requires both.
            messages: OutputChannel::new(move |s: String| {
                tx.send(s).unwrap();
            })
            .require_total_order()
            .require_exactly_once(),
        };
        let mut flow_sender = crate::echo_network_embedded::echo_sender(
            crate::echo_network_embedded::echo_sender::EmbeddedInputs {
                input: in_memory(input),
            },
            &mut net_out,
        );
        flow_sender.run_tick().await;
        drop(flow_sender);

        // The transport delivers raw `String`s (an `EMBEDDED` channel has no transport `Result`).
        let mut msgs: Vec<String> = vec![];
        while let Ok(s) = rx.try_recv() {
            msgs.push(s);
        }
        assert_eq!(msgs.len(), 2);

        // Receiver: `messages` is now `Stream<Item = String>` (the raw payload).
        let net_in = crate::echo_network_embedded::echo_receiver::EmbeddedNetworkIn {
            messages: in_memory(stream::iter(msgs)),
        };
        let mut received = vec![];
        let mut outputs = crate::echo_network_embedded::echo_receiver::EmbeddedOutputs {
            output: OutputChannel::new(|s: String| received.push(s))
                .require_total_order()
                .require_exactly_once(),
        };
        let mut flow_receiver = crate::echo_network_embedded::echo_receiver(&mut outputs, net_in);
        flow_receiver.run_tick().await;
        drop(flow_receiver);
        assert_eq!(received, vec!["HELLO", "WORLD"]);
    }

    // --- o2m_broadcast (process -> cluster) ---
    // sender (process): (membership, inputs, network_out)
    // receiver (cluster): (self_id, outputs, network_in)
    #[tokio::test]
    async fn test_o2m_broadcast() {
        let member_id = TaglessMemberId::from_raw_id(0);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(TaglessMemberId, Bytes)>();

        // Sender (process): (membership, input, net_out)
        let input = stream::iter(vec!["hello".to_owned(), "world".to_owned()]);
        let membership = crate::o2m_broadcast::o2m_sender::EmbeddedMembershipStreams {
            o2m_receiver: in_memory(stream::iter(vec![(
                member_id.clone(),
                MembershipEvent::Joined,
            )])),
        };
        let mut net_out = crate::o2m_broadcast::o2m_sender::EmbeddedNetworkOut {
            o2m_data: OutputChannel::new(move |item: (TaglessMemberId, Bytes)| {
                tx.send(item).unwrap();
            }),
        };
        let mut flow_sender = crate::o2m_broadcast::o2m_sender(
            membership,
            crate::o2m_broadcast::o2m_sender::EmbeddedInputs {
                input: in_memory(input),
            },
            &mut net_out,
        );
        flow_sender.run_tick().await;
        drop(flow_sender);

        let mut tagged_bytes = vec![];
        while let Ok((id, b)) = rx.try_recv() {
            assert_eq!(id, member_id);
            tagged_bytes.push(Ok(BytesMut::from(b.as_ref())));
        }
        assert_eq!(tagged_bytes.len(), 2);

        // Receiver (cluster): (self_id, outputs, network_in)
        let net_in = crate::o2m_broadcast::o2m_receiver::EmbeddedNetworkIn {
            o2m_data: in_memory(stream::iter(tagged_bytes)),
        };
        let mut received = vec![];
        let mut outputs = crate::o2m_broadcast::o2m_receiver::EmbeddedOutputs {
            output: OutputChannel::new(|s: String| received.push(s)),
        };
        let mut flow_receiver =
            crate::o2m_broadcast::o2m_receiver(&member_id, &mut outputs, net_in);
        flow_receiver.run_tick().await;
        drop(flow_receiver);
        assert_eq!(received, vec!["HELLO", "WORLD"]);
    }

    // --- o2m_demux_embedded (process -> cluster demux, an `EMBEDDED` channel) ---
    // The payload type has no serde derives; the embedded channel moves it in-process
    // (regression test for hydro-project/hydro#3158).
    // sender (process): (inputs, network_out)
    // receiver (cluster): (self_id, outputs, network_in)
    #[tokio::test]
    async fn test_o2m_demux_embedded() {
        use hydro_test::embedded::o2m_demux_embedded::OpaquePayload;

        let member_id = TaglessMemberId::from_raw_id(0);
        let (tx, mut rx) =
            tokio::sync::mpsc::unbounded_channel::<(TaglessMemberId, OpaquePayload)>();

        // Sender (process): (input, net_out); the channel carries the raw payload (not `Bytes`).
        let input = stream::iter(vec!["hello".to_owned(), "world".to_owned()]);
        let mut net_out = crate::o2m_demux_embedded::o2m_demux_sender::EmbeddedNetworkOut {
            demux_data: OutputChannel::new(move |item: (TaglessMemberId, OpaquePayload)| {
                tx.send(item).unwrap();
            }),
        };
        let mut flow_sender = crate::o2m_demux_embedded::o2m_demux_sender(
            crate::o2m_demux_embedded::o2m_demux_sender::EmbeddedInputs {
                input: in_memory(input),
            },
            &mut net_out,
        );
        flow_sender.run_tick().await;
        drop(flow_sender);

        let mut payloads = vec![];
        while let Ok((id, payload)) = rx.try_recv() {
            assert_eq!(id, member_id);
            payloads.push(payload);
        }
        assert_eq!(payloads.len(), 2);

        // Receiver (cluster): (self_id, outputs, network_in); raw payloads, no deserialization.
        let net_in = crate::o2m_demux_embedded::o2m_demux_receiver::EmbeddedNetworkIn {
            demux_data: in_memory(stream::iter(payloads)),
        };
        let mut received = vec![];
        let mut outputs = crate::o2m_demux_embedded::o2m_demux_receiver::EmbeddedOutputs {
            output: OutputChannel::new(|s: String| received.push(s)),
        };
        let mut flow_receiver =
            crate::o2m_demux_embedded::o2m_demux_receiver(&member_id, &mut outputs, net_in);
        flow_receiver.run_tick().await;
        drop(flow_receiver);
        assert_eq!(received, vec!["HELLO", "WORLD"]);
    }

    // --- m2o_send (cluster -> process) ---
    // sender (cluster): (self_id, inputs, network_out)
    // receiver (process): (outputs, network_in)
    #[tokio::test]
    async fn test_m2o_send() {
        let member_id = TaglessMemberId::from_raw_id(42);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Bytes>();

        // Sender (cluster): (self_id, input, net_out)
        let input = stream::iter(vec!["foo".to_owned(), "bar".to_owned()]);
        let mut net_out = crate::m2o_send::m2o_sender::EmbeddedNetworkOut {
            m2o_data: OutputChannel::new(move |bytes: Bytes| {
                tx.send(bytes).unwrap();
            }),
        };
        let mut flow_sender = crate::m2o_send::m2o_sender(
            &member_id,
            crate::m2o_send::m2o_sender::EmbeddedInputs {
                input: in_memory(input),
            },
            &mut net_out,
        );
        flow_sender.run_tick().await;
        drop(flow_sender);

        let mut tagged_bytes = vec![];
        // Wrap as tagged (simulating transport tagging by member id)
        while let Ok(b) = rx.try_recv() {
            tagged_bytes.push(Ok((member_id.clone(), BytesMut::from(b.as_ref()))));
        }
        assert_eq!(tagged_bytes.len(), 2);

        // Receiver (process): (outputs, network_in)
        let net_in = crate::m2o_send::m2o_receiver::EmbeddedNetworkIn {
            m2o_data: in_memory(stream::iter(tagged_bytes)),
        };
        let mut received = vec![];
        let mut outputs = crate::m2o_send::m2o_receiver::EmbeddedOutputs {
            output: OutputChannel::new(|s| received.push(s)),
        };
        let mut flow_receiver = crate::m2o_send::m2o_receiver(&mut outputs, net_in);
        flow_receiver.run_tick().await;
        drop(flow_receiver);
        assert_eq!(received.len(), 2);
        // Values are uppercased; entries() gives (MemberId, String)
        assert_eq!(received[0].1, "FOO");
        assert_eq!(received[1].1, "BAR");
    }

    // --- m2m_broadcast (cluster -> cluster) ---
    // sender (cluster): (self_id, membership, inputs, network_out)
    // receiver (cluster): (self_id, outputs, network_in)
    #[tokio::test]
    async fn test_m2m_broadcast() {
        let src_id = TaglessMemberId::from_raw_id(0);
        let dst_id = TaglessMemberId::from_raw_id(0);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(TaglessMemberId, Bytes)>();

        // Sender (cluster): (self_id, membership, input, net_out)
        let input = stream::iter(vec!["ping".to_owned()]);
        let membership = crate::m2m_broadcast::m2m_sender::EmbeddedMembershipStreams {
            m2m_receiver: in_memory(stream::iter(vec![(
                dst_id.clone(),
                MembershipEvent::Joined,
            )])),
        };
        let mut net_out = crate::m2m_broadcast::m2m_sender::EmbeddedNetworkOut {
            m2m_data: OutputChannel::new(move |item: (TaglessMemberId, Bytes)| {
                tx.send(item).unwrap();
            }),
        };
        let mut flow_sender = crate::m2m_broadcast::m2m_sender(
            &src_id,
            membership,
            crate::m2m_broadcast::m2m_sender::EmbeddedInputs {
                input: in_memory(input),
            },
            &mut net_out,
        );
        flow_sender.run_tick().await;
        drop(flow_sender);

        let mut tagged_bytes = vec![];
        while let Ok((id, b)) = rx.try_recv() {
            assert_eq!(id, dst_id);
            tagged_bytes.push(Ok((src_id.clone(), BytesMut::from(b.as_ref()))));
        }
        assert_eq!(tagged_bytes.len(), 1);

        // Receiver (cluster): (self_id, outputs, network_in)
        let net_in = crate::m2m_broadcast::m2m_receiver::EmbeddedNetworkIn {
            m2m_data: in_memory(stream::iter(tagged_bytes)),
        };
        let mut received = vec![];
        let mut outputs = crate::m2m_broadcast::m2m_receiver::EmbeddedOutputs {
            output: OutputChannel::new(|s| received.push(s)),
        };
        let mut flow_receiver = crate::m2m_broadcast::m2m_receiver(&dst_id, &mut outputs, net_in);
        flow_receiver.run_tick().await;
        drop(flow_receiver);
        assert_eq!(received.len(), 1);
        assert_eq!(received[0].1, "PING");
    }
}
