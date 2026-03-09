use std::future::Future;
use std::hint::black_box;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

use criterion::{Criterion, criterion_group, criterion_main};
use dfir_rs::dfir_pipes::pull::{
    self, HalfSetJoinState, Pull, PullStep, symmetric_hash_join as shj_fn,
};
use rand::SeedableRng;
use rand::distributions::Distribution;
use rand::rngs::StdRng;

/// Helper function to run a symmetric hash join benchmark and consume all results
#[inline(always)]
fn run_join_benchmark<K, V1, V2>(
    lhs: impl IntoIterator<Item = (K, V1)>,
    rhs: impl IntoIterator<Item = (K, V2)>,
) where
    K: std::hash::Hash + Eq + Clone,
    V1: Clone + Eq,
    V2: Clone + Eq,
{
    let (mut lhs_state, mut rhs_state) =
        black_box((HalfSetJoinState::default(), HalfSetJoinState::default()));
    let lhs_pull = pull::iter(lhs).fuse();
    let rhs_pull = pull::iter(rhs).fuse();
    let join = shj_fn(
        black_box(lhs_pull),
        black_box(rhs_pull),
        &mut lhs_state,
        &mut rhs_state,
        false,
    );
    let join = pin!(join);
    let Poll::Ready(join) = Future::poll(join, &mut Context::from_waker(Waker::noop())) else {
        panic!()
    };

    let mut join = pin!(join);
    loop {
        match join.as_mut().pull(&mut ()) {
            PullStep::Ready(item, _) => {
                black_box(item);
            }
            PullStep::Ended(_) => break,
            PullStep::Pending(_) => unreachable!(),
        }
    }
}

fn ops(c: &mut Criterion) {
    let mut rng = StdRng::from_entropy();

    c.bench_function("symmetric_hash_join/no_match", |b| {
        let lhs: Vec<_> = (0..3000).map(|v| (v, ())).collect();
        let rhs: Vec<_> = (0..3000).map(|v| (v + 50000, ())).collect();

        b.iter(|| run_join_benchmark(lhs.iter().cloned(), rhs.iter().cloned()));
    });

    c.bench_function("symmetric_hash_join/match_keys_diff_values", |b| {
        let lhs: Vec<_> = (0..3000).map(|v| (v, v)).collect();
        let rhs: Vec<_> = (0..3000).map(|v| (v, v + 50000)).collect();

        b.iter(|| run_join_benchmark(lhs.iter().cloned(), rhs.iter().cloned()));
    });

    c.bench_function("symmetric_hash_join/match_keys_same_values", |b| {
        let lhs: Vec<_> = (0..3000).map(|v| (v, v)).collect();
        let rhs: Vec<_> = (0..3000).map(|v| (v, v)).collect();

        b.iter(|| run_join_benchmark(lhs.iter().cloned(), rhs.iter().cloned()));
    });

    c.bench_function(
        "symmetric_hash_join/zipf_keys_low_contention_unique_values",
        |b| {
            let dist = rand_distr::Zipf::new(8000, 0.5).unwrap();

            let lhs: Vec<_> = (0..2000)
                .map(|v| (dist.sample(&mut rng) as usize, v))
                .collect();

            let rhs: Vec<_> = (0..2000)
                .map(|v| (dist.sample(&mut rng) as usize, v + 8000))
                .collect();

            b.iter(|| run_join_benchmark(lhs.iter().cloned(), rhs.iter().cloned()));
        },
    );

    c.bench_function(
        "symmetric_hash_join/zipf_keys_high_contention_unique_values",
        |b| {
            let dist = rand_distr::Zipf::new(8000, 4.0).unwrap();

            let lhs: Vec<_> = (0..1000)
                .map(|v| (dist.sample(&mut rng) as usize, v))
                .collect();

            let rhs: Vec<_> = (0..1000)
                .map(|v| (dist.sample(&mut rng) as usize, v + 8000))
                .collect();

            b.iter(|| run_join_benchmark(lhs.iter().cloned(), rhs.iter().cloned()));
        },
    );
}

criterion_group!(symmetric_hash_join, ops,);
criterion_main!(symmetric_hash_join);
