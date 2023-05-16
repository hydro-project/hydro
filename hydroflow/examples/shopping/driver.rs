use std::iter::Iterator;

use crate::flows::bp_flow::bp_flow;
use crate::flows::client_state_flow::client_state_flow;
use crate::flows::orig_flow::orig_flow;
use crate::flows::push_group_flow::push_group_flow;
use crate::flows::rep_server_flow::rep_server_flow;
use crate::flows::server_state_flow::server_state_flow;
use crate::flows::ssiv_flow::ssiv_flow;
use crate::test_data::{all_clients_iter, client100_vec, client1_vec, client2_vec};
use crate::wrappers::{bp_wrap, ssiv_wrap, tuple_wrap};
use crate::{GraphType, Opts};

use hydroflow::util::ipv4_resolve;

pub(crate) async fn run_driver(opts: Opts) {
    // port for outputting results
    // XXX set up a listening thread on this port that dumps the results to stdout?
    let out_addr = ipv4_resolve("localhost:23459").unwrap();
    let (out, _, _) = hydroflow::util::bind_udp_bytes(out_addr).await;

    // address for a separate server thread when we need one
    let server_addr = ipv4_resolve("localhost:23456").unwrap();

    // define the shopping workload from the test data
    let shopping = tuple_wrap(all_clients_iter());

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

    // run the chosen dataflow
    let mut hf = match opts.opt {
        1 => orig_flow(shopping, out_addr, out).await,
        2 => bp_flow(shopping_bp, out_addr, out).await,
        3 => ssiv_flow(shopping_ssiv, out_addr, out).await,
        4 => push_group_flow(shopping_ssiv, out_addr, out).await,
        5 => server_state_flow(shopping_ssiv, out_addr, out, server_addr).await,
        6 => client_state_flow(shopping_ssiv, out_addr, out, server_addr).await,
        7 => {
            // define the server addresses
            let addr1 = ipv4_resolve("localhost:23456").unwrap();
            let addr2 = ipv4_resolve("localhost:23457").unwrap();
            let addr3 = ipv4_resolve("localhost:23458").unwrap();
            let server_addrs = vec![addr1, addr2, addr3].into_iter();
            rep_server_flow(shopping_ssiv, out_addr, out, server_addrs).await
        }
        _ => panic!("Invalid opt number"),
    };

    // optionally print the dataflow graph
    if let Some(graph) = opts.graph {
        let serde_graph = hf
            .meta_graph()
            .expect("No graph found, maybe failed to parse.");
        match graph {
            GraphType::Mermaid => {
                println!("{}", serde_graph.to_mermaid());
            }
            GraphType::Dot => {
                println!("{}", serde_graph.to_dot())
            }
            GraphType::Json => {
                unimplemented!();
            }
        }
    }

    // fire up the Hydroflow transducers, one per thread
    match opts.opt {
        // the first 4 options each require only a single machine representing the server
        1 | 2 | 3 | 4 => hf.run_available(),
        // for the next 2 options, this thread is a proxy for the client,
        // and we need to spawn another thread for the server
        5 | 6 => hf.run_available(),
        // the next option requires 3 threads, all of which are server replicas
        7 => hf.run_available(),
        _ => panic!("Invalid opt number"),
    };
}
