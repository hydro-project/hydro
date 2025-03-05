fn main() {
    let mut df = dfir_rs::dfir_syntax! {
        a = source_iter(0..10) -> tee();
        loop {
            a -> batch() -> fold::<'tick>(|| 0, |old: &mut _, val| {
                *old += val;
            }) -> for_each(|v| println!("{:?}", v));

            a -> batch() -> fold::<'static>(|| 0, |old: &mut _, val| {
                *old += val;
            }) -> for_each(|v| println!("{:?}", v));
        };
    };
    df.run_available();
}
