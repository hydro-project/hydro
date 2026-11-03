//! Top-level `for_each` closures that do not borrow the surrounding stack are built once per run.
//!
//! A closure that opens a file (the state lives inside the closure, not in a captured local)
//! must not run that setup again on every tick. Closures that borrow caller locals stay inside
//! the tick so `Dfir::into_erased` can still require a `'static` tick closure. Closures inside
//! a `loop` are rebuilt each time the loop fires.

use std::sync::atomic::{AtomicUsize, Ordering};

use dfir_rs::dfir_syntax;
use multiplatform_test::multiplatform_test;

static TOP_LEVEL_FOR_EACH_BUILDS: AtomicUsize = AtomicUsize::new(0);

#[multiplatform_test]
pub fn test_top_level_for_each_closure_built_once_across_ticks() {
    TOP_LEVEL_FOR_EACH_BUILDS.store(0, Ordering::Relaxed);

    let mut df = dfir_syntax! {
        source_iter([1, 2]) -> for_each({
            TOP_LEVEL_FOR_EACH_BUILDS.fetch_add(1, Ordering::Relaxed);
            |_x| {}
        });
    };

    df.run_tick_sync();
    df.run_tick_sync();

    assert_eq!(
        1,
        TOP_LEVEL_FOR_EACH_BUILDS.load(Ordering::Relaxed),
        "top-level for_each closure was built {} times across two ticks; expected once per run",
        TOP_LEVEL_FOR_EACH_BUILDS.load(Ordering::Relaxed),
    );
}

/// Borrowing a caller local must keep compiling, including through `into_erased`.
#[multiplatform_test]
pub fn test_borrowing_for_each_stays_static() {
    let (out_send, _out_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let df = dfir_syntax! {
        source_iter([1, 2, 3]) -> for_each(|x| out_send.send(x).unwrap());
    };
    let df = df.into_erased();
    drop(df);
}

/// `context` is only in scope inside a tick, so this closure must still see the tick id.
#[multiplatform_test]
pub fn test_top_level_for_each_can_use_context() {
    let ticks = std::cell::Cell::new(Vec::new());
    let ticks_ref = &ticks;

    let mut df = dfir_syntax! {
        source_iter([()]) -> for_each(|()| {
            ticks_ref.set({
                let mut seen = ticks_ref.take();
                seen.push(context.current_tick().0);
                seen
            });
        });
    };

    df.run_tick_sync();
    df.run_tick_sync();

    assert_eq!(vec![0], ticks.take());
}

/// Loop bodies are not Flo top-level: the closure is rebuilt each time the loop fires.
#[multiplatform_test]
pub fn test_loop_for_each_closure_built_each_firing() {
    let (in_send, in_recv) = dfir_rs::util::unbounded_channel::<i32>();
    let constructions = std::cell::Cell::new(0);
    let constructions_ref = &constructions;

    let mut df = dfir_syntax! {
        inp = source_stream(in_recv);
        loop {
            inp -> batch() -> for_each({
                constructions_ref.set(constructions_ref.get() + 1);
                |_x| {}
            });
        };
    };

    in_send.send(1).unwrap();
    df.run_tick_sync();
    in_send.send(2).unwrap();
    df.run_tick_sync();

    assert_eq!(2, constructions.get());
}
