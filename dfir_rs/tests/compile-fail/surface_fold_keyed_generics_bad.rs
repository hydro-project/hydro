fn main() {
    let mut df = dfir_rs::dfir_syntax! {
        source_iter(["hello", "world"])
            -> fold_keyed::<'tick, &str, usize>(String::new, |old: &mut _, val| {
                *old += val;
            })
            -> for_each(|(k, v)| println!("({:?}, {:?})", k, v));
    };
    df.run_available_sync();
}
