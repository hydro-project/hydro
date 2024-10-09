use hydroflow_plus::deploy::SingleProcessGraph;
use hydroflow_plus::tokio::sync::mpsc::UnboundedSender;
use hydroflow_plus::tokio_stream::wrappers::UnboundedReceiverStream;
use hydroflow_plus::*;
use stageleft::*;

#[stageleft::entry]
pub fn graph_reachability<'a>(
    flow: FlowBuilder<'a>,
    roots: RuntimeData<UnboundedReceiverStream<u32>>,
    edges: RuntimeData<UnboundedReceiverStream<(u32, u32)>>,
    reached_out: RuntimeData<&'a UnboundedSender<u32>>,
) -> impl Quoted<'a, Hydroflow<'a>> {
    let process = flow.process::<()>();

    let roots = process.source_stream(roots);
    let edges = process.source_stream(edges);

    let (set_reached_cycle, reached_cycle) = process.cycle();

    let reached = roots.union(reached_cycle);
    let reachable = reached
        .clone()
        .map(q!(|r| (r, ())))
        .join(edges)
        .map(q!(|(_from, (_, to))| to));
    set_reached_cycle.complete(reachable);

    reached.unique().for_each(q!(|v| {
        reached_out.send(v).unwrap();
    }));

    flow.with_default_optimize()
        .compile_no_network::<SingleProcessGraph>()
}

#[stageleft::runtime]
#[cfg(test)]
mod tests {
    use hydroflow_plus::assert_graphvis_snapshots;
    use hydroflow_plus::util::collect_ready;

    #[test]
    pub fn test_reachability() {
        let (roots_send, roots) = hydroflow_plus::util::unbounded_channel();
        let (edges_send, edges) = hydroflow_plus::util::unbounded_channel();
        let (out, mut out_recv) = hydroflow_plus::util::unbounded_channel();

        let mut reachability = super::graph_reachability!(roots, edges, &out);
        assert_graphvis_snapshots!(reachability);

        roots_send.send(1).unwrap();
        roots_send.send(2).unwrap();

        edges_send.send((1, 2)).unwrap();
        edges_send.send((2, 3)).unwrap();
        edges_send.send((3, 4)).unwrap();
        edges_send.send((4, 5)).unwrap();

        reachability.run_tick();

        assert_eq!(
            &*collect_ready::<Vec<_>, _>(&mut out_recv),
            &[1, 2, 3, 4, 5]
        );
    }
}
