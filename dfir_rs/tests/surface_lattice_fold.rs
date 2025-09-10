use dfir_rs::dfir_syntax;
use dfir_rs::lattices::{IsTop, Max, Merge};
use dfir_rs::util::collect_ready;

#[test]
fn test_basic() {
    let mut df = dfir_syntax! {
        source_iter([1,2,3,4,5])
            -> map(Max::new)
            -> lattice_fold::<'static>(|| Max::<u32>::new(0))
            -> for_each(|x: Max<u32>| println!("Least upper bound: {:?}", x));
    };
    df.run_available_sync();
}

#[test]
fn test_fold_loop() {
    let (output_send, output_recv) = dfir_rs::util::unbounded_channel::<u8>();
    let mut df = dfir_syntax! {
        start = source_iter([1])
            -> map(Max::new)
            -> folder;
        folder = union()
            -> fold::<'static>(|| Max::<u8>::new(0), |accum, x| { accum.merge(x); })
            -> map(|x| Max::<u8>::new(x.into_reveal() + 1))
            -> filter(|x| !x.is_top())
            -> tee();
        folder -> folder;
        folder -> inspect(|v| println!("{:?}", v))
            -> for_each(|v: Max<u8>| output_send.send(*v.as_reveal_ref()).unwrap());
    };
    df.run_tick_sync();
    assert_eq!(&[2], &*collect_ready::<Vec<_>, _>(output_recv));
}

#[test]
fn test_lattice_fold_loop() {
    let (output_send, output_recv) = dfir_rs::util::unbounded_channel::<u8>();
    let mut df = dfir_syntax! {
        start = source_iter([1])
            -> map(Max::<u8>::new)
            -> folder;
        folder = union()
            -> lattice_fold::<'static>(|| Max::<u8>::new(0))
            -> map(|x| Max::<u8>::new(x.into_reveal() + 1))
            -> filter(|x| !x.is_top())
            -> tee();
        folder -> folder;
        folder
            -> for_each(|v: Max<u8>| output_send.send(*v.as_reveal_ref()).unwrap());
    };
    df.run_tick_sync();
    assert_eq!(
        &(2..=254).collect::<Vec<u8>>(),
        &*collect_ready::<Vec<_>, _>(output_recv)
    );
}
