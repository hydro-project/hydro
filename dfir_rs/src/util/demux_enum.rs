//! Trait for the `demux_enum` derive and operator.

use std::task::{Context, Poll};

pub use dfir_macro::DemuxEnum;

/// Trait for use with the `demux_enum` operator.
///
/// This trait is meant to be derived: `#[derive(DemuxEnum)]`.
///
/// The derive will implement this such that `Outputs` can be any tuple where each item is a
/// `Sink` that corresponds to each of the variants of the tuple, in alphabetic order.
#[diagnostic::on_unimplemented(
    note = "ensure there is exactly one output for each enum variant.",
    note = "ensure that the type for each output is a tuple of the field for the variant: `()`, `(a,)`, or `(a, b, ...)`."
)]
pub trait DemuxEnumSink<Outputs>: DemuxEnumBase {
    /// The error type for pushing self into the `Outputs` `Sink`s.
    type Error;

    /// Call `poll_ready` on all `Outputs`.
    fn poll_ready(outputs: &mut Outputs, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>>;

    /// Pushes `self` into the corresponding output sink in `outputs`.
    fn start_send(self, outputs: &mut Outputs) -> Result<(), Self::Error>;

    /// Call `poll_flush` on all `Outputs`.
    fn poll_flush(outputs: &mut Outputs, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>>;

    /// Call `poll_close` on all `Outputs`.
    fn poll_close(outputs: &mut Outputs, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>>;
}

/// Special case of [`DemuxEnum`] for when there is only one variant.
#[diagnostic::on_unimplemented(
    note = "requires that the enum have only one variant.",
    note = "ensure there are no missing outputs; there must be exactly one output for each enum variant."
)]
pub trait SingleVariant: DemuxEnumBase {
    /// Output tuple type.
    type Output;
    /// Convert self into it's single variant tuple Output.
    fn single_variant(self) -> Self::Output;
}

/// Base implementation to constrain that [`DemuxEnum<SOMETHING>`] is implemented.
#[diagnostic::on_unimplemented(note = "use `#[derive(dfir_rs::DemuxEnum)]`")]
pub trait DemuxEnumBase {}
