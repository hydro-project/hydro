use std::mem::size_of;

use hydro_lang::ir::{HydroLeaf, HydroNode};

fn main() {
    println!("HydroNode size: {}", size_of::<HydroNode>());
    println!("HydroLeaf size: {}", size_of::<HydroLeaf>());
}
