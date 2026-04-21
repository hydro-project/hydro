fn main() {
    let mut df = dfir_rs::dfir_syntax! {
        a = source_iter(0..10)
            -> fold::<'loop>(|| 0, |old: &mut _, val| {
                *old += val;
            })
            -> for_each(|v| println!("{:?}", v));
    };
    df.run_available_sync();
}
