use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::LazyLock;

use criterion::{Criterion, criterion_group, criterion_main};
use dfir_rs::dfir_syntax;
use dfir_rs::itertools::Itertools;
use nameof::name_of;

const OUTPUT: usize = 5_123_595;

static WORDS: LazyLock<String> = LazyLock::new(|| {
    let mut path = PathBuf::new();
    path.push(std::env::current_dir().unwrap());
    path.pop();
    path.push(file!());
    path.pop();
    path.push("words_alpha.txt");
    std::fs::read_to_string(path).unwrap()
});
fn words() -> impl Iterator<Item = String> + Clone {
    WORDS
        .lines()
        .filter(|&s| !hash_code(s).is_multiple_of(2))
        .map(|s| s.to_owned())
}
fn hash_code(s: &str) -> u32 {
    s.bytes()
        .fold(0, |n, c| n.wrapping_mul(31).wrapping_add(c as u32))
}

fn dfir_rs_diamond(c: &mut Criterion) {
    let _ = *WORDS;

    c.bench_function(name_of!(dfir_rs_diamond), |b| {
        b.iter(|| {
            let words = words();
            let mut df = dfir_syntax! {
                my_tee = source_iter(words) -> tee();
                my_tee -> flat_map(|s| [format!("hi {}", s), format!("bye {}", s)]) -> my_union;
                my_tee -> filter(|s| 0 == s.len() % 5) -> my_union;
                my_union = union() -> fold(|| 0, |n, s| {
                    *n += s.len();
                }) -> assert_eq([OUTPUT]);
            };
            df.run_available_sync();
        })
    });
}

fn hydroflo2_diamond_forloop(c: &mut Criterion) {
    let _ = *WORDS;

    c.bench_function(name_of!(hydroflo2_diamond_forloop), |b| {
        b.iter(|| {
            let mut c = 0;
            for s in words() {
                let a = { [format!("hi {}", s), format!("bye {}", s)] };
                let b = { if 0 == s.len() % 5 { Some(s) } else { None } };
                for s in a.into_iter().chain(b) {
                    c += s.len();
                }
            }
            assert_eq!(OUTPUT, c);
        })
    });
}
fn hydroflo2_diamond_iter_clone_chain(c: &mut Criterion) {
    let _ = *WORDS;

    c.bench_function(name_of!(hydroflo2_diamond_iter_clone_chain), |b| {
        b.iter(|| {
            let i = words();
            let a = i
                .clone()
                .flat_map(|s| [format!("hi {}", s), format!("bye {}", s)]);
            let b = i.filter(|s| 0 == s.len() % 5);
            let n = a.chain(b).fold(0, |n, s| n + s.len());
            assert_eq!(OUTPUT, n);
        })
    });
}
fn hydroflo2_diamond_iter_clone_interleave(c: &mut Criterion) {
    let _ = *WORDS;

    c.bench_function(name_of!(hydroflo2_diamond_iter_clone_interleave), |b| {
        b.iter(|| {
            let i = words();
            let a = i
                .clone()
                .flat_map(|s| [format!("hi {}", s), format!("bye {}", s)]);
            let b = i.filter(|s| 0 == s.len() % 5);
            let n = a.interleave(b).fold(0, |n, s| n + s.len());
            assert_eq!(OUTPUT, n);
        })
    });
}
fn hydroflo2_diamond_iter_buffer_chain(c: &mut Criterion) {
    let _ = *WORDS;

    c.bench_function(name_of!(hydroflo2_diamond_iter_buffer_chain), |b| {
        b.iter(|| {
            let i = words();
            let v = i.collect::<Vec<_>>();
            let a = v
                .iter()
                .cloned()
                .flat_map(|s| [format!("hi {}", s), format!("bye {}", s)]);
            #[expect(clippy::iter_overeager_cloned, reason = "benchmark consistency")]
            let b = v.iter().cloned().filter(|s| 0 == s.len() % 5);
            let n = a.chain(b).fold(0, |n, s| n + s.len());
            assert_eq!(OUTPUT, n);
        })
    });
}
fn hydroflo2_diamond_iter_tee_chain(c: &mut Criterion) {
    let _ = *WORDS;

    c.bench_function(name_of!(hydroflo2_diamond_iter_tee_chain), |b| {
        b.iter(|| {
            let i = words();
            let (a, b) = i.tee();
            let a = a.flat_map(|s| [format!("hi {}", s), format!("bye {}", s)]);
            let b = b.filter(|s| 0 == s.len() % 5);
            let n = a.chain(b).fold(0, |n, s| n + s.len());
            assert_eq!(OUTPUT, n);
        })
    });
}

