use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use hydroflow::compiled::pull::{SetJoinState, SymmetricHashJoin};
use rand::distributions::Distribution;
use rand::rngs::StdRng;
use rand::SeedableRng;

fn ops(c: &mut Criterion) {
    let mut rng = StdRng::from_entropy();

    c.bench_function("symmetric_hash_join/no_match", |b| {
        let lhs: Vec<_> = (0..3000).map(|v| (v, ())).collect();
        let rhs: Vec<_> = (0..3000).map(|v| (v + 50000, ())).collect();

        b.iter(|| {
            let mut state = black_box(SetJoinState::default());
            let join = SymmetricHashJoin::new(
                black_box(lhs.iter().cloned()),
                black_box(rhs.iter().cloned()),
                &mut state,
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
            let mut state = black_box(SetJoinState::default());
            let join = SymmetricHashJoin::new(
                black_box(lhs.iter().cloned()),
                black_box(rhs.iter().cloned()),
                &mut state,
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
            let mut state = black_box(SetJoinState::default());
            let join = SymmetricHashJoin::new(
                black_box(lhs.iter().cloned()),
                black_box(rhs.iter().cloned()),
                &mut state,
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
                let mut state = black_box(SetJoinState::default());
                let join = SymmetricHashJoin::new(
                    black_box(lhs.iter().cloned()),
                    black_box(rhs.iter().cloned()),
                    &mut state,
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
                let mut state = black_box(SetJoinState::default());
                let join = SymmetricHashJoin::new(
                    black_box(lhs.iter().cloned()),
                    black_box(rhs.iter().cloned()),
                    &mut state,
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
