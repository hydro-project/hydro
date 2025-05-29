use hydro_lang::*;

pub struct Server {}

pub struct Clients {}

pub fn echo_server<'a>(flow: &FlowBuilder<'a>) -> (Process<'a, Server>, Cluster<'a, Clients>) {
    // For testing, a fixed cluster of clients.
    let clients = flow.cluster::<Clients>();

    // Assume single server.
    let server = flow.process::<Server>();

    // assume 1 echo request is generated from each client
    let client_requests = clients
        .source_iter(q!([format!("src: {}", CLUSTER_SELF_ID.raw_id)]))
        .send_bincode(&server)
        .clone()
        .inspect(q!(|(id, t)| println!(
            "...received request {} from client #{}, echoing back...",
            t, id.raw_id
        )));
    client_requests
        .send_bincode(&clients)
        .for_each(q!(|t| println!("received response {}", t)));

    (server, clients)
}
