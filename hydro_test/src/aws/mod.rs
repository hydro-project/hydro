use aws_config::SdkConfig;
use hydro_lang::location::tick::NoAtomic;
use hydro_lang::location::{Location, NoTick};
use hydro_lang::prelude::*;

#[cfg(feature = "aws_sqs")]
pub mod sqs;

/// At-least-once delivery, message ordering isn't preserved.
pub fn source_sdk_config<'a, Loc>(location: Loc) -> Singleton<SdkConfig, Loc, Bounded>
where
    Loc: Location<'a> + NoTick + NoAtomic,
{
    location
        .singleton(q!(aws_config::load_defaults(
            aws_config::BehaviorVersion::latest()
        )))
        .resolve_future_blocking()
}
