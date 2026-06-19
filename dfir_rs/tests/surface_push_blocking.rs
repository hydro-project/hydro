//! Tests that blocking operators work on the push side of a subgraph.
//! These operators end up push-side when a tee (multi-output) is upstream in the same subgraph.

use dfir_rs::dfir_syntax;
use dfir_rs::util::collect_ready;
use multiplatform_test::multiplatform_test;

/// sort on push side: source -> tee -> sort -> for_each
#[multiplatform_test]
pub fn test_sort_push() {
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let mut df = dfir_syntax! {
        my_tee = source_iter([3, 1, 2]) -> tee();
        my_tee -> sort() -> for_each(|v| out_send.send(v).unwrap());
        my_tee -> null();
    };
    df.run_available_sync();
    assert_eq!(&[1, 2, 3], &*collect_ready::<Vec<_>, _>(&mut out_recv));
}

/// sort_by_key on push side: source -> tee -> sort_by_key -> for_each
#[multiplatform_test]
pub fn test_sort_by_key_push() {
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<(i32, char)>();
    let mut df = dfir_syntax! {
        my_tee = source_iter([(2, 'y'), (3, 'x'), (1, 'z')]) -> tee();
        my_tee -> sort_by_key(|(k, _v)| k) -> for_each(|v| out_send.send(v).unwrap());
        my_tee -> null();
    };
    df.run_available_sync();
    assert_eq!(
        &[(1, 'z'), (2, 'y'), (3, 'x')],
        &*collect_ready::<Vec<_>, _>(&mut out_recv)
    );
}

/// reduce on push side: source -> tee -> reduce -> for_each
#[multiplatform_test]
pub fn test_reduce_push() {
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let mut df = dfir_syntax! {
        my_tee = source_iter([1, 2, 3]) -> tee();
        my_tee -> reduce(|a: &mut _, b| *a += b) -> for_each(|v| out_send.send(v).unwrap());
        my_tee -> null();
    };
    df.run_available_sync();
    assert_eq!(&[6], &*collect_ready::<Vec<_>, _>(&mut out_recv));
}

/// fold_keyed on push side
#[multiplatform_test]
pub fn test_fold_keyed_push() {
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<(i32, i32)>();
    let mut df = dfir_syntax! {
        my_tee = source_iter([(1, 10), (1, 20), (2, 30)]) -> tee();
        my_tee -> fold_keyed(|| 0, |a: &mut _, b| *a += b) -> for_each(|v| out_send.send(v).unwrap());
        my_tee -> null();
    };
    df.run_available_sync();
    let mut out = collect_ready::<Vec<_>, _>(&mut out_recv);
    out.sort();
    assert_eq!(&[(1, 30), (2, 30)], &*out);
}

/// reduce_keyed on push side
#[multiplatform_test]
pub fn test_reduce_keyed_push() {
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<(i32, i32)>();
    let mut df = dfir_syntax! {
        my_tee = source_iter([(1, 10), (1, 20), (2, 30)]) -> tee();
        my_tee -> reduce_keyed(|a: &mut _, b| *a += b) -> for_each(|v| out_send.send(v).unwrap());
        my_tee -> null();
    };
    df.run_available_sync();
    let mut out = collect_ready::<Vec<_>, _>(&mut out_recv);
    out.sort();
    assert_eq!(&[(1, 30), (2, 30)], &*out);
}

/// fold on push side: source -> tee -> fold -> for_each
#[multiplatform_test]
pub fn test_fold_push() {
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let mut df = dfir_syntax! {
        my_tee = source_iter([1, 2, 3]) -> tee();
        my_tee -> fold(|| 0, |a: &mut _, b| *a += b) -> for_each(|v| out_send.send(v).unwrap());
        my_tee -> null();
    };
    df.run_available_sync();
    assert_eq!(&[6], &*collect_ready::<Vec<_>, _>(&mut out_recv));
}

