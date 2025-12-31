use std::net::SocketAddr;

use dfir_rs::util::ipv4_resolve;
use web_time::Duration;

use crate::Opts;
use crate::flows::bp_flow::bp_flow;
use crate::flows::client_state_flow::client_state_flow;
use crate::flows::listener_flow::listener_flow;
use crate::flows::orig_flow::orig_flow;
use crate::flows::push_group_flow::push_group_flow;
use crate::flows::rep_server_flow::rep_server_flow;
use crate::flows::server_state_flow::server_state_flow;
use crate::flows::ssiv_flow::ssiv_flow;
use crate::test_data::{client1_vec, client2_vec, client100_vec};
use crate::wrappers::{bp_wrap, ssiv_wrap, tuple_wrap};

// spawn a listener to get the output of a flow and print it on the console
async fn spawn_listener(
    tuple_listener_addr: SocketAddr,
    bp_listener_addr: SocketAddr,
    ssiv_listener_addr: SocketAddr,
) {
    let (_, tuple_listener_in, _) = dfir_rs::util::bind_udp_bytes(tuple_listener_addr).await;
    let (_, bp_listener_in, _) = dfir_rs::util::bind_udp_bytes(bp_listener_addr).await;
    let (_, ssiv_listener_in, _) = dfir_rs::util::bind_udp_bytes(ssiv_listener_addr).await;
    // spawn a listener thread to print out what each flow sends over the network
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let local = tokio::task::LocalSet::new();
        local.block_on(&runtime, async {
            let mut hf = listener_flow(tuple_listener_in, bp_listener_in, ssiv_listener_in).await;
            hf.run().await;
        });
    });
}

