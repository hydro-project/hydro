use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use criterion::{Criterion, criterion_group, criterion_main};
use dfir_rs::dfir_syntax;
use dfir_rs::scheduled::graph::Dfir;

const NUM_ELEMS: u32 = 3000;

/// A future which returns () after it manually woken
pub struct ManualFut {
    done: Rc<RefCell<bool>>,
    waker: Rc<RefCell<Option<Waker>>>,
}
impl ManualFut {
    pub fn new(done: Rc<RefCell<bool>>, waker: Rc<RefCell<Option<Waker>>>) -> ManualFut {
        ManualFut { done, waker }
    }
}

impl Future for ManualFut {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match *self.done.borrow() {
            true => Poll::Ready(()),
            false => {
                self.waker.replace(Some(cx.waker().clone()));
                Poll::Pending
            }
        }
    }
}

fn benchmark_immediately_available(c: &mut Criterion) {
    c.bench_function("futures/immediately_available", |b| {
        b.iter_batched(
            || {
                let mut df = dfir_syntax! {
                    source_iter(0..NUM_ELEMS)
                    -> map(|x| async move {
                        x
                    })
                    -> defer_tick()
                    -> resolve_futures()
                    -> for_each(|_| {});
                };

                df.run_tick_sync(); // skip loading and mapping to future
                df
            },
            |mut df| {
                df.run_tick_sync();
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

type WakeInfo = (Rc<RefCell<bool>>, Vec<Rc<RefCell<Option<Waker>>>>);

fn benchmark_delayed(c: &mut Criterion) {
    fn setup<'a>(count: u32, wake_one: bool) -> (Dfir<'a>, WakeInfo) {
        let done = Rc::new(RefCell::new(false));
        let mut wakers = Vec::new();

        let range = 0..count;
        let futs = range
            .map(|i| {
                let waker = Rc::new(RefCell::new(None));
                let d = if !wake_one || i == 0 {
                    wakers.push(waker.clone());
                    done.clone()
                } else {
                    Rc::new(RefCell::new(false))
                };
                ManualFut::new(d, waker)
            })
            .collect::<Vec<_>>();

        let df = {
            dfir_syntax! {
                source_iter(futs)
                -> resolve_futures()
                -> for_each(|_| {});
            }
        };

        (df, (done, wakers))
    }

    fn wake_all((done, wakers): WakeInfo) {
        *done.borrow_mut() = true;
        wakers.into_iter().for_each(|waker| {
            if let Some(waker) = waker.borrow_mut().take() {
                waker.wake();
            } else {
                panic!("waker not found but future should have been polled")
            }
        })
    }

    // Tick with the initial poll
    c.bench_function("futures/delayed/initial", |b| {
        b.iter_batched(
            || setup(NUM_ELEMS, false).0,
            |mut df| {
                df.run_tick_sync();
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // Tick when no results are available
    c.bench_function("futures/delayed/waiting", |b| {
        b.iter_batched(
            || {
                let (mut df, wakes) = setup(NUM_ELEMS, true);
                df.run_tick_sync();
                df.run_tick_sync();
                df.run_tick_sync();
                wake_all(wakes);
                df
            },
            |mut df| {
                df.run_tick_sync();
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // Tick when results became available
    c.bench_function("futures/delayed/ready", |b| {
        b.iter_batched(
            || {
                let (mut df, wakes) = setup(NUM_ELEMS, false);
                df.run_tick_sync();
                wake_all(wakes);
                df
            },
            |mut df| {
                df.run_tick_sync();
            },
            criterion::BatchSize::SmallInput,
        );
    });
    // Tick after all results have been consumed
    c.bench_function("futures/delayed/done", |b| {
        b.iter_batched(
            || {
                let (mut df, wakes) = setup(NUM_ELEMS, false);
                df.run_tick_sync();
                wake_all(wakes);
                df.run_tick_sync();
                df
            },
            |mut df| {
                df.run_tick_sync();
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

criterion_group!(
    name=futures;
    config=Criterion::default().measurement_time(Duration::from_secs(30));
    targets=benchmark_immediately_available,
    benchmark_delayed
);
criterion_main!(futures);
