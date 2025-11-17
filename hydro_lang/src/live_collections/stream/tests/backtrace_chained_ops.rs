// separate file for stable line numbers

#[cfg(feature = "build")]
#[test]
fn backtrace_chained_ops() {
    if cfg!(not(target_os = "linux")) && std::env::var_os("GITHUB_ACTIONS").is_some() {
        eprintln!("Backtrace tests fail on non-linux Github Actions runners, skipping.");
        return;
    }

    use stageleft::q;

    use crate::compile::ir::HydroRoot;
    use crate::location::Location;
    use crate::prelude::FlowBuilder;

    let flow = FlowBuilder::new();
    let node = flow.process::<()>();

    node.source_iter(q!([123])).for_each(q!(|_| {}));

    let finalized: crate::compile::built::BuiltFlow<'_> = flow.finalize();

    let source_meta = if let HydroRoot::ForEach { input, .. } = &finalized.ir()[0] {
        use crate::compile::ir::HydroNode;

        if let HydroNode::Source { metadata, .. } = input.as_ref() {
            &metadata.op
        } else {
            panic!()
        }
    } else {
        panic!()
    };
    let for_each_meta = finalized.ir()[0].op_metadata();

    hydro_build_utils::assert_debug_snapshot!(source_meta.backtrace.elements());
    hydro_build_utils::assert_debug_snapshot!(for_each_meta.backtrace.elements());
}
