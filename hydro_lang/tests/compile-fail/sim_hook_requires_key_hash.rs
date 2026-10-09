use hydro_lang::compile::builder::FlowBuilder;
use hydro_lang::sim_hooks::KeyedMergeOrderedHook;

// Deliberately does not implement `Hash`.
#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct NotHashable(u32);

fn main() {
    let mut flow = FlowBuilder::new();

    // The simulator buffers keyed hooks per key in a hash map, so handle creation
    // requires `K: Hash`: the error is reported here, at the `sim_hook()` call,
    // rather than as a rustc failure inside the generated simulation dylib.
    let _hook: KeyedMergeOrderedHook<NotHashable, u32> = flow.sim_hook();
}
