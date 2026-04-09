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

#[cfg(feature = "test_embedded")]
#[expect(
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    reason = "generated code"
)]
#[allow(unused_imports, unused_qualifications, missing_docs, non_snake_case)]
pub mod embedded_inline {
    include!(concat!(env!("OUT_DIR"), "/embedded_inline.rs"));
}

#[cfg(feature = "test_embedded")]
#[expect(
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    reason = "generated code"
)]
#[allow(unused_imports, unused_qualifications, missing_docs, non_snake_case)]
pub mod singleton_input_inline {
    include!(concat!(env!("OUT_DIR"), "/singleton_input_inline.rs"));
}

#[cfg(feature = "test_embedded")]
#[expect(
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    reason = "generated code"
)]
#[allow(unused_imports, unused_qualifications, missing_docs, non_snake_case)]
pub mod echo_network_inline {
    include!(concat!(env!("OUT_DIR"), "/echo_network_inline.rs"));
}

#[cfg(feature = "test_embedded")]
#[expect(
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    reason = "generated code"
)]
#[allow(unused_imports, unused_qualifications, missing_docs, non_snake_case)]
pub mod o2m_broadcast_inline {
    include!(concat!(env!("OUT_DIR"), "/o2m_broadcast_inline.rs"));
}

#[cfg(feature = "test_embedded")]
#[expect(
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    reason = "generated code"
)]
#[allow(unused_imports, unused_qualifications, missing_docs, non_snake_case)]
pub mod m2o_send_inline {
    include!(concat!(env!("OUT_DIR"), "/m2o_send_inline.rs"));
}

#[cfg(feature = "test_embedded")]
#[expect(
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    reason = "generated code"
)]
#[allow(unused_imports, unused_qualifications, missing_docs, non_snake_case)]
pub mod m2m_broadcast_inline {
    include!(concat!(env!("OUT_DIR"), "/m2m_broadcast_inline.rs"));
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
    use hydro_lang::location::MembershipEvent;
    use hydro_lang::location::member_id::TaglessMemberId;

    async fn run_dfir(mut flow: dfir_rs::scheduled::graph::Dfir<'_>) {
        tokio::task::LocalSet::new()
            .run_until(flow.run_available())
            .await;
    }

    // --- capitalize (no networking) ---
    // Order: (inputs, outputs)
    #[tokio::test]
    async fn test_embedded_capitalize() {
        let input = stream::iter(vec![
            "hello".to_owned(),
            "world".to_owned(),
            "hydro".to_owned(),
        ]);
        let mut collected = vec![];
        let mut outputs = crate::embedded::capitalize::EmbeddedOutputs {
            output: |s: String| collected.push(s),
        };
        let flow = crate::embedded::capitalize(input, &mut outputs);
        run_dfir(flow).await;
        assert_eq!(collected, vec!["HELLO", "WORLD", "HYDRO"]);
    }

    // --- capitalize_inline (no networking, inline codegen) ---
    #[tokio::test]
    async fn test_embedded_capitalize_inline() {
        let input = stream::iter(vec![
            "hello".to_owned(),
            "world".to_owned(),
            "hydro".to_owned(),
        ]);
        let mut collected = vec![];
        let mut outputs = crate::embedded_inline::capitalize_inline::EmbeddedOutputs {
            output: |s: String| collected.push(s),
        };
        let mut flow = crate::embedded_inline::capitalize_inline(input, &mut outputs);
        tokio::task::LocalSet::new()
            .run_until(flow.run_tick())
            .await;
        drop(flow);
        assert_eq!(collected, vec!["HELLO", "WORLD", "HYDRO"]);
    }

    // --- singleton_input_inline (singleton + stream, inline codegen) ---
    #[tokio::test]
    async fn test_embedded_singleton_input_inline() {
        let names = stream::iter(vec!["Alice".to_owned(), "Bob".to_owned()]);
        let mut collected = vec![];
        let mut outputs = crate::singleton_input_inline::prefix_names_inline::EmbeddedOutputs {
            output: |s: String| collected.push(s),
        };
        let mut flow =
            crate::singleton_input_inline::prefix_names_inline("Hello".to_owned(), names, &mut outputs);
        tokio::task::LocalSet::new()
            .run_until(flow.run_tick())
            .await;
        drop(flow);
        assert_eq!(collected, vec!["Hello Alice", "Hello Bob"]);
    }

