use hydro_lang::prelude::*;

pub fn capitalize<'a>(input: Stream<String, Process<'a, ()>>) {
    input
        .map(q!(|s| s.to_uppercase()))
        .embedded_output("output");
}
