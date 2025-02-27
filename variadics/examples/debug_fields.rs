//! https://poignardazur.github.io/2023/11/08/time-for-variadic-generics/

use std::fmt::{Debug, DebugTuple, Formatter, Result};

use variadics::{Variadic, var_args, var_expr, var_type};

#[sealed::sealed]
pub trait DebugVariadicTrait {
    fn apply(&self, f: &mut DebugTuple);
}
#[sealed::sealed]
impl DebugVariadicTrait for () {
    fn apply(&self, _f: &mut DebugTuple) {}
}
#[sealed::sealed]
impl<Head: Debug, Tail: Variadic + DebugVariadicTrait> DebugVariadicTrait for var_type!(Head, ...Tail) {
    fn apply(&self, f: &mut DebugTuple) {
        let var_args!(head, ...tail) = self;
        f.field(head);
        tail.apply(f);
    }
}

pub struct DebugVariadic<V: DebugVariadicTrait>(V);
impl<V: DebugVariadicTrait> Debug for DebugVariadic<V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.0.apply(&mut f.debug_tuple(""));
        Ok(())
    }
}

fn main() {
    let var_tuple = var_expr!("hello", 1, 'w');
    let var_tuple = DebugVariadic(var_tuple);
    println!("{:?}", var_tuple);
}

// impl<Ts: Variadic + Debug> Debug for Ts {
//     fn fmt(&self, f: &mut Formatter<'_>) -> Result {
//         let mut f = f.debug_tuple("");
//         self.apply(f);
//         f.finish()
//     }
// }
