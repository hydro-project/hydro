use hydro_lang::live_collections::stream::{AtLeastOnce, NoOrder};
use hydro_lang::prelude::*;

pub fn capitalize<'a>(input: Stream<String, Process<'a, ()>>) {
    input
        .map(q!(|s| s.to_uppercase()))
        .embedded_output("output");
}

/// Like [`capitalize`], but emits an unordered, at-least-once stream, which `embedded_output`
/// also accepts.
pub fn capitalize_unordered<'a>(input: Stream<String, Process<'a, ()>>) {
    input
        .map(q!(|s| s.to_uppercase()))
        .weaken_ordering::<NoOrder>()
        .weaken_retries::<AtLeastOnce>()
        .embedded_output("output");
}
