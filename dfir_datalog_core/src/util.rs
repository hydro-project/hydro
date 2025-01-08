use std::ops::RangeFrom;

use quote::ToTokens;
use syn::parse_quote;

pub type Counter = RangeFrom<isize>;

pub fn repeat_tuple<I: ToTokens, T: syn::parse::Parse>(rep: fn() -> I, n: usize) -> T {
    let values = (1..=n).map(|_| rep()).collect::<Vec<I>>();

    parse_quote!((#(#values, )*))
}
