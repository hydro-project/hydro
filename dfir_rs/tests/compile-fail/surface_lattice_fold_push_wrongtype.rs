use dfir_rs::dfir_syntax;
use dfir_rs::lattices::set_union::SetUnionHashSet;

fn main() {
    let mut df = dfir_syntax! {
        my_tee = source_iter([1, 2, 3, 4, 5]) -> tee();
        my_tee -> lattice_fold::<'static>(SetUnionHashSet::<u32>::default) -> for_each(|x| println!("{:?}", x));
        my_tee -> null();
    };
    df.run_available_sync();
}
