use quote::quote_spanned;
use syn::parse_quote_spanned;

use super::{
    FloType, OperatorCategory, OperatorConstraints, OperatorWriteOutput, WriteContextArgs, RANGE_0,
    RANGE_1,
};

/// > 0 input streams, 1 output stream
///
/// > Arguments: A [`Duration`](https://doc.rust-lang.org/stable/std/time/struct.Duration.html) for this interval.
///
/// Emits units `()` on a repeated interval. The first tick completes immediately. Missed ticks will
/// be scheduled as soon as possible.
///
/// Note that this requires the dfir instance be run within a [Tokio `Runtime`](https://docs.rs/tokio/1/tokio/runtime/struct.Runtime.html).
/// The easiest way to do this is with a [`#[dfir_rs::main]`](https://hydro.run/rustdoc/dfir_rs/attr.main)
/// annotation on `async fn main() { ... }` as in the example below.
///
/// ```rustbook
/// use std::time::Duration;
/// use std::time::Instant;
///use dfir_rs::dfir_syntax;
///
/// #[dfir_rs::main]
/// async fn main() {
///     let mut hf = dfir_syntax! {
///         source_interval(Duration::from_secs(1))
///             -> map(|_| { Instant::now() } )
///             -> for_each(|time| println!("This runs every second: {:?}", time));
///     };
///
///     // Will print 4 times (fencepost counting).
///     tokio::time::timeout(Duration::from_secs_f32(3.5), hf.run())
///         .await
///         .expect_err("Expected time out");
///
///     // Example output:
///     // This runs every second: Instant { t: 27471.704813s }
///     // This runs every second: Instant { t: 27472.704813s }
///     // This runs every second: Instant { t: 27473.704813s }
///     // This runs every second: Instant { t: 27474.704813s }
/// }
/// ```
pub const SOURCE_INTERVAL: OperatorConstraints = OperatorConstraints {
    name: "source_interval",
    categories: &[OperatorCategory::Source],
    hard_range_inn: RANGE_0,
    soft_range_inn: RANGE_0,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 1,
    persistence_args: RANGE_0,
    type_args: RANGE_0,
    is_external_input: true,
    has_singleton_output: false,
    flo_type: Some(FloType::Source),
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   op_span,
                   arguments,
                   ..
               },
               diagnostics| {
        let ident_intervalstream = wc.make_ident("intervalstream");
        let mut write_prologue = quote_spanned! {op_span=>
            let #ident_intervalstream =
                #root::tokio_stream::StreamExt::map(
                    #root::tokio_stream::wrappers::IntervalStream::new(#root::tokio::time::interval(#arguments)),
                    |_| {  }
                );
        };
        let wc = WriteContextArgs {
            arguments: &parse_quote_spanned!(op_span=> #ident_intervalstream),
            ..wc.clone()
        };
        let write_output = (super::source_stream::SOURCE_STREAM.write_fn)(&wc, diagnostics)?;
        write_prologue.extend(write_output.write_prologue);
        Ok(OperatorWriteOutput {
            write_prologue,
            ..write_output
        })
    },
};
