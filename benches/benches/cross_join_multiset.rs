use std::future::Future;
use std::hint::black_box;
use std::iter::{Fuse, Peekable};
use std::pin::pin;
use std::task::{Context, Poll, Waker};

use criterion::{Criterion, criterion_group, criterion_main};
use dfir_rs::compiled::pull::{HalfMultisetJoinState, symmetric_hash_join_into_stream};
use dfir_rs::dfir_syntax;
use futures::stream::Stream;

fn run_shj_cross_benchmark<V1, V2>(
    lhs: impl IntoIterator<Item = V1>,
    rhs: impl IntoIterator<Item = V2>,
) where
    V1: Clone,
    V2: Clone,
{
    use futures::stream::StreamExt;

    let (mut lhs_state, mut rhs_state) = (
        HalfMultisetJoinState::default(),
        HalfMultisetJoinState::default(),
    );
    let join = symmetric_hash_join_into_stream(
        futures::stream::iter(lhs).map(|x| ((), x)),
        futures::stream::iter(rhs).map(|x| ((), x)),
        &mut lhs_state,
        &mut rhs_state,
        false,
    );
    let join = pin!(join);
    let Poll::Ready(join) = Future::poll(join, &mut Context::from_waker(Waker::noop())) else {
        panic!()
    };

    let mut join = pin!(join);
    while let Poll::Ready(Some(item)) =
        Stream::poll_next(join.as_mut(), &mut Context::from_waker(Waker::noop()))
    {
        black_box(item);
    }
}

fn nested_loops<V1, V2>(lhs: impl IntoIterator<Item = V1>, rhs: impl IntoIterator<Item = V2>)
where
    V1: Clone,
    V2: Clone,
{
    let rhs = rhs.into_iter().collect::<Vec<_>>();
    for v1 in lhs {
        for v2 in rhs.iter() {
            black_box((v1.clone(), v2.clone()));
        }
    }
}

struct CrossIter<I1, I2>
where
    I1: Iterator,
    I2: Iterator,
{
    lhs: Peekable<I1>,
    rhs: Fuse<I2>,
    rhs_items: Vec<I2::Item>,
    rhs_idx: usize,
}

impl<I1, I2> CrossIter<I1, I2>
where
    I1: Iterator,
    I2: Iterator,
    I1::Item: Clone,
    I2::Item: Clone,
{
    pub fn new(lhs: I1, rhs: I2) -> Self {
        Self {
            lhs: lhs.peekable(),
            rhs: rhs.fuse(),
            rhs_items: Vec::new(),
            rhs_idx: 0,
        }
    }
}

impl<I1, I2> Iterator for CrossIter<I1, I2>
where
    I1: Iterator,
    I2: Iterator,
    I1::Item: Clone,
    I2::Item: Clone,
{
    type Item = (I1::Item, I2::Item);

    fn next(&mut self) -> Option<Self::Item> {
        for v2 in self.rhs.by_ref() {
            self.rhs_items.push(v2);
        }
        while let Some(lhs) = self.lhs.peek() {
            if let Some(rhs) = self.rhs_items.get(self.rhs_idx) {
                self.rhs_idx += 1;

                return Some((lhs.clone(), rhs.clone()));
            }
            let _ = self.lhs.next();
            self.rhs_idx = 0;
        }
        None
    }
}

#[test]
fn test_cross_iter() {
    let lhs = [1, 2, 3];
    let rhs = ['a', 'b'];

    let mut expected = Vec::new();
    for &l in &lhs {
        for &r in &rhs {
            expected.push((l, r));
        }
    }

    let actual: Vec<_> = CrossIter::new(lhs, rhs).collect();
    assert_eq!(expected, actual);
}

fn nested_loops_cross_iter<V1, V2>(
    lhs: impl IntoIterator<Item = V1>,
    rhs: impl IntoIterator<Item = V2>,
) where
    V1: Clone,
    V2: Clone,
{
    let iter = CrossIter::new(lhs.into_iter(), rhs.into_iter());
    for (v1, v2) in iter {
        black_box((v1, v2));
    }
}

fn nested_loops_fn_iter<V1, V2>(
    lhs: impl IntoIterator<Item = V1>,
    rhs: impl IntoIterator<Item = V2>,
) where
    V1: Clone,
    V2: Clone,
{
    let rhs = rhs.into_iter().collect::<Vec<_>>();
    for (v1, v2) in lhs
        .into_iter()
        .flat_map(|v1| rhs.iter().map(move |v2| (v1.clone(), v2.clone())))
    {
        black_box((v1, v2));
    }
}

fn dfir<V1, V2>(lhs: impl IntoIterator<Item = V1>, rhs: impl IntoIterator<Item = V2>)
where
    V1: 'static + Clone,
    V2: 'static + Clone,
{
    let mut dfir = dfir_syntax! {
        source_iter(lhs) -> [0]cj;
        source_iter(rhs) -> [1]cj;
        cj = cross_join_multiset() -> for_each(|x| { black_box(x); });
    };
    dfir.run_available_sync();
}

fn ops(c: &mut Criterion) {
    for (size1, size2) in [(100, 100), (3000, 3000), (30, 30_000), (30_000, 30)] {
        let mut group = c.benchmark_group(format!("cross_join_multiset/{size1}/{size2}"));

        let iter = |n| {
            let mut x: u32 = 123;
            std::iter::from_fn(move || {
                x = x.wrapping_mul(16843009).wrapping_add(3014898611);
                let l = ((x & 0x00ff0000) >> 16) ^ (x & 0x000000ff);
                let u = ((x & 0xff000000) >> 16) ^ (x & 0x0000ff00);
                Some((l << 8) | (u >> 8))
            })
            .take(n)
        };

        group.bench_function("shj", |b| {
            b.iter(|| run_shj_cross_benchmark(iter(size1), iter(size2)))
        });

        group.bench_function("nested_loops", |b| {
            b.iter(|| nested_loops(iter(size1), iter(size2)))
        });

        group.bench_function("nested_loops_cross_iter", |b| {
            b.iter(|| nested_loops_cross_iter(iter(size1), iter(size2)))
        });

        group.bench_function("nested_loops_fn_iter", |b| {
            b.iter(|| nested_loops_fn_iter(iter(size1), iter(size2)))
        });

        group.bench_function("dfir", |b| b.iter(|| dfir(iter(size1), iter(size2))));
    }
}

criterion_group!(cross_join_multiset, ops,);
criterion_main!(cross_join_multiset);
