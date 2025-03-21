use dfir_rs::scheduled::graph::Dfir;
use hydro_lang::deploy::SingleProcessGraph;
use hydro_lang::*;
use stageleft::{Quoted, RuntimeData};
use tokio::sync::mpsc::UnboundedSender;

#[stageleft::entry]
pub fn test_difference<'a>(
    flow: FlowBuilder<'a>,
    output: RuntimeData<&'a UnboundedSender<u32>>,
    persist1: bool,
    persist2: bool,
) -> impl Quoted<'a, Dfir<'a>> {
    let process = flow.process::<()>();
    let tick = process.tick();

    let mut source = unsafe {
        // SAFETY: intentionally using ticks
        process.source_iter(q!(0..5)).tick_batch(&tick)
    };
    if persist1 {
        source = source.persist();
    }

    let mut source2 = unsafe {
        // SAFETY: intentionally using ticks
        process.source_iter(q!(3..6)).tick_batch(&tick)
    };
    if persist2 {
        source2 = source2.persist();
    }

    source.filter_not_in(source2).all_ticks().for_each(q!(|v| {
        output.send(v).unwrap();
    }));

    flow.compile_no_network::<SingleProcessGraph>()
}

#[stageleft::entry]
pub fn test_anti_join<'a>(
    flow: FlowBuilder<'a>,
    output: RuntimeData<&'a UnboundedSender<u32>>,
    persist1: bool,
    persist2: bool,
) -> impl Quoted<'a, Dfir<'a>> {
    let process = flow.process::<()>();
    let tick = process.tick();

    let mut source = unsafe {
        // SAFETY: intentionally using ticks
        process
            .source_iter(q!(0..5))
            .map(q!(|v| (v, v)))
            .tick_batch(&tick)
    };
    if persist1 {
        source = source.persist();
    }

    let mut source2 = unsafe {
        // SAFETY: intentionally using ticks
        process.source_iter(q!(3..6)).tick_batch(&tick)
    };
    if persist2 {
        source2 = source2.persist();
    }

    source.anti_join(source2).all_ticks().for_each(q!(|v| {
        output.send(v.0).unwrap();
    }));

    flow.compile_no_network::<SingleProcessGraph>()
}

#[cfg(stageleft_runtime)]
#[cfg(test)]
mod tests {
    use dfir_rs::assert_graphvis_snapshots;
    use dfir_rs::util::collect_ready;

    #[test]
    fn test_difference_tick_tick() {
        let (out, mut out_recv) = dfir_rs::util::unbounded_channel();

        let mut flow = super::test_difference!(&out, false, false);
        assert_graphvis_snapshots!(flow);

        flow.run_tick();

        assert_eq!(&*collect_ready::<Vec<_>, _>(&mut out_recv), &[0, 1, 2]);

        flow.run_tick();

        assert_eq!(&*collect_ready::<Vec<_>, _>(&mut out_recv), &[] as &[u32]);
    }

    #[test]
    fn test_difference_tick_static() {
        let (out, mut out_recv) = dfir_rs::util::unbounded_channel();

        let mut flow = super::test_difference!(&out, false, true);
        assert_graphvis_snapshots!(flow);

        flow.run_tick();

        assert_eq!(&*collect_ready::<Vec<_>, _>(&mut out_recv), &[0, 1, 2]);

        flow.run_tick();

        assert_eq!(&*collect_ready::<Vec<_>, _>(&mut out_recv), &[] as &[u32]);
    }

    #[test]
    fn test_difference_static_tick() {
        let (out, mut out_recv) = dfir_rs::util::unbounded_channel();

        let mut flow = super::test_difference!(&out, true, false);
        assert_graphvis_snapshots!(flow);

        flow.run_tick();

        assert_eq!(&*collect_ready::<Vec<_>, _>(&mut out_recv), &[0, 1, 2]);

        flow.run_tick();

        assert_eq!(
            &*collect_ready::<Vec<_>, _>(&mut out_recv),
            &[0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn test_difference_static_static() {
        let (out, mut out_recv) = dfir_rs::util::unbounded_channel();

        let mut flow = super::test_difference!(&out, true, true);
        assert_graphvis_snapshots!(flow);

        flow.run_tick();

        assert_eq!(&*collect_ready::<Vec<_>, _>(&mut out_recv), &[0, 1, 2]);

        flow.run_tick();

        assert_eq!(&*collect_ready::<Vec<_>, _>(&mut out_recv), &[0, 1, 2]);
    }

    #[test]
    fn test_anti_join_tick_tick() {
        let (out, mut out_recv) = dfir_rs::util::unbounded_channel();

        let mut flow = super::test_anti_join!(&out, false, false);
        assert_graphvis_snapshots!(flow);

        flow.run_tick();

        assert_eq!(&*collect_ready::<Vec<_>, _>(&mut out_recv), &[0, 1, 2]);

        flow.run_tick();

        assert_eq!(&*collect_ready::<Vec<_>, _>(&mut out_recv), &[] as &[u32]);
    }

    #[test]
    fn test_anti_join_tick_static() {
        let (out, mut out_recv) = dfir_rs::util::unbounded_channel();

        let mut flow = super::test_anti_join!(&out, false, true);
        assert_graphvis_snapshots!(flow);

        flow.run_tick();

        assert_eq!(&*collect_ready::<Vec<_>, _>(&mut out_recv), &[0, 1, 2]);

        flow.run_tick();

        assert_eq!(&*collect_ready::<Vec<_>, _>(&mut out_recv), &[] as &[u32]);
    }

    #[test]
    fn test_anti_join_static_tick() {
        let (out, mut out_recv) = dfir_rs::util::unbounded_channel();

        let mut flow = super::test_anti_join!(&out, true, false);
        assert_graphvis_snapshots!(flow);

        flow.run_tick();

        assert_eq!(&*collect_ready::<Vec<_>, _>(&mut out_recv), &[0, 1, 2]);

        flow.run_tick();

        assert_eq!(
            &*collect_ready::<Vec<_>, _>(&mut out_recv),
            &[0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn test_anti_join_static_static() {
        let (out, mut out_recv) = dfir_rs::util::unbounded_channel();

        let mut flow = super::test_anti_join!(&out, true, true);
        assert_graphvis_snapshots!(flow);

        flow.run_tick();

        assert_eq!(&*collect_ready::<Vec<_>, _>(&mut out_recv), &[0, 1, 2]);

        flow.run_tick();

        assert_eq!(&*collect_ready::<Vec<_>, _>(&mut out_recv), &[0, 1, 2]);
    }
}
