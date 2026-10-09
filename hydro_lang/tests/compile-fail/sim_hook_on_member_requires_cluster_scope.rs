#![allow(unexpected_cfgs)]

use hydro_lang::prelude::*;
use hydro_lang::sim_hooks::OrderingHook;

fn main() {
    let mut flow = FlowBuilder::new();

    // `.on(member_id)` selects one cluster member's instance of the hooked operator, so
    // it only exists on `OnCluster`-scoped handles; this handle is scoped to a process
    // (the default), whose operator has exactly one instance.
    let ordering: OrderingHook<u32> = flow.sim_hook();
    let _scoped = ordering.on(0);
}
