use dfir_rs::scheduled::graph::Dfir;
use dfir_rs::tokio_stream::wrappers::UnboundedReceiverStream;
use hydro_lang::deploy::SingleProcessGraph;
use hydro_lang::*;
use stageleft::{Quoted, RuntimeData};
use tokio::sync::mpsc::UnboundedSender;

pub fn count_elems_generic<'a, T: 'a>(
    flow: FlowBuilder<'a>,
    input_stream: RuntimeData<UnboundedReceiverStream<T>>,
    output: RuntimeData<&'a UnboundedSender<u32>>,
) -> impl Quoted<'a, Dfir<'a>> {
    let process = flow.process::<()>();
    let tick = process.tick();

    let source = process.source_stream(input_stream);
    let count = unsafe {
        // SAFETY: intentionally using ticks
        source.map(q!(|_| 1)).tick_batch(&tick)
    }
    .fold(q!(|| 0), q!(|a, b| *a += b))
    .all_ticks();

    count.for_each(q!(|v| {
        output.send(v).unwrap();
    }));

    flow.compile_no_network::<SingleProcessGraph>()
}

#[stageleft::entry]
pub fn count_elems<'a>(
    flow: FlowBuilder<'a>,
    input_stream: RuntimeData<UnboundedReceiverStream<usize>>,
    output: RuntimeData<&'a UnboundedSender<u32>>,
) -> impl Quoted<'a, Dfir<'a>> {
    count_elems_generic(flow, input_stream, output)
}

#[cfg(stageleft_runtime)]
#[cfg(test)]
mod tests {
    use dfir_rs::assert_graphvis_snapshots;
    use dfir_rs::util::collect_ready;

    #[test]
    pub fn test_count() {
        let (in_send, input) = dfir_rs::util::unbounded_channel();
        let (out, mut out_recv) = dfir_rs::util::unbounded_channel();

        let mut count = super::count_elems!(input, &out);
        assert_graphvis_snapshots!(count);

        in_send.send(1).unwrap();
        in_send.send(1).unwrap();
        in_send.send(1).unwrap();

        count.run_tick();

        assert_eq!(&*collect_ready::<Vec<_>, _>(&mut out_recv), &[3]);
    }
}
