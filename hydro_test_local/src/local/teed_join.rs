use dfir_rs::scheduled::graph::Dfir;
use futures::stream::Stream;
use hydro_lang::deploy::MultiGraph;
use hydro_lang::*;
use stageleft::{Quoted, RuntimeData};
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::wrappers::UnboundedReceiverStream;

struct N0 {}
struct N1 {}

#[stageleft::entry(UnboundedReceiverStream<u32>)]
pub fn teed_join<'a, S: Stream<Item = u32> + Unpin + 'a>(
    flow: FlowBuilder<'a>,
    input_stream: RuntimeData<S>,
    output: RuntimeData<&'a UnboundedSender<u32>>,
    send_twice: bool,
    subgraph_id: RuntimeData<usize>,
) -> impl Quoted<'a, Dfir<'a>> {
    let node_zero = flow.process::<N0>();
    let node_one = flow.process::<N1>();
    let n0_tick = node_zero.tick();

    let source = unsafe {
        // SAFETY: intentionally using ticks
        node_zero.source_stream(input_stream).tick_batch(&n0_tick)
    };
    let map1 = source.clone().map(q!(|v| (v + 1, ())));
    let map2 = source.map(q!(|v| (v - 1, ())));

    let joined = map1.join(map2).map(q!(|t| t.0));

    joined.clone().all_ticks().for_each(q!(|v| {
        output.send(v).unwrap();
    }));

    if send_twice {
        joined.all_ticks().for_each(q!(|v| {
            output.send(v).unwrap();
        }));
    }

    let source_node_id_1 = node_one.source_iter(q!(0..5));
    source_node_id_1.for_each(q!(|v| {
        output.send(v).unwrap();
    }));

    flow.compile_no_network::<MultiGraph>()
        .with_dynamic_id(subgraph_id)
}

#[cfg(stageleft_runtime)]
#[cfg(test)]
mod tests {
    use dfir_rs::assert_graphvis_snapshots;
    use dfir_rs::util::collect_ready;

    #[test]
    fn test_teed_join() {
        let (in_send, input) = dfir_rs::util::unbounded_channel();
        let (out, mut out_recv) = dfir_rs::util::unbounded_channel();

        let mut joined = super::teed_join!(input, &out, false, 0);
        assert_graphvis_snapshots!(joined);

        in_send.send(1).unwrap();
        in_send.send(2).unwrap();
        in_send.send(3).unwrap();
        in_send.send(4).unwrap();

        joined.run_tick();

        assert_eq!(&*collect_ready::<Vec<_>, _>(&mut out_recv), &[2, 3]);
    }

    #[test]
    fn test_teed_join_twice() {
        let (in_send, input) = dfir_rs::util::unbounded_channel();
        let (out, mut out_recv) = dfir_rs::util::unbounded_channel();

        let mut joined = super::teed_join!(input, &out, true, 0);
        assert_graphvis_snapshots!(joined);

        in_send.send(1).unwrap();
        in_send.send(2).unwrap();
        in_send.send(3).unwrap();
        in_send.send(4).unwrap();

        joined.run_tick();

        assert_eq!(&*collect_ready::<Vec<_>, _>(&mut out_recv), &[2, 2, 3, 3]);
    }

    #[test]
    fn test_teed_join_multi_node() {
        let (_, input) = dfir_rs::util::unbounded_channel();
        let (out, mut out_recv) = dfir_rs::util::unbounded_channel();

        let mut joined = super::teed_join!(input, &out, true, 1);
        assert_graphvis_snapshots!(joined);

        joined.run_tick();

        assert_eq!(
            &*collect_ready::<Vec<_>, _>(&mut out_recv),
            &[0, 1, 2, 3, 4]
        );
    }
}
