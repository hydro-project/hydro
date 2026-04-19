//! Coordination consistency tests using the Hydro simulator.
//!
//! Demonstrates that the coordination analysis correctly identifies
//! inconsistent programs by finding concrete counterexamples via
//! the simulator's exhaustive mode.

#[cfg(test)]
mod tests {
    use hydro_lang::compile::coordination::ConsistencyCollector;
    use hydro_lang::prelude::*;

    /// A non-commutative fold: concatenates strings in arrival order.
    /// Two concurrent senders produce different results depending on
    /// message ordering. The coordination analysis flags this as INCON
    /// (because the fold lacks commutativity proof on unbounded input).
    ///
    /// This test uses the simulator to find a concrete inconsistency:
    /// different runs produce different output strings.
    #[test]
    #[should_panic(expected = "Prefix inconsistency")]
    fn incon_fold_produces_different_outputs() {
        let mut flow = FlowBuilder::new();
        let process = flow.process::<()>();

        // Two sources that will be merged — order is nondeterministic
        let tick = process.tick();
        let a = process.source_iter(q!(vec!["A".to_string(), "C".to_string()]))
            .batch(&tick, nondet!(/** batch */)).all_ticks();
        let b = process.source_iter(q!(vec!["B".to_string(), "D".to_string()]))
            .batch(&tick, nondet!(/** batch */)).all_ticks();

        // Merge (nondeterministic order) then scan (non-commutative: string concat)
        let merged = a.merge_unordered(b)
            .assume_ordering::<hydro_lang::live_collections::stream::TotalOrder>(
                nondet!(/** nondeterministic merge order */)
            );

        let output = merged.scan(
            q!(|| String::new()),
            q!(|acc: &mut String, x: String| {
                acc.push_str(&x);
                acc.push_str(",");
                Some(acc.clone())
            }),
        );

        let out_port = output.sim_output();

        // The analysis will flag this as INCON because merge_unordered
        // breaks prefix order and the scan depends on arrival order.
        let collector = ConsistencyCollector::new();
        let num_runs = flow.sim().exhaustive(async || {
            let outputs: Vec<String> = out_port.collect().await;
            collector.record_run(outputs);
        });

        println!("Explored {} runs", num_runs);

        // This will panic with "Prefix inconsistency" — caught by #[should_panic]
        collector.check_prefix_consistency();
    }
}
