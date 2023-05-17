use hydroflow::hydroflow_syntax;
use hydroflow::util::collect_ready;
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
pub fn test_repeat_iter() {
    let (out_send, mut out_recv) = hydroflow::util::unbounded_channel::<usize>();

    let mut df = hydroflow_syntax! {
        repeat_iter([1]) -> for_each(|v| out_send.send(v).unwrap());
    };
    assert_eq!((0, 0), (df.current_tick(), df.current_stratum()));
    df.run_tick();
    assert_eq!((1, 0), (df.current_tick(), df.current_stratum()));
    df.run_tick();
    assert_eq!((2, 0), (df.current_tick(), df.current_stratum()));
    df.run_tick();
    assert_eq!((3, 0), (df.current_tick(), df.current_stratum()));

    assert_eq!(&[1, 1, 1], &*collect_ready::<Vec<_>, _>(&mut out_recv));

    df.run_available(); // Should return quickly and not hang
}

#[multiplatform_test]
pub fn test_fold_tick() {
    let (out_send, mut out_recv) = hydroflow::util::unbounded_channel::<usize>();

    let mut df = hydroflow_syntax! {
        source_iter([1]) -> fold::<'tick>(0, |accum, elem| accum + elem) -> for_each(|v| out_send.send(v).unwrap());
    };
    assert_eq!((0, 0), (df.current_tick(), df.current_stratum()));
    df.run_tick();
    assert_eq!((1, 0), (df.current_tick(), df.current_stratum()));
    df.run_tick();
    assert_eq!((2, 0), (df.current_tick(), df.current_stratum()));
    df.run_tick();
    assert_eq!((3, 0), (df.current_tick(), df.current_stratum()));

    assert_eq!(&[1], &*collect_ready::<Vec<_>, _>(&mut out_recv));

    df.run_available(); // Should return quickly and not hang
}

#[multiplatform_test]
pub fn test_fold_static() {
    let (out_send, mut out_recv) = hydroflow::util::unbounded_channel::<usize>();

    let mut df = hydroflow_syntax! {
        source_iter([1]) -> fold::<'static>(0, |accum, elem| accum + elem) -> for_each(|v| out_send.send(v).unwrap());
    };
    assert_eq!((0, 0), (df.current_tick(), df.current_stratum()));
    df.run_tick();
    assert_eq!((1, 0), (df.current_tick(), df.current_stratum()));
    df.run_tick();
    assert_eq!((2, 0), (df.current_tick(), df.current_stratum()));
    df.run_tick();
    assert_eq!((3, 0), (df.current_tick(), df.current_stratum()));

    assert_eq!(&[1, 1, 1], &*collect_ready::<Vec<_>, _>(&mut out_recv));

    df.run_available(); // Should return quickly and not hang
}

#[multiplatform_test]
pub fn test_reduce_tick() {
    let (out_send, mut out_recv) = hydroflow::util::unbounded_channel::<usize>();

    let mut df = hydroflow_syntax! {
        source_iter([1]) -> reduce::<'tick>(|a, b| a + b) -> for_each(|v| out_send.send(v).unwrap());
    };
    assert_eq!((0, 0), (df.current_tick(), df.current_stratum()));
    df.run_tick();
    assert_eq!((1, 0), (df.current_tick(), df.current_stratum()));
    df.run_tick();
    assert_eq!((2, 0), (df.current_tick(), df.current_stratum()));
    df.run_tick();
    assert_eq!((3, 0), (df.current_tick(), df.current_stratum()));

    assert_eq!(&[1], &*collect_ready::<Vec<_>, _>(&mut out_recv));

    df.run_available(); // Should return quickly and not hang
}

#[multiplatform_test]
pub fn test_reduce_static() {
    let (out_send, mut out_recv) = hydroflow::util::unbounded_channel::<usize>();

    let mut df = hydroflow_syntax! {
        source_iter([1]) -> reduce::<'static>(|a, b| a + b) -> for_each(|v| out_send.send(v).unwrap());
    };
    assert_eq!((0, 0), (df.current_tick(), df.current_stratum()));
    df.run_tick();
    assert_eq!((1, 0), (df.current_tick(), df.current_stratum()));
    df.run_tick();
    assert_eq!((2, 0), (df.current_tick(), df.current_stratum()));
    df.run_tick();
    assert_eq!((3, 0), (df.current_tick(), df.current_stratum()));

    assert_eq!(&[1, 1, 1], &*collect_ready::<Vec<_>, _>(&mut out_recv));

    df.run_available(); // Should return quickly and not hang
}

