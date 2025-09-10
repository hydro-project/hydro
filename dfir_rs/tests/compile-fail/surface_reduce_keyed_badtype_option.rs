fn main() {
    let mut df = dfir_rs::dfir_syntax! {
        source_iter([ Some(5), None, Some(12) ])
            -> fold_keyed(|| 0, |old: &mut u32, val: u32| { *old += val; })
            -> for_each(std::mem::drop);
    };
    df.run_available_sync();
}
