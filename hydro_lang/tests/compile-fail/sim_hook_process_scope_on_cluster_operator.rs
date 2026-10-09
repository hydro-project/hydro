#![allow(unexpected_cfgs)]

use hydro_lang::live_collections::stream::{NoOrder, TotalOrder};
use hydro_lang::nondet::NonDet;
use hydro_lang::prelude::*;
use hydro_lang::sim_hooks::OrderingHook;

struct Workers {}

fn main() {
    let mut flow = FlowBuilder::new();
    let cluster = flow.cluster::<Workers>();

    // This handle defaults to the `OnProcess` scope, but the operator below runs on a
    // cluster: it must be created as `OrderingHook<u32, Unbounded, OnCluster<Workers>>`
    // to bind there (and then scripted per member with `.on(member_id)`).
    let ordering: OrderingHook<u32> = flow.sim_hook();
    let guard: NonDet<Option<OrderingHook<u32>>> = nondet!(
        /// scripted
        hook = ordering
    );

    let _ = cluster
        .source_iter(q!(vec![1u32]))
        .weaken_ordering::<NoOrder>()
        .assume_ordering::<TotalOrder>(guard);
}
