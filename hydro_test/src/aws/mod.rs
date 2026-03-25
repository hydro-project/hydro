use aws_config;
use hydro_lang::location::tick::NoAtomic;
use hydro_lang::location::{Location, NoTick};
use hydro_lang::prelude::*;

// #[cfg(feature = "aws_sqs")]
pub mod sqs;

#[ctor::ctor]
fn init_rewrites() {
    stageleft::add_private_reexport(
        vec!["futures_util", "stream", "stream"],
        vec!["futures_util", "stream"],
    );
    stageleft::add_private_reexport(
        vec!["futures_util", "stream", "iter"],
        vec!["futures_util", "stream"],
    );
    stageleft::add_private_reexport(
        vec!["futures_util", "stream", "unfold"],
        vec!["futures_util", "stream"],
    );
    stageleft::add_private_reexport(
        vec!["aws_types", "sdk_config"], /* TODO(mingwei): Uhh .. should we just depend on `aws_types`? */
        vec!["aws_config"],
    );
}

/// Singleton of the default AWS SDK config.
pub fn source_sdk_config<'a, Loc>(location: &Loc) -> Singleton<aws_config::SdkConfig, Loc, Bounded>
where
    Loc: Location<'a> + NoTick + NoAtomic,
{
    location
        .singleton(q!(aws_config::load_defaults(
            aws_config::BehaviorVersion::latest()
        )))
        .resolve_future_blocking()
}
