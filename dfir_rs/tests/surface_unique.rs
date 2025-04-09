use dfir_rs::util::collect_ready;
use dfir_rs::{assert_graphvis_snapshots, dfir_syntax};
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
pub fn test_unique() {
    let (items_send, items_recv) = dfir_rs::util::unbounded_channel::<usize>();

    let mut df = dfir_syntax! {
        source_stream(items_recv)
            -> unique()
            -> for_each(|v| print!("{:?}, ", v));
    };
    assert_graphvis_snapshots!(df);
    df.run_available();

    print!("\nA: ");

    items_send.send(9).unwrap();
    items_send.send(9).unwrap();
    items_send.send(5).unwrap();
    df.run_available();

    print!("\nB: ");

    items_send.send(9).unwrap();
    items_send.send(9).unwrap();
    items_send.send(2).unwrap();
    items_send.send(0).unwrap();
    items_send.send(2).unwrap();
    df.run_available();

    println!();
}

#[multiplatform_test]
pub fn test_unique_tick_pull() {
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<usize>();

    let mut df = dfir_syntax! {
        source_iter(0..10) -> persist::<'static>() -> m1;
        source_iter(5..15) -> persist::<'static>() -> m1;
        m1 = union() -> unique::<'tick>() -> m2;
        source_iter(0..0) -> persist::<'static>() -> m2; // Extra union to force `unique()` to be pull.
        m2 = union() -> for_each(|v| out_send.send(v).unwrap());
    };
    assert_graphvis_snapshots!(df);
    df.run_tick();
    let mut out: Vec<_> = collect_ready(&mut out_recv);
    out.sort_unstable();
    assert_eq!((0..15).collect::<Vec<_>>(), out);

    df.run_tick();
    let mut out: Vec<_> = collect_ready(&mut out_recv);
    out.sort_unstable();
    assert_eq!((0..15).collect::<Vec<_>>(), out);
}

#[multiplatform_test]
pub fn test_unique_static_pull() {
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<usize>();

    let mut df = dfir_syntax! {
        source_iter(0..10) -> persist::<'static>() -> m1;
        source_iter(5..15) -> persist::<'static>() -> m1;
        m1 = union() -> unique::<'static>() -> m2;
        source_iter(0..0) -> persist::<'static>() -> m2; // Extra union to force `unique()` to be pull.
        m2 = union() -> for_each(|v| out_send.send(v).unwrap());
    };
    assert_graphvis_snapshots!(df);
    df.run_tick();
    let mut out: Vec<_> = collect_ready(&mut out_recv);
    out.sort_unstable();
    assert_eq!((0..15).collect::<Vec<_>>(), out);

    df.run_tick();
    let mut out: Vec<_> = collect_ready(&mut out_recv);
    out.sort_unstable();
    assert_eq!(Vec::<usize>::new(), out);
}

#[multiplatform_test]
pub fn test_unique_tick_push() {
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<usize>();

    let mut df = dfir_syntax! {
        source_iter(0..10) -> persist::<'static>() -> pivot;
        source_iter(5..15) -> persist::<'static>() -> pivot;
        pivot = union() -> tee();
        pivot -> unique::<'tick>() -> for_each(|v| out_send.send(v).unwrap());
        pivot -> for_each(std::mem::drop); // Force to be push.
    };
    assert_graphvis_snapshots!(df);
    df.run_tick();
    let mut out: Vec<_> = collect_ready(&mut out_recv);
    out.sort_unstable();
    assert_eq!((0..15).collect::<Vec<_>>(), out);

    df.run_tick();
    let mut out: Vec<_> = collect_ready(&mut out_recv);
    out.sort_unstable();
    assert_eq!((0..15).collect::<Vec<_>>(), out);
}

#[multiplatform_test]
pub fn test_unique_static_push() {
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<usize>();

    let mut df = dfir_syntax! {
        source_iter(0..10) -> persist::<'static>() -> pivot;
        source_iter(5..15) -> persist::<'static>() -> pivot;
        pivot = union() -> tee();
        pivot -> unique::<'static>() -> for_each(|v| out_send.send(v).unwrap());
        pivot -> for_each(std::mem::drop); // Force to be push.
    };
    assert_graphvis_snapshots!(df);
    df.run_tick();
    let mut out: Vec<_> = collect_ready(&mut out_recv);
    out.sort_unstable();
    assert_eq!((0..15).collect::<Vec<_>>(), out);

    df.run_tick();
    let mut out: Vec<_> = collect_ready(&mut out_recv);
    out.sort_unstable();
    assert_eq!(Vec::<usize>::new(), out);
}

#[multiplatform_test]
pub fn test_loop_lifetime() {
    let (result1_send, mut result1_recv) = dfir_rs::util::unbounded_channel::<_>();
    let (result2_send, mut result2_recv) = dfir_rs::util::unbounded_channel::<_>();
    let mut df = dfir_syntax! {
        init = source_iter(0..5);
        loop {
            batch_init = init -> batch() -> tee();
            loop {
                batch_init -> repeat_n(3) -> unique::<'none>() -> for_each(|x| result1_send.send(x).unwrap());
                batch_init -> repeat_n(3) -> unique::<'loop>() -> for_each(|x| result2_send.send(x).unwrap());
            };
        };
    };
    df.run_available();

    assert_eq!(
        &[0, 1, 2, 3, 4, 0, 1, 2, 3, 4, 0, 1, 2, 3, 4],
        &*collect_ready::<Vec<_>, _>(&mut result1_recv),
        "Unique per loop iteration."
    );

    assert_eq!(
        &[0, 1, 2, 3, 4],
        &*collect_ready::<Vec<_>, _>(&mut result2_recv),
        "Unique across loop iterations, per loop execution."
    );
}
