use std::collections::HashMap;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use timely::dataflow::channels::pact::Pipeline;
use timely::dataflow::operators::{Operator, ToStream};

trait JoinValue: Clone + std::hash::Hash + Eq {
    fn name() -> &'static str;
    fn new(i: usize) -> Self;
}

impl JoinValue for usize {
    fn name() -> &'static str {
        "usize"
    }

    fn new(i: usize) -> Self {
        i
    }
}

impl JoinValue for String {
    fn name() -> &'static str {
        "String"
    }

    fn new(i: usize) -> Self {
        format!("value{}", i)
    }
}

const NUM_INTS: usize = 100_000;

fn benchmark_timely<L, R>(c: &mut Criterion)
where
    L: 'static + JoinValue,
    R: 'static + JoinValue,
{
    c.bench_function(
        format!("join/{}/{}/timely", L::name(), R::name()).as_str(),
        |b| {
            b.iter(|| {
                timely::example(move |scope| {
                    let lhs = (0..NUM_INTS).map(|x| (x, L::new(x))).to_stream(scope);
                    let rhs = (0..NUM_INTS).map(|x| (x, R::new(x))).to_stream(scope);

                    lhs.binary(&rhs, Pipeline, Pipeline, "HashJoin", |_, _| {
                        let mut left_tab: HashMap<usize, Vec<L>> = HashMap::new();
                        let mut right_tab: HashMap<usize, Vec<R>> = HashMap::new();
                        let mut lvec: Vec<(usize, L)> = Vec::new();
                        let mut rvec: Vec<(usize, R)> = Vec::new();
                        move |left, right, output| {
                            left.for_each(|time, data| {
                                data.swap(&mut lvec);
                                let mut session = output.session(&time);

                                for (k, v) in lvec.drain(..) {
                                    if let Some(matches) = right_tab.get(&k) {
                                        for v2 in matches {
                                            session.give((k, v.clone(), v2.clone()))
                                        }
                                    }

                                    left_tab.entry(k).or_insert_with(Vec::new).push(v);
                                }
                            });

                            right.for_each(|time, data| {
                                data.swap(&mut rvec);
                                let mut session = output.session(&time);

                                for (k, v) in rvec.drain(..) {
                                    if let Some(matches) = left_tab.get(&k) {
                                        for v2 in matches {
                                            session.give((k, v2.clone(), v.clone()))
                                        }
                                    }

                                    right_tab.entry(k).or_insert_with(Vec::new).push(v);
                                }
                            });
                        }
                    });
                });
            })
        },
    );
}

fn benchmark_sol<L, R>(c: &mut Criterion)
where
    L: 'static + JoinValue,
    R: 'static + JoinValue,
{
    c.bench_function(
        format!("join/{}/{}/sol", L::name(), R::name()).as_str(),
        |b| {
            b.iter(|| {
                let iter_a = (0..NUM_INTS).map(|x| (x, L::new(x)));
                let iter_b = (0..NUM_INTS).map(|x| (x, R::new(x)));
                let mut items_a = HashMap::new();
                let mut items_b = HashMap::new();

                for (key, val_a) in iter_a {
                    if let Some(vals_b) = items_b.get(&key) {
                        for val_b in vals_b {
                            black_box((key, val_a.clone(), val_b));
                        }
                    }
                    items_a.entry(key).or_insert_with(Vec::new).push(val_a);
                }
                for (key, val_b) in iter_b {
                    if let Some(vals_a) = items_a.get(&key) {
                        for val_a in vals_a {
                            black_box((key, val_a, val_b.clone()));
                        }
                    }
                    items_b.entry(key).or_insert_with(Vec::new).push(val_b);
                }
            });
        },
    );
}

criterion_group!(
    fan_in_dataflow,
    benchmark_timely<usize, usize>,
    benchmark_sol<usize, usize>,
    benchmark_timely<String, String>,
    benchmark_sol<String, String>,
);
criterion_main!(fan_in_dataflow);
