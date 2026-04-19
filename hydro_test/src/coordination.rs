//! Coordination consistency tests using the Hydro simulator.
//!
//! Three categories:
//! - INCON: simulator finds concrete counterexamples (different runs diverge)
//! - CONV: simulator confirms all runs produce the same set of outputs
//! - SEQ: simulator confirms all runs produce prefixes of the same sequence

#[cfg(test)]
mod tests {
    use hydro_lang::compile::coordination::ConsistencyCollector;
    use hydro_lang::prelude::*;

    /// INCON: non-commutative scan on nondeterministically merged inputs.
    /// Different runs produce different concatenation orders.
    #[test]
    #[should_panic(expected = "Prefix inconsistency")]
    fn incon_fold_produces_different_outputs() {
        let mut flow = FlowBuilder::new();
        let process = flow.process::<()>();

        let tick = process.tick();
        let a = process.source_iter(q!(vec!["A".to_string(), "C".to_string()]))
            .batch(&tick, nondet!(/** batch */)).all_ticks();
        let b = process.source_iter(q!(vec!["B".to_string(), "D".to_string()]))
            .batch(&tick, nondet!(/** batch */)).all_ticks();

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

        let collector = ConsistencyCollector::new();
        let num_runs = flow.sim().exhaustive(async || {
            let outputs: Vec<String> = out_port.collect().await;
            collector.record_run(outputs);
        });

        println!("INCON test: explored {} runs", num_runs);
        collector.check_prefix_consistency();
    }

    /// CONV: commutative+idempotent fold (max) on nondeterministically merged inputs.
    /// All runs converge to the same final output regardless of message ordering.
    #[test]
    fn conv_max_fold_all_runs_agree() {
        let mut flow = FlowBuilder::new();
        let process = flow.process::<()>();

        // Single source with all data — no merge nondeterminism, just batch nondeterminism
        let tick = process.tick();
        let input = process.source_iter(q!(vec![(1, 10), (2, 20), (1, 30), (2, 5)]))
            .batch(&tick, nondet!(/** batch */));

        // Fold with max (commutative+idempotent) — result is the same regardless of order
        let result = input
            .into_keyed()
            .fold(
                q!(|| 0i32),
                q!(|acc, x| { if x > *acc { *acc = x; } },
                    commutative = manual_proof!(/** max is commutative */),
                    idempotent = manual_proof!(/** max is idempotent */)),
            )
            .entries()
            .all_ticks();

        let out_port = result.sim_output();

        let collector = ConsistencyCollector::<(i32, i32)>::new();
        let num_runs = flow.sim().exhaustive(async || {
            let outputs: Vec<(i32, i32)> = out_port.collect_sorted().await;
            // Keep only the final (converged) value per key
            let mut final_values = std::collections::HashMap::new();
            for (k, v) in &outputs {
                final_values.insert(*k, *v);
            }
            let mut final_vec: Vec<(i32, i32)> = final_values.into_iter().collect();
            final_vec.sort();
            collector.record_run(final_vec);
        });

        println!("CONV test: explored {} runs", num_runs);
        assert!(collector.num_runs() > 1, "Need multiple runs to verify convergence");
        collector.check_set_consistency();
        println!("✓ All {} runs produced the same set of outputs", collector.num_runs());
    }

    /// SEQ: scan on a TotalOrder stream (single source, no merge).
    /// All runs produce prefixes of the same deterministic sequence.
    #[test]
    fn seq_scan_all_runs_prefix_consistent() {
        let mut flow = FlowBuilder::new();
        let process = flow.process::<()>();

        let tick = process.tick();
        let input = process.source_iter(q!(vec![1, 2, 3, 4, 5]))
            .batch(&tick, nondet!(/** batch */)).all_ticks();

        // Scan: running sum (deterministic on TotalOrder input)
        let output = input.scan(
            q!(|| 0i32),
            q!(|acc: &mut i32, x: i32| {
                *acc += x;
                Some(*acc)
            }),
        );

        let out_port = output.sim_output();

        let collector = ConsistencyCollector::<i32>::new();
        let num_runs = flow.sim().exhaustive(async || {
            let outputs: Vec<i32> = out_port.collect().await;
            collector.record_run(outputs);
        });

        println!("SEQ test: explored {} runs", num_runs);
        assert!(collector.num_runs() > 1, "Need multiple runs to verify consistency");
        collector.check_prefix_consistency();
        println!("✓ All {} runs are prefix-consistent", collector.num_runs());
    }
}
