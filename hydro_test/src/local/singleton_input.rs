use hydro_lang::prelude::*;

pub fn prefix_names<'a>(
    names: Stream<String, Process<'a, ()>>,
    prefix: Singleton<String, Process<'a, ()>, Bounded>,
) {
    names
        .cross_singleton(prefix)
        .map(q!(|(name, prefix)| format!("{prefix} {name}")))
        .embedded_output("output");
}