    // --- echo_network_inline (o2o networking, inline codegen) ---
    #[tokio::test]
    async fn test_echo_network_inline() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Bytes>();

        // Sender: (input, net_out)
        let input = stream::iter(vec!["hello".to_owned(), "world".to_owned()]);
        let mut net_out = crate::echo_network_inline::echo_sender_inline::EmbeddedNetworkOut {
            messages: move |bytes: Bytes| {
                tx.send(bytes).unwrap();
            },
        };
        let mut flow_sender =
            crate::echo_network_inline::echo_sender_inline(input, &mut net_out);
        tokio::task::LocalSet::new()
            .run_until(flow_sender.run_tick())
            .await;
        drop(flow_sender);

        let mut bytes_vec = vec![];
        while let Ok(b) = rx.try_recv() {
            bytes_vec.push(Ok(BytesMut::from(b.as_ref())));
        }
        assert_eq!(bytes_vec.len(), 2);

        // Receiver: (outputs, network_in)
        let net_in = crate::echo_network_inline::echo_receiver_inline::EmbeddedNetworkIn {
            messages: stream::iter(bytes_vec),
        };
        let mut received = vec![];
        let mut outputs = crate::echo_network_inline::echo_receiver_inline::EmbeddedOutputs {
            output: |s: String| received.push(s),
        };
        let mut flow_receiver =
            crate::echo_network_inline::echo_receiver_inline(&mut outputs, net_in);
        tokio::task::LocalSet::new()
            .run_until(flow_receiver.run_tick())
            .await;
        drop(flow_receiver);
        assert_eq!(received, vec!["HELLO", "WORLD"]);
    }

    // Helper to run an inline tick closure in a LocalSet.
    async fn run_inline(flow: &mut dfir_rs::scheduled::context::InlineFlow<impl std::ops::AsyncFnMut()>) {
        tokio::task::LocalSet::new()
            .run_until(flow.run_tick())
            .await;
    }

    // --- o2m_broadcast_inline (process -> cluster, inline) ---
    #[tokio::test]
    async fn test_o2m_broadcast_inline() {
        let member_id = TaglessMemberId::from_raw_id(0);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(TaglessMemberId, Bytes)>();

        let input = stream::iter(vec!["hello".to_owned(), "world".to_owned()]);
        let membership = crate::o2m_broadcast_inline::o2m_sender_inline::EmbeddedMembershipStreams {
            o2m_receiver_inline: stream::iter(vec![(member_id.clone(), MembershipEvent::Joined)]),
        };
        let mut net_out = crate::o2m_broadcast_inline::o2m_sender_inline::EmbeddedNetworkOut {
            o2m_data: move |item: (TaglessMemberId, Bytes)| {
                tx.send(item).unwrap();
            },
        };
        let mut flow_sender =
            crate::o2m_broadcast_inline::o2m_sender_inline(membership, input, &mut net_out);
        run_inline(&mut flow_sender).await;
        drop(flow_sender);

        let mut tagged_bytes = vec![];
        while let Ok((id, b)) = rx.try_recv() {
            assert_eq!(id, member_id);
            tagged_bytes.push(Ok(BytesMut::from(b.as_ref())));
        }
        assert_eq!(tagged_bytes.len(), 2);

        let net_in = crate::o2m_broadcast_inline::o2m_receiver_inline::EmbeddedNetworkIn {
            o2m_data: stream::iter(tagged_bytes),
        };
        let mut received = vec![];
        let mut outputs = crate::o2m_broadcast_inline::o2m_receiver_inline::EmbeddedOutputs {
            output: |s: String| received.push(s),
        };
        let mut flow_receiver =
            crate::o2m_broadcast_inline::o2m_receiver_inline(&member_id, &mut outputs, net_in);
        run_inline(&mut flow_receiver).await;
        drop(flow_receiver);
        assert_eq!(received, vec!["HELLO", "WORLD"]);
    }

    // --- m2o_send_inline (cluster -> process, inline) ---
    #[tokio::test]
    async fn test_m2o_send_inline() {
        let member_id = TaglessMemberId::from_raw_id(42);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Bytes>();

        let input = stream::iter(vec!["foo".to_owned(), "bar".to_owned()]);
        let mut net_out = crate::m2o_send_inline::m2o_sender_inline::EmbeddedNetworkOut {
            m2o_data: move |bytes: Bytes| {
                tx.send(bytes).unwrap();
            },
        };
        let mut flow_sender =
            crate::m2o_send_inline::m2o_sender_inline(&member_id, input, &mut net_out);
        run_inline(&mut flow_sender).await;
        drop(flow_sender);

        let mut tagged_bytes = vec![];
        while let Ok(b) = rx.try_recv() {
            tagged_bytes.push(Ok((member_id.clone(), BytesMut::from(b.as_ref()))));
        }
        assert_eq!(tagged_bytes.len(), 2);

        let net_in = crate::m2o_send_inline::m2o_receiver_inline::EmbeddedNetworkIn {
            m2o_data: stream::iter(tagged_bytes),
        };
        let mut received = vec![];
        let mut outputs = crate::m2o_send_inline::m2o_receiver_inline::EmbeddedOutputs {
            output: |s| received.push(s),
        };
        let mut flow_receiver =
            crate::m2o_send_inline::m2o_receiver_inline(&mut outputs, net_in);
        run_inline(&mut flow_receiver).await;
        drop(flow_receiver);
        assert_eq!(received.len(), 2);
        assert_eq!(received[0].1, "FOO");
        assert_eq!(received[1].1, "BAR");
    }

    // --- m2m_broadcast_inline (cluster -> cluster, inline) ---
    #[tokio::test]
    async fn test_m2m_broadcast_inline() {
        let src_id = TaglessMemberId::from_raw_id(0);
        let dst_id = TaglessMemberId::from_raw_id(0);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(TaglessMemberId, Bytes)>();

        let input = stream::iter(vec!["ping".to_owned()]);
        let membership = crate::m2m_broadcast_inline::m2m_sender_inline::EmbeddedMembershipStreams {
            m2m_receiver_inline: stream::iter(vec![(dst_id.clone(), MembershipEvent::Joined)]),
        };
        let mut net_out = crate::m2m_broadcast_inline::m2m_sender_inline::EmbeddedNetworkOut {
            m2m_data: move |item: (TaglessMemberId, Bytes)| {
                tx.send(item).unwrap();
            },
        };
        let mut flow_sender =
            crate::m2m_broadcast_inline::m2m_sender_inline(&src_id, membership, input, &mut net_out);
        run_inline(&mut flow_sender).await;
        drop(flow_sender);

        let mut tagged_bytes = vec![];
        while let Ok((id, b)) = rx.try_recv() {
            assert_eq!(id, dst_id);
            tagged_bytes.push(Ok((src_id.clone(), BytesMut::from(b.as_ref()))));
        }
        assert_eq!(tagged_bytes.len(), 1);

        let net_in = crate::m2m_broadcast_inline::m2m_receiver_inline::EmbeddedNetworkIn {
            m2m_data: stream::iter(tagged_bytes),
        };
        let mut received = vec![];
        let mut outputs = crate::m2m_broadcast_inline::m2m_receiver_inline::EmbeddedOutputs {
            output: |s| received.push(s),
        };
        let mut flow_receiver =
            crate::m2m_broadcast_inline::m2m_receiver_inline(&dst_id, &mut outputs, net_in);
        run_inline(&mut flow_receiver).await;
        drop(flow_receiver);
        assert_eq!(received.len(), 1);
        assert_eq!(received[0].1, "PING");
    }

    // --- singleton_input (singleton + stream, no networking) ---
    // Order: (singleton_inputs, inputs, outputs)
    #[tokio::test]
    async fn test_embedded_singleton_input() {
        let names = stream::iter(vec!["Alice".to_owned(), "Bob".to_owned()]);
        let mut collected = vec![];
        let mut outputs = crate::singleton_input::prefix_names::EmbeddedOutputs {
            output: |s: String| collected.push(s),
        };
        let flow = crate::singleton_input::prefix_names("Hello".to_owned(), names, &mut outputs);
        run_dfir(flow).await;
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
            messages: move |bytes: Bytes| {
                tx.send(bytes).unwrap();
            },
        };
        run_dfir(crate::echo_network::echo_sender(input, &mut net_out)).await;

        let mut bytes_vec = vec![];
        while let Ok(b) = rx.try_recv() {
            bytes_vec.push(Ok(BytesMut::from(b.as_ref())));
        }
        assert_eq!(bytes_vec.len(), 2);

        // Receiver: (outputs, network_in)
        let net_in = crate::echo_network::echo_receiver::EmbeddedNetworkIn {
            messages: stream::iter(bytes_vec),
        };
        let mut received = vec![];
        let mut outputs = crate::echo_network::echo_receiver::EmbeddedOutputs {
            output: |s: String| received.push(s),
        };
        run_dfir(crate::echo_network::echo_receiver(&mut outputs, net_in)).await;
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
            o2m_receiver: stream::iter(vec![(member_id.clone(), MembershipEvent::Joined)]),
        };
        let mut net_out = crate::o2m_broadcast::o2m_sender::EmbeddedNetworkOut {
            o2m_data: move |item: (TaglessMemberId, Bytes)| {
                tx.send(item).unwrap();
            },
        };
        run_dfir(crate::o2m_broadcast::o2m_sender(
            membership,
            input,
            &mut net_out,
        ))
        .await;

        let mut tagged_bytes = vec![];
        while let Ok((id, b)) = rx.try_recv() {
            assert_eq!(id, member_id);
            tagged_bytes.push(Ok(BytesMut::from(b.as_ref())));
        }
        assert_eq!(tagged_bytes.len(), 2);

        // Receiver (cluster): (self_id, outputs, network_in)
        let net_in = crate::o2m_broadcast::o2m_receiver::EmbeddedNetworkIn {
            o2m_data: stream::iter(tagged_bytes),
        };
        let mut received = vec![];
        let mut outputs = crate::o2m_broadcast::o2m_receiver::EmbeddedOutputs {
            output: |s: String| received.push(s),
        };
        run_dfir(crate::o2m_broadcast::o2m_receiver(
            &member_id,
            &mut outputs,
            net_in,
        ))
        .await;
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
            m2o_data: move |bytes: Bytes| {
                tx.send(bytes).unwrap();
            },
        };
        run_dfir(crate::m2o_send::m2o_sender(&member_id, input, &mut net_out)).await;

        // Wrap as tagged (simulating transport tagging by member id)
        let mut tagged_bytes = vec![];
        while let Ok(b) = rx.try_recv() {
            tagged_bytes.push(Ok((member_id.clone(), BytesMut::from(b.as_ref()))));
        }
        assert_eq!(tagged_bytes.len(), 2);

        // Receiver (process): (outputs, network_in)
        let net_in = crate::m2o_send::m2o_receiver::EmbeddedNetworkIn {
            m2o_data: stream::iter(tagged_bytes),
        };
        let mut received = vec![];
        let mut outputs = crate::m2o_send::m2o_receiver::EmbeddedOutputs {
            output: |s| received.push(s),
        };
        run_dfir(crate::m2o_send::m2o_receiver(&mut outputs, net_in)).await;
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
            m2m_receiver: stream::iter(vec![(dst_id.clone(), MembershipEvent::Joined)]),
        };
        let mut net_out = crate::m2m_broadcast::m2m_sender::EmbeddedNetworkOut {
            m2m_data: move |item: (TaglessMemberId, Bytes)| {
                tx.send(item).unwrap();
            },
        };
        run_dfir(crate::m2m_broadcast::m2m_sender(
            &src_id,
            membership,
            input,
            &mut net_out,
        ))
        .await;

        let mut tagged_bytes = vec![];
        while let Ok((id, b)) = rx.try_recv() {
            assert_eq!(id, dst_id);
            tagged_bytes.push(Ok((src_id.clone(), BytesMut::from(b.as_ref()))));
        }
        assert_eq!(tagged_bytes.len(), 1);

        // Receiver (cluster): (self_id, outputs, network_in)
        let net_in = crate::m2m_broadcast::m2m_receiver::EmbeddedNetworkIn {
            m2m_data: stream::iter(tagged_bytes),
        };
        let mut received = vec![];
        let mut outputs = crate::m2m_broadcast::m2m_receiver::EmbeddedOutputs {
            output: |s| received.push(s),
        };
        run_dfir(crate::m2m_broadcast::m2m_receiver(
            &dst_id,
            &mut outputs,
            net_in,
        ))
        .await;
        assert_eq!(received.len(), 1);
        assert_eq!(received[0].1, "PING");
    }
}
