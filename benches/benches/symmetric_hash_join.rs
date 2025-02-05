use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use dfir_rs::compiled::pull::{symmetric_hash_join_into_iter, HalfSetJoinState};
use rand::distributions::Distribution;
use rand::rngs::StdRng;
use rand::SeedableRng;

fn ops(c: &mut Criterion) {
    let mut rng = StdRng::from_entropy();

    c.bench_function("symmetric_hash_join/no_match", |b| {
        let lhs: Vec<_> = (0..3000).map(|v| (v, ())).collect();
        let rhs: Vec<_> = (0..3000).map(|v| (v + 50000, ())).collect();

        b.iter(|| {
            let (mut lhs_state, mut rhs_state) =
                black_box((HalfSetJoinState::default(), HalfSetJoinState::default()));
            let join = symmetric_hash_join_into_iter(
                black_box(lhs.iter().cloned()),
                black_box(rhs.iter().cloned()),
                &mut lhs_state,
                &mut rhs_state,
                false,
            );

            for v in join {
                black_box(v);
            }
        });
    });

    c.bench_function("symmetric_hash_join/match_keys_diff_values", |b| {
        let lhs: Vec<_> = (0..3000).map(|v| (v, v)).collect();
        let rhs: Vec<_> = (0..3000).map(|v| (v, v + 50000)).collect();

        b.iter(|| {
            let (mut lhs_state, mut rhs_state) =
                black_box((HalfSetJoinState::default(), HalfSetJoinState::default()));
            let join = symmetric_hash_join_into_iter(
                black_box(lhs.iter().cloned()),
                black_box(rhs.iter().cloned()),
                &mut lhs_state,
                &mut rhs_state,
                false,
            );

            for v in join {
                black_box(v);
            }
        });
    });

    c.bench_function("symmetric_hash_join/match_keys_same_values", |b| {
        let lhs: Vec<_> = (0..3000).map(|v| (v, v)).collect();
        let rhs: Vec<_> = (0..3000).map(|v| (v, v)).collect();

        b.iter(|| {
            let (mut lhs_state, mut rhs_state) =
                black_box((HalfSetJoinState::default(), HalfSetJoinState::default()));
            let join = symmetric_hash_join_into_iter(
                black_box(lhs.iter().cloned()),
                black_box(rhs.iter().cloned()),
                &mut lhs_state,
                &mut rhs_state,
                false,
            );
            for v in join {
                black_box(v);
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
                let join = symmetric_hash_join_into_iter(
                    black_box(lhs.iter().cloned()),
                    black_box(rhs.iter().cloned()),
                    &mut lhs_state,
                    &mut rhs_state,
                    false,
                );

                for v in join {
                    black_box(v);
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
                let join = symmetric_hash_join_into_iter(
                    black_box(lhs.iter().cloned()),
                    black_box(rhs.iter().cloned()),
                    &mut lhs_state,
                    &mut rhs_state,
                    false,
                );

                for v in join {
                    black_box(v);
                }
            });
        },
    );
}

criterion_group!(symmetric_hash_join, ops,);
criterion_main!(symmetric_hash_join);