#[multiplatform_test]
pub fn test_group_by_tick() {
    let (out_send, mut out_recv) = hydroflow::util::unbounded_channel::<(char, usize)>();

    let mut df = hydroflow_syntax! {
        source_iter([('a', 1), ('a', 2)]) -> group_by::<'tick>(|| 0, |acc: &mut usize, item| { *acc += item; }) -> for_each(|v| out_send.send(v).unwrap());
    };
    assert_eq!((0, 0), (df.current_tick(), df.current_stratum()));
    df.run_tick();
    assert_eq!((1, 0), (df.current_tick(), df.current_stratum()));
    df.run_tick();
    assert_eq!((2, 0), (df.current_tick(), df.current_stratum()));
    df.run_tick();
    assert_eq!((3, 0), (df.current_tick(), df.current_stratum()));

    assert_eq!(&[('a', 3)], &*collect_ready::<Vec<_>, _>(&mut out_recv));

    df.run_available(); // Should return quickly and not hang
}

#[multiplatform_test]
pub fn test_group_by_static() {
    let (out_send, mut out_recv) = hydroflow::util::unbounded_channel::<(char, usize)>();

    let mut df = hydroflow_syntax! {
        source_iter([('a', 1), ('a', 2)]) -> group_by::<'static>(|| 0, |acc: &mut usize, item| { *acc += item; }) -> for_each(|v| out_send.send(v).unwrap());
    };
    assert_eq!((0, 0), (df.current_tick(), df.current_stratum()));
    df.run_tick();
    assert_eq!((1, 0), (df.current_tick(), df.current_stratum()));
    df.run_tick();
    assert_eq!((2, 0), (df.current_tick(), df.current_stratum()));
    df.run_tick();
    assert_eq!((3, 0), (df.current_tick(), df.current_stratum()));

    assert_eq!(
        &[('a', 3), ('a', 3), ('a', 3)],
        &*collect_ready::<Vec<_>, _>(&mut out_recv)
    );

    df.run_available(); // Should return quickly and not hang
}

/// Check that the scheduler does not unnecsarily schedule internal events,
/// but will resume when external events come in.
#[multiplatform_test]
pub fn test_resume_external_event() {
    let (in_send, in_recv) = hydroflow::util::unbounded_channel::<usize>();
    let (out_send, mut out_recv) = hydroflow::util::unbounded_channel::<usize>();

    let mut df = hydroflow_syntax! {
        source_stream(in_recv) -> fold::<'static>(0, <usize as std::ops::Add<usize>>::add) -> for_each(|v| out_send.send(v).unwrap());
    };

    assert_eq!((0, 0), (df.current_tick(), df.current_stratum()));
    assert_eq!(&[] as &[usize], &*collect_ready::<Vec<_>, _>(&mut out_recv));

    df.run_tick();
    assert_eq!((1, 0), (df.current_tick(), df.current_stratum()));
    assert_eq!(&[0], &*collect_ready::<Vec<_>, _>(&mut out_recv));

    df.run_tick();
    assert_eq!((2, 0), (df.current_tick(), df.current_stratum()));
    assert_eq!(&[0], &*collect_ready::<Vec<_>, _>(&mut out_recv));

    df.run_tick();
    assert_eq!((3, 0), (df.current_tick(), df.current_stratum()));
    assert_eq!(&[0], &*collect_ready::<Vec<_>, _>(&mut out_recv));

    df.run_available();
    assert_eq!((4, 0), (df.current_tick(), df.current_stratum()));
    assert_eq!(&[0], &*collect_ready::<Vec<_>, _>(&mut out_recv));

    in_send.send(1).unwrap();
    df.run_tick();
    assert_eq!((5, 0), (df.current_tick(), df.current_stratum()));
    assert_eq!(&[1], &*collect_ready::<Vec<_>, _>(&mut out_recv));

    in_send.send(2).unwrap();
    df.run_available();
    assert_eq!((6, 0), (df.current_tick(), df.current_stratum()));
    assert_eq!(&[3], &*collect_ready::<Vec<_>, _>(&mut out_recv));

    df.run_available();
    assert_eq!((7, 0), (df.current_tick(), df.current_stratum()));
    assert_eq!(&[3], &*collect_ready::<Vec<_>, _>(&mut out_recv));
}
