#![allow(unexpected_cfgs)]

use hydro_lang::prelude::*;
use hydro_lang::sim_hooks::{OnMember, OrderingHook};

fn main() {
    let mut flow = FlowBuilder::new();

    // `OnMember` handles only arise from `.on(member_id)` on an `OnCluster`-scoped
    // handle at scripting time; they cannot be created (or bound) directly.
    let _hook: OrderingHook<u32, Unbounded, OnMember> = flow.sim_hook();
}
