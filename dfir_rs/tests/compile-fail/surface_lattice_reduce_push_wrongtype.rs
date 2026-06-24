use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        my_tee = source_iter([1, 2, 3, 4, 5]) -> tee();
        my_tee -> lattice_reduce() -> for_each(|x| println!("{:?}", x));
        my_tee -> null();
    };
    df.run_available_sync();
}
