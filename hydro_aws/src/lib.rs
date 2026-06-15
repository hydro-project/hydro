#[cfg(stageleft_runtime)]
hydro_lang::setup!();

use hydro_lang::location::Location;
use hydro_lang::prelude::*;

#[cfg(feature = "sqs")]
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
pub fn source_sdk_config<'a, Loc>(
    location: &Loc,
) -> Singleton<aws_config::SdkConfig, Loc::DropConsistency, Bounded>
where
    Loc: Location<'a>,
{
    location
        .singleton(q!(aws_config::load_from_env()))
        .resolve_future_blocking()
}
