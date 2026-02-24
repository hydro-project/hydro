//! Runtime helpers for the embedded deployment backend.
//!
//! This module is NOT gated on the `build` feature so that the staged
//! re-exports are available at runtime in the generated code.

use futures::Stream;
use stageleft::{QuotedWithContext, RuntimeData, q};

use crate::location::MembershipEvent;
use crate::location::member_id::TaglessMemberId;

#[cfg_attr(
    not(any(
        feature = "deploy_integration",
        feature = "docker_runtime",
        feature = "maelstrom_runtime"
    )),
    expect(
        unreachable_code,
        reason = "uninhabited but deploy_integration required at embedded runtime"
    )
)]
/// Returns a [`QuotedWithContext`] that references the `__cluster_self_id` runtime variable.
pub fn embedded_cluster_self_id<'a>() -> impl QuotedWithContext<'a, TaglessMemberId, ()> + Clone + 'a
{
    let self_id: RuntimeData<&TaglessMemberId> = RuntimeData::new("__cluster_self_id");
    q!(self_id.clone())
}

/// Returns a [`QuotedWithContext`] that references a `__membership_{idx}` runtime variable.
pub fn embedded_cluster_membership_stream<'a>(
    idx: usize,
) -> impl QuotedWithContext<'a, Box<dyn Stream<Item = (TaglessMemberId, MembershipEvent)> + Unpin>, ()>
{
    // TODO(shadaj): change `Deploy` trait to use `syn::Expr` to avoid leaking here
    // TODO(shadaj): this will not work if the same location reads the same membership stream multiple times
    let var_name: &'static str = Box::leak(format!("__membership_{}", idx).into_boxed_str());
    let membership: RuntimeData<
        Box<dyn Stream<Item = (TaglessMemberId, MembershipEvent)> + Unpin>,
    > = RuntimeData::new(var_name);
    q!(membership)
}
