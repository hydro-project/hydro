#![allow(unexpected_cfgs)]

use hydro_lang::location::MemberId;
use hydro_lang::prelude::*;

struct P1 {}
struct Workers {}

// Deliberately does not implement `Serialize`.
struct OpaquePayload {
    text: String,
}

// A stub `Deserialize` impl, so that only the missing `Serialize` produces an
// error: rustc's "the following other types implement ..." candidate list for
// `Deserialize` includes concrete `&'a X` impls contributed by whatever
// happens to be in the dependency graph (e.g. `&'a camino::Utf8Path` when
// camino's serde feature is enabled), which makes its stderr snapshot vary
// across environments. `Serialize`'s list is stable blanket/tuple impls.
impl<'de> serde::Deserialize<'de> for OpaquePayload {
    fn deserialize<D: serde::Deserializer<'de>>(_: D) -> Result<Self, D::Error> {
        unreachable!()
    }
}

fn test<'a>(p1: &Process<'a, P1>, workers: &Cluster<'a, Workers>) {
    let payloads = p1
        .source_iter(q!(vec!["hello".to_owned()]))
        .map(q!(|s| (MemberId::from_raw_id(0), OpaquePayload { text: s })));

    // `.bincode()` serializes items, so the payload must implement
    // `Serialize` (and `DeserializeOwned`); `.embedded()` would accept it.
    payloads.demux(workers, TCP.fail_stop().bincode());
}

fn main() {}
