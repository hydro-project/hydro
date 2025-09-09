use hydro_lang::prelude::*;

pub struct Leader {}
pub struct Worker {}

pub fn map_reduce<'a>(flow: &FlowBuilder<'a>) -> (Process<'a, Leader>, Cluster<'a, Worker>) {
    let process = flow.process();
    let cluster = flow.cluster();

    let words = process
        .source_iter(q!(vec!["abc", "abc", "xyz", "abc"]))
        .map(q!(|s| s.to_string()));

    let partitioned_words = words
        .round_robin_bincode(&cluster, nondet!(/** test */))
        .map(q!(|string| (string, ())))
        .into_keyed();

    let batches = partitioned_words
        .batch(
            &cluster.tick(),
            nondet!(/** addition is associative so we can batch reduce */),
        )
        .fold(q!(|| 0), q!(|count, _| *count += 1))
        .entries()
        .inspect(q!(|(string, count)| println!(
            "partition count: {} - {}",
            string, count
        )))
        .all_ticks()
        .send_bincode(&process)
        .values();

    let reduced = batches
        .into_keyed()
        .reduce_commutative(q!(|total, count| *total += count));

    reduced
        .snapshot(&process.tick(), nondet!(/** intentional output */))
        .entries()
        .all_ticks()
        .for_each(q!(|(string, count)| println!("{}: {}", string, count)));

    (process, cluster)
}

#[cfg(test)]
mod tests {
    use hydro_lang::deploy::HydroDeploy;

    #[test]
    fn map_reduce_ir() {
        let builder = hydro_lang::compile::builder::FlowBuilder::new();
        let _ = super::map_reduce(&builder);
        let built = builder.with_default_optimize::<HydroDeploy>();

        hydro_build_utils::assert_debug_snapshot!(built.ir());

        for (id, ir) in built.preview_compile().all_dfir() {
            hydro_build_utils::insta::with_settings!({
                snapshot_suffix => format!("surface_graph_{id}")
            }, {
                hydro_build_utils::assert_snapshot!(ir.surface_syntax_string());
            });
        }
    }
}