fn hydroflo2_diamond_iter_tee_interleave(c: &mut Criterion) {
    let _ = *WORDS;

    c.bench_function(name_of!(hydroflo2_diamond_iter_tee_interleave), |b| {
        b.iter(|| {
            let i = words();
            let (a, b) = i.tee();
            let a = a.flat_map(|s| [format!("hi {}", s), format!("bye {}", s)]);
            let b = b.filter(|s| 0 == s.len() % 5);
            let c = a.interleave(b);
            let n = c.fold(0, |n, s| n + s.len());
            assert_eq!(OUTPUT, n);
        })
    });
}

fn hydroflo2_diamond_iter_buffer_one(c: &mut Criterion) {
    let _ = *WORDS;

    c.bench_function(name_of!(hydroflo2_diamond_iter_buffer_one), |b| {
        b.iter(|| {
            let i = words();
            let buffer = RefCell::new(Vec::new());
            let a = i
                .inspect(|s| buffer.borrow_mut().push(s.clone()))
                .flat_map(|s| [format!("hi {}", s), format!("bye {}", s)]);
            let c = a.chain_with(|| {
                std::mem::take(&mut *buffer.borrow_mut())
                    .into_iter()
                    .filter(|s| 0 == s.len() % 5)
            });
            let n = c.fold(0, |n, s| n + s.len());
            assert_eq!(OUTPUT, n);
        })
    });
}

criterion_group!(
    words_diamond,
    dfir_rs_diamond,
    hydroflo2_diamond_forloop,
    hydroflo2_diamond_iter_clone_chain,
    hydroflo2_diamond_iter_clone_interleave,
    hydroflo2_diamond_iter_buffer_chain,
    hydroflo2_diamond_iter_tee_chain,
    hydroflo2_diamond_iter_tee_interleave,
    hydroflo2_diamond_iter_buffer_one,
);
criterion_main!(words_diamond);

trait IteratorExt: Iterator {
    fn chain_with<Factory, IntoIter>(
        self,
        f: Factory,
    ) -> ChainWith<Self, IntoIter::IntoIter, Factory>
    where
        Self: Sized,
        Factory: FnOnce() -> IntoIter,
        IntoIter: IntoIterator<Item = Self::Item>,
    {
        ChainWith::IterA {
            iter_a: self,
            factory: Some(f),
        }
    }
}

impl<I: Iterator> IteratorExt for I {}

pub enum ChainWith<IterA, IterB, Factory> {
    IterA {
        iter_a: IterA,
        factory: Option<Factory>,
    },
    IterB {
        iter_b: IterB,
    },
}

impl<IterA, IntoIterB, Factory> Iterator for ChainWith<IterA, IntoIterB::IntoIter, Factory>
where
    IterA: Iterator,
    IntoIterB: IntoIterator<Item = IterA::Item>,
    Factory: FnOnce() -> IntoIterB,
{
    type Item = IterA::Item;
    fn next(&mut self) -> Option<Self::Item> {
        if let Self::IterA {
            iter_a: iter,
            factory,
        } = self
        {
            if let Some(item) = iter.next() {
                return Some(item);
            }

            // First iter exhausted, transition to second.
            let factory = factory.take().unwrap();
            *self = Self::IterB {
                iter_b: (factory)().into_iter(),
            };
        }

        let Self::IterB { iter_b: iter } = self else {
            unreachable!();
        };
        iter.next()
    }
}
