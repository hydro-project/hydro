use std::hint::black_box;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use criterion::{Criterion, criterion_group, criterion_main};
use dfir_rs::compiled::pull::{HalfSetJoinState, symmetric_hash_join_into_stream};
use rand::SeedableRng;
use rand::distributions::Distribution;
use rand::rngs::StdRng;

fn ops(c: &mut Criterion) {
    let mut rng = StdRng::from_entropy();

    c.bench_function("symmetric_hash_join/no_match", |b| {
        let lhs: Vec<_> = (0..3000).map(|v| (v, ())).collect();
        let rhs: Vec<_> = (0..3000).map(|v| (v + 50000, ())).collect();

        b.iter(|| {
            let (mut lhs_state, mut rhs_state) =
                black_box((HalfSetJoinState::default(), HalfSetJoinState::default()));
            let mut join = symmetric_hash_join_into_stream(
                black_box(futures::stream::iter(lhs.iter().cloned())),
                black_box(futures::stream::iter(rhs.iter().cloned())),
                &mut lhs_state,
                &mut rhs_state,
                false,
            );

            while let Poll::Ready(Some(item)) = futures::stream::Stream::poll_next(
                Pin::new(&mut join),
                &mut Context::from_waker(Waker::noop()),
            ) {
                black_box(item);
            }
        });
    });

    c.bench_function("symmetric_hash_join/match_keys_diff_values", |b| {
        let lhs: Vec<_> = (0..3000).map(|v| (v, v)).collect();
        let rhs: Vec<_> = (0..3000).map(|v| (v, v + 50000)).collect();

        b.iter(|| {
            let (mut lhs_state, mut rhs_state) =
                black_box((HalfSetJoinState::default(), HalfSetJoinState::default()));
            let mut join = symmetric_hash_join_into_stream(
                black_box(futures::stream::iter(lhs.iter().cloned())),
                black_box(futures::stream::iter(rhs.iter().cloned())),
                &mut lhs_state,
                &mut rhs_state,
                false,
            );

            while let Poll::Ready(Some(item)) = futures::stream::Stream::poll_next(
                Pin::new(&mut join),
                &mut Context::from_waker(Waker::noop()),
            ) {
                black_box(item);
            }
        });
    });

    c.bench_function("symmetric_hash_join/match_keys_same_values", |b| {
        let lhs: Vec<_> = (0..3000).map(|v| (v, v)).collect();
        let rhs: Vec<_> = (0..3000).map(|v| (v, v)).collect();

        b.iter(|| {
            let (mut lhs_state, mut rhs_state) =
                black_box((HalfSetJoinState::default(), HalfSetJoinState::default()));
            let mut join = symmetric_hash_join_into_stream(
                black_box(futures::stream::iter(lhs.iter().cloned())),
                black_box(futures::stream::iter(rhs.iter().cloned())),
                &mut lhs_state,
                &mut rhs_state,
                false,
            );

            while let Poll::Ready(Some(item)) = futures::stream::Stream::poll_next(
                Pin::new(&mut join),
                &mut Context::from_waker(Waker::noop()),
            ) {
                black_box(item);
            }
        });
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

            b.iter(|| {
                let (mut lhs_state, mut rhs_state) =
                    black_box((HalfSetJoinState::default(), HalfSetJoinState::default()));
                let mut join = symmetric_hash_join_into_stream(
                    black_box(futures::stream::iter(lhs.iter().cloned())),
                    black_box(futures::stream::iter(rhs.iter().cloned())),
                    &mut lhs_state,
                    &mut rhs_state,
                    false,
                );

                while let Poll::Ready(Some(item)) = futures::stream::Stream::poll_next(
                    Pin::new(&mut join),
                    &mut Context::from_waker(Waker::noop()),
                ) {
                    black_box(item);
                }
            });
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

            b.iter(|| {
                let (mut lhs_state, mut rhs_state) =
                    black_box((HalfSetJoinState::default(), HalfSetJoinState::default()));
                let mut join = symmetric_hash_join_into_stream(
                    black_box(futures::stream::iter(lhs.iter().cloned())),
                    black_box(futures::stream::iter(rhs.iter().cloned())),
                    &mut lhs_state,
                    &mut rhs_state,
                    false,
                );

                while let Poll::Ready(Some(item)) = futures::stream::Stream::poll_next(
                    Pin::new(&mut join),
                    &mut Context::from_waker(Waker::noop()),
                ) {
                    black_box(item);
                }
            });
        },
    );
}

criterion_group!(symmetric_hash_join, ops,);
criterion_main!(symmetric_hash_join);
