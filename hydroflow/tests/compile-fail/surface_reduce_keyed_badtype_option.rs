fn main() {
    let mut df = hydroflow::hydroflow_syntax! {
        source_iter([ Some(5), None, Some(12) ])
            -> fold_keyed(|| 0, |old: &mut u32, val: u32| { *old += val; })
            -> for_each(std::mem::drop);
    };
    df.run_available();
}
