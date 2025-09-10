use dfir_rs::dfir_syntax;
use dfir_rs::lattices::Max;

#[test]
fn test_basic() {
    let mut df = dfir_syntax! {
        source_iter([1,2,3,4,5])
            -> map(Max::new)
            -> lattice_reduce()
            -> for_each(|x: Max<u32>| println!("Least upper bound: {:?}", x));
    };
    df.run_available_sync();
}
