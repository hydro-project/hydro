use hydro_lang::live_collections::stream::{AtLeastOnce, NoOrder};
use hydro_lang::prelude::*;

/// Emits each distinct name once. The input is `NoOrder` and `AtLeastOnce`, for a host whose
/// transport may reorder names and deliver a name more than once.
pub fn unique_names<'a>(input: Stream<String, Process<'a, ()>, Unbounded, NoOrder, AtLeastOnce>) {
    input.unique().embedded_output("output");
}
