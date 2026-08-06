use hydro_lang::live_collections::stream::TotalOrder;
use hydro_lang::location::MemberId;
use hydro_lang::prelude::*;

pub struct Client;
pub struct LogServer;

/// Builds a totally-ordered event log on a central server from events emitted
/// by a cluster of clients. Events from *different* clients may interleave
/// arbitrarily (the arrival order is non-deterministic), but the log always
/// preserves each client's local order, since TCP delivers an in-order prefix
/// per sender.
///
/// This example is featured on the Hydro website landing page.
pub fn ordered_event_log<'a>(
    events: Stream<String, Cluster<'a, Client>>,
    server: &Process<'a, LogServer>,
) -> Stream<(MemberId<Client>, String), Process<'a, LogServer>, Unbounded, TotalOrder> {
    events
        .send(server, TCP.fail_stop().bincode())
        .entries_partially_ordered(nondet!(/** log order = arrival order */))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_per_member_prefix_order() {
        let mut flow = FlowBuilder::new();
        let clients = flow.cluster::<Client>();
        let server = flow.process::<LogServer>();

        let (event_port, events) = clients.sim_input();
        let log = ordered_event_log(events, &server).sim_output();

        let instances = flow
            .sim()
            .with_cluster_size(&clients, 3)
            .exhaustive(async || {
                event_port.send(0, "a1".to_string());
                event_port.send(0, "a2".to_string());
                event_port.send(1, "b1".to_string());

                let entries: Vec<_> = log.collect().await;
                let pos = |m: &str| entries.iter().position(|(_, e)| e == m).unwrap();
                // events interleave arbitrarily, but per-client order always holds
                assert!(pos("a1") < pos("a2"));
            });

        // The landing page displays this exact exploration: three distinct
        // interleavings of [a1, a2] x [b1] that preserve per-member prefixes.
        assert_eq!(instances, 3);
    }
}