/// fold_no_replay on push side: source -> tee -> fold_no_replay -> for_each
#[multiplatform_test]
pub fn test_fold_no_replay_push() {
    let (items_send, items_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let mut df = dfir_syntax! {
        my_tee = source_stream(items_recv) -> tee();
        my_tee -> fold_no_replay::<'static>(|| 0, |a: &mut _, b| *a += b) -> for_each(|v| out_send.send(v).unwrap());
        my_tee -> null();
    };

    items_send.send(1).unwrap();
    items_send.send(2).unwrap();
    df.run_tick_sync();
    assert_eq!(&[3], &*collect_ready::<Vec<_>, _>(&mut out_recv));

    // No new input: fold_no_replay should NOT emit.
    df.run_tick_sync();
    assert_eq!(&[] as &[i32], &*collect_ready::<Vec<_>, _>(&mut out_recv));

    // New input arrives: should emit updated accumulator.
    items_send.send(10).unwrap();
    df.run_tick_sync();
    assert_eq!(&[13], &*collect_ready::<Vec<_>, _>(&mut out_recv));
}

/// reduce_no_replay on push side: source -> tee -> reduce_no_replay -> for_each
#[multiplatform_test]
pub fn test_reduce_no_replay_push() {
    let (items_send, items_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let mut df = dfir_syntax! {
        my_tee = source_stream(items_recv) -> tee();
        my_tee -> reduce_no_replay::<'static>(|a: &mut _, b| *a += b) -> for_each(|v| out_send.send(v).unwrap());
        my_tee -> null();
    };

    items_send.send(1).unwrap();
    items_send.send(2).unwrap();
    df.run_tick_sync();
    assert_eq!(&[3], &*collect_ready::<Vec<_>, _>(&mut out_recv));

    // No new input: reduce_no_replay should NOT emit.
    df.run_tick_sync();
    assert_eq!(&[] as &[i32], &*collect_ready::<Vec<_>, _>(&mut out_recv));

    // New input arrives: should emit updated accumulator.
    items_send.send(10).unwrap();
    df.run_tick_sync();
    assert_eq!(&[13], &*collect_ready::<Vec<_>, _>(&mut out_recv));
}

/// persist_mut on push side: source -> tee -> persist_mut -> for_each
#[multiplatform_test]
pub fn test_persist_mut_push() {
    use dfir_rs::util::Persistence::*;

    let (items_send, items_recv) =
        dfir_rs::util::unbounded_channel::<dfir_rs::util::Persistence<usize>>();
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<usize>();
    let mut df = dfir_syntax! {
        my_tee = source_stream(items_recv) -> tee();
        my_tee -> persist_mut::<'mutable>() -> for_each(|v| out_send.send(v).unwrap());
        my_tee -> null();
    };

    items_send.send(Persist(1)).unwrap();
    items_send.send(Persist(2)).unwrap();
    items_send.send(Persist(3)).unwrap();
    items_send.send(Delete(2)).unwrap();
    df.run_tick_sync();
    let mut out = collect_ready::<Vec<_>, _>(&mut out_recv);
    out.sort();
    assert_eq!(&[1, 3], &*out);

    // Second tick: persist_mut should replay persisted state.
    df.run_tick_sync();
    let mut out = collect_ready::<Vec<_>, _>(&mut out_recv);
    out.sort();
    assert_eq!(&[1, 3], &*out);
}

/// persist_mut_keyed on push side: source -> tee -> persist_mut_keyed -> for_each
#[multiplatform_test]
pub fn test_persist_mut_keyed_push() {
    use dfir_rs::util::PersistenceKeyed::*;

    let (items_send, items_recv) =
        dfir_rs::util::unbounded_channel::<dfir_rs::util::PersistenceKeyed<i32, i32>>();
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<(i32, i32)>();
    let mut df = dfir_syntax! {
        my_tee = source_stream(items_recv) -> tee();
        my_tee -> persist_mut_keyed::<'mutable>() -> for_each(|v| out_send.send(v).unwrap());
        my_tee -> null();
    };

    items_send.send(Persist(1, 10)).unwrap();
    items_send.send(Persist(2, 20)).unwrap();
    items_send.send(Persist(3, 30)).unwrap();
    items_send.send(Delete(2)).unwrap();
    df.run_tick_sync();
    let mut out = collect_ready::<Vec<_>, _>(&mut out_recv);
    out.sort();
    assert_eq!(&[(1, 10), (3, 30)], &*out);

    // Second tick: persist_mut_keyed should replay persisted state.
    df.run_tick_sync();
    let mut out = collect_ready::<Vec<_>, _>(&mut out_recv);
    out.sort();
    assert_eq!(&[(1, 10), (3, 30)], &*out);
}
