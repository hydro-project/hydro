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
pub mod echo_network {
    include!(concat!(env!("OUT_DIR"), "/echo_network.rs"));
}

#[cfg(all(test, feature = "test_embedded"))]
mod tests {
    #[tokio::test]
    async fn test_embedded_capitalize() {
        let input = dfir_rs::futures::stream::iter(vec![
            "hello".to_owned(),
            "world".to_owned(),
            "hydro".to_owned(),
        ]);

        let mut collected = vec![];
        let mut outputs = crate::embedded::capitalize::EmbeddedOutputs {
            output: |s: String| {
                collected.push(s);
            },
        };

        let mut flow = crate::embedded::capitalize(input, &mut outputs);
        tokio::task::LocalSet::new()
            .run_until(flow.run_available())
            .await;
        drop(flow);

        assert_eq!(collected, vec!["HELLO", "WORLD", "HYDRO"],);
    }

    #[tokio::test]
    async fn test_echo_network() {
        use dfir_rs::bytes::{Bytes, BytesMut};
        use dfir_rs::futures::stream;

        // Wire sender -> receiver via an in-memory channel.
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Bytes>();

        // --- Run sender ---
        let input = stream::iter(vec!["hello".to_owned(), "world".to_owned()]);

        let mut sender_net_out = crate::echo_network::echo_sender::EmbeddedNetworkOut {
            messages: move |bytes: Bytes| {
                tx.send(bytes).unwrap();
            },
        };

        let mut sender_flow = crate::echo_network::echo_sender(input, &mut sender_net_out);
        tokio::task::LocalSet::new()
            .run_until(sender_flow.run_available())
            .await;
        drop(sender_flow);

        // Collect serialized bytes.
        let mut bytes_vec = vec![];
        while let Ok(b) = rx.try_recv() {
            bytes_vec.push(Ok(BytesMut::from(b.as_ref())));
        }
        assert_eq!(bytes_vec.len(), 2, "sender should have produced 2 messages");

        // --- Run receiver ---
        let receiver_net_in = crate::echo_network::echo_receiver::EmbeddedNetworkIn {
            messages: stream::iter(bytes_vec),
        };

        let mut received = vec![];
        let mut receiver_outputs = crate::echo_network::echo_receiver::EmbeddedOutputs {
            output: |s: String| {
                received.push(s);
            },
        };

        let mut receiver_flow =
            crate::echo_network::echo_receiver(&mut receiver_outputs, receiver_net_in);
        tokio::task::LocalSet::new()
            .run_until(receiver_flow.run_available())
            .await;
        drop(receiver_flow);

        assert_eq!(received, vec!["HELLO", "WORLD"]);
    }
}