// driver program to demonstrate the various shopping cart implementations
pub(crate) async fn run_driver(opts: Opts) {
    // the address for the output; the listener will listen to this address
    let out_addr = ipv4_resolve("localhost:0").unwrap();
    let (out, _, _) = dfir_rs::util::bind_udp_bytes(out_addr).await;

    // define the shopping workload from the test data
    let client1 = tuple_wrap(client1_vec().into_iter());
    let client2 = tuple_wrap(client2_vec().into_iter());
    let client100 = tuple_wrap(client100_vec().into_iter());
    let shopping = client1.chain(client2).chain(client100);

    // define a BoundedPrefix version of the shopping workload from the test data.
    // each client gets a separate BoundedPrefix
    let client1_bp = bp_wrap(client1_vec().into_iter()).map(|r| (1usize, r));
    let client2_bp = bp_wrap(client2_vec().into_iter()).map(|r| (2usize, r));
    let client100_bp = bp_wrap(client100_vec().into_iter()).map(|r| (100usize, r));
    let shopping_bp = client1_bp.chain(client2_bp).chain(client100_bp);

    // define an SSIV version of the shopping workload from the test data.
    // each client gets a separate SSIV
    let client1_ssiv = ssiv_wrap(client1_vec().into_iter()).map(|r| (1usize, r));
    let client2_ssiv = ssiv_wrap(client2_vec().into_iter()).map(|r| (2usize, r));
    let client100_ssiv = ssiv_wrap(client100_vec().into_iter()).map(|r| (100usize, r));
    let shopping_ssiv = client1_ssiv.chain(client2_ssiv).chain(client100_ssiv);

    // set up a listener to get the output of the flows and print to stdout
    let tuple_listener_addr = ipv4_resolve("localhost:23470").unwrap();
    let bp_listener_addr = ipv4_resolve("localhost:23471").unwrap();
    let ssiv_listener_addr = ipv4_resolve("localhost:23472").unwrap();
    spawn_listener(tuple_listener_addr, bp_listener_addr, ssiv_listener_addr).await;

    // run the chosen dataflow
    let mut hf = match opts.opt {
        1 => orig_flow(shopping, tuple_listener_addr, out).await,
        2 => bp_flow(shopping_bp, bp_listener_addr, out).await,
        3 => ssiv_flow(shopping_ssiv, ssiv_listener_addr, out).await,
        4 => push_group_flow(shopping_ssiv, ssiv_listener_addr, out).await,
        opt @ (5 | 6) => {
            // address for a server thread
            let server_addr = ipv4_resolve("localhost:23456").unwrap();
            // addresses for a client proxy thread
            let client_addr = ipv4_resolve("localhost:23457").unwrap();
            let client_out_addr = ipv4_resolve("localhost:23460").unwrap();
            let (client_out, _, _) = dfir_rs::util::bind_udp_bytes(client_out_addr).await;

            // shopping input is handled by the client proxy process
            // so the server process should get an empty iterator as its first argument
            let empty_ssiv = std::iter::empty();

            // Spawn server
            std::thread::spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                let local = tokio::task::LocalSet::new();
                local.block_on(&runtime, async {
                    let mut hf = match opt {
                        5 => {
                            server_state_flow(
                                empty_ssiv,
                                ssiv_listener_addr,
                                out,
                                server_addr,
                                client_addr,
                            )
                            .await
                        }
                        6 => {
                            client_state_flow(
                                empty_ssiv,
                                ssiv_listener_addr,
                                out,
                                server_addr,
                                client_addr,
                            )
                            .await
                        }
                        _ => unreachable!(),
                    };
                    hf.run().await;
                });
            });

            // The above thread spawn sets up some sockets, but we do not wait for it to do so.
            // So there is a race condition where, while the thread is still spawning, we can proceed and send messages to a socket that is not bound.
            // This means that the spawned thread would be waiting for a message that it has already missed and which won't get re-sent.
            // There's a timeout at the bottom of this function that was presumably added to fix the issue of this test hanging.
            // This sleep makes it much more likely that the thread has set up everything needed by the tiem we finish this sleep.
            std::thread::sleep(Duration::from_secs(1));

            // Run client proxy in this thread
            match opt {
                5 => {
                    server_state_flow(
                        shopping_ssiv,
                        client_out_addr,
                        client_out,
                        client_addr,
                        server_addr,
                    )
                    .await
                }
                6 => {
                    client_state_flow(
                        shopping_ssiv,
                        client_out_addr,
                        client_out,
                        client_addr,
                        server_addr,
                    )
                    .await
                }
                _ => unreachable!(),
            }
        }
        7 => {
            // define the server addresses
            let addr1 = ipv4_resolve("localhost:23430").unwrap();
            let addr2 = ipv4_resolve("localhost:23431").unwrap();
            let addr3 = ipv4_resolve("localhost:23432").unwrap();
            let server_addrs = [addr1, addr2, addr3];

            // define the server addresses for gossip
            let gossip_addr1 = ipv4_resolve("localhost:23440").unwrap();
            let gossip_addr2 = ipv4_resolve("localhost:23441").unwrap();
            let gossip_addr3 = ipv4_resolve("localhost:23442").unwrap();
            let gossip_addrs = vec![gossip_addr1, gossip_addr2, gossip_addr3];

            // address for a client proxy thread
            let client_addr = ipv4_resolve("localhost:23457").unwrap();
            let client_out_addr = ipv4_resolve("localhost:23460").unwrap();
            let (client_out, _, _) = dfir_rs::util::bind_udp_bytes(client_out_addr).await;

            // Spawn 3 server replicas asynchronously
            for pair in server_addrs.iter().zip(gossip_addrs.iter()) {
                let (&addr, &gossip_addr) = pair;

                let out_addr = ipv4_resolve("localhost:0").unwrap();
                let (out, _, _) = dfir_rs::util::bind_udp_bytes(out_addr).await;
                let gossip_addrs = gossip_addrs.clone();

                // shopping input is handled by the client proxy process
                // so the server processes should get an empty iterator as first argument
                let empty_ssiv = std::iter::empty();

                // Spawn server
                std::thread::spawn(move || {
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap();
                    let local = tokio::task::LocalSet::new();
                    local.block_on(&runtime, async {
                        let mut hf = rep_server_flow(
                            empty_ssiv,
                            ssiv_listener_addr,
                            out,
                            addr,
                            ssiv_listener_addr,
                            gossip_addr,
                            gossip_addrs.into_iter(),
                        )
                        .await;
                        hf.run().await;
                    });
                });
            }

            // The above thread spawn sets up some sockets, but we do not wait for it to do so.
            // So there is a race condition where, while the thread is still spawning, we can proceed and send messages to a socket that is not bound.
            // This means that the spawned thread would be waiting for a message that it has already missed and which won't get re-sent.
            // There's a timeout at the bottom of this function that was presumably added to fix the issue of this test hanging.
            // This sleep makes it much more likely that the thread has set up everything needed by the tiem we finish this sleep.
            std::thread::sleep(Duration::from_secs(1));

            // Run client proxy in this thread
            rep_server_flow(
                shopping_ssiv,
                client_out_addr,
                client_out,
                client_addr,
                server_addrs[0],
                ipv4_resolve("localhost:23443").unwrap(),
                gossip_addrs.into_iter(),
            )
            .await
        }
        _ => panic!("Invalid opt number"),
    };

    // optionally print the dataflow graph
    #[cfg(feature = "debugging")]
    if let Some(graph) = opts.graph {
        let serde_graph = hf
            .meta_graph()
            .expect("No graph found, maybe failed to parse.");
        serde_graph.open_graph(graph, opts.write_config).unwrap();
    }

    // Run the client for 1 second; should be long enough to get all the results
    let _timeout = tokio::time::timeout(Duration::from_secs(1), hf.run())
        .await
        .unwrap_err();
}
