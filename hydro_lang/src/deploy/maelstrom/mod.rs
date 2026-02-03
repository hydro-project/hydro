//! Deployment backend for running correctness tests against Jepsen Maelstrom (https://github.com/jepsen-io/maelstrom)

use crate::forward_handle::ForwardHandle;
use crate::live_collections::KeyedStream;
use crate::location::{Cluster, NoTick};
use crate::nondet::nondet;

#[cfg(stageleft_runtime)]
#[cfg(feature = "maelstrom")]
#[cfg_attr(docsrs, doc(cfg(feature = "maelstrom")))]
pub mod deploy_maelstrom;

pub mod deploy_runtime_maelstrom;

/// Sets up bidirectional communication with Maelstrom clients on a cluster.
///
/// This function provides a similar API to `bidi_external_many_bytes` but for Maelstrom
/// client communication. It returns a keyed input stream of client messages and accepts
/// a keyed output stream of responses.
///
/// The key type is `String` (the client ID like "c1", "c2").
/// The value type is `serde_json::Value` (the message body).
///
/// # Example
/// ```ignore
/// let (input, output_handle) = maelstrom_bidi_clients(&cluster);
/// output_handle.complete(input.map(q!(|(client_id, body)| {
///     // Process and return response
///     (client_id, response_body)
/// })));
/// ```
#[expect(clippy::type_complexity, reason = "stream markers")]
pub fn maelstrom_bidi_clients<'a, C>(
    cluster: &Cluster<'a, C>,
) -> (
    KeyedStream<String, serde_json::Value, Cluster<'a, C>>,
    ForwardHandle<'a, KeyedStream<String, serde_json::Value, Cluster<'a, C>>>,
)
where
    Cluster<'a, C>: NoTick,
{
    use stageleft::q;

    use crate::location::Location;

    let meta: stageleft::RuntimeData<&deploy_runtime_maelstrom::MaelstromMeta> =
        stageleft::RuntimeData::new("__hydro_lang_maelstrom_meta");

    // Create the input stream from Maelstrom clients
    let input: KeyedStream<String, serde_json::Value, Cluster<'a, C>> = cluster
        .source_stream(q!(deploy_runtime_maelstrom::maelstrom_client_source(meta)))
        .into_keyed();

    // Create a forward reference for the output stream
    let (fwd_handle, output_stream) =
        cluster.forward_ref::<KeyedStream<String, serde_json::Value, Cluster<'a, C>>>();

    // Set up the output sink to send responses back to clients
    output_stream
        .entries()
        .assume_ordering(nondet!(/** maelstrom responses can be sent in any order */))
        .for_each(q!(|(client_id, body): (String, serde_json::Value)| {
            deploy_runtime_maelstrom::maelstrom_send_response(&meta.node_id, &client_id, body);
        }));

    (input, fwd_handle)
}
