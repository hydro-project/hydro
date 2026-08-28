use hydro_lang::location::MemberId;
use hydro_lang::prelude::*;

pub struct Src {}
pub struct Dst {}

/// A payload that deliberately does **not** implement `Serialize`/`DeserializeOwned`.
///
/// `.embedded()` channels move values in-process without ever serializing them, so demuxing
/// this type must compile without serde impls (regression test for hydro-project/hydro#3158).
pub struct OpaquePayload {
    pub text: String,
}

/// Like [`super::o2m_broadcast::o2m_broadcast`], but demuxes to explicit members with
/// `.embedded()` serialization and a payload type that has no serde derives.
pub fn o2m_demux_embedded<'a>(
    cluster: &Cluster<'a, Dst>,
    input: Stream<String, Process<'a, Src>>,
) -> Stream<String, Cluster<'a, Dst>> {
    input
        .map(q!(|s| (
            MemberId::from_raw_id(0),
            OpaquePayload { text: s }
        )))
        .demux(cluster, TCP.fail_stop().embedded().name("demux_data"))
        .map(q!(|payload| payload.text.to_uppercase()))
}
