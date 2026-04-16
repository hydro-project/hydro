use dfir_rs::dfir_syntax;
use dfir_rs::lattices::set_union::SetUnionHashSet;

fn main() {
    let mut df = dfir_syntax! {
        source_iter([1,2,3,4,5])
            -> lattice_fold::<'static, SetUnionHashSet<u32>>(SetUnionHashSet::<u32>::default())
            -> for_each(|x| println!("Least upper bound: {:?}", x));
    };
    df.run_available_sync();
}
