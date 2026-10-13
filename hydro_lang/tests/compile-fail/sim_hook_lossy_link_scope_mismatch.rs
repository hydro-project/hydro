#![allow(unexpected_cfgs)]

use hydro_lang::prelude::*;
use hydro_lang::sim_hooks::LossHook;

struct Workers {}

fn test<'a>(p1: &Process<'a>, workers: &Cluster<'a, Workers>, hook: LossHook<u32>) {
    let numbers = p1.source_iter(q!(vec![123u32]));

    // A loss hook is typed by the channel's endpoints: this channel's receiving end is
    // a cluster, so a process-to-process-shaped `LossHook<u32>` does not fit the link
    // (its receiver scope must be `OnCluster<Workers>`).
    numbers.broadcast_closed(
        workers,
        TCP.lossy(nondet!(/** test */ hook = hook)).bincode(),
    );
}

fn main() {}
