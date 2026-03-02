use std::cell::RefCell;
use std::pin::Pin;

use dfir_pipes::{Context, FusedPull, Pull, Step, Toggle, Yes};
use lattices::{LatticeBimorphism, Merge};
use pin_project_lite::pin_project;

pin_project! {
    /// Pull combinator for lattice bimorphism operations.
    #[must_use = "pull do nothing unless polled"]
    pub struct LatticeBimorphismPull<'a, Func, LhsPrev, RhsPrev, LhsState, RhsState, Output> {
        #[pin]
        lhs_prev: LhsPrev,
        #[pin]
        rhs_prev: RhsPrev,

        func: Func,

        lhs_state: &'a RefCell<LhsState>,
        rhs_state: &'a RefCell<RhsState>,

        output: Option<Output>,
    }
}

impl<'a, Func, LhsPrev, RhsPrev, LhsState, RhsState, Output>
    LatticeBimorphismPull<'a, Func, LhsPrev, RhsPrev, LhsState, RhsState, Output>
where
    Func: 'a
        + LatticeBimorphism<LhsState, RhsPrev::Item, Output = Output>
        + LatticeBimorphism<LhsPrev::Item, RhsState, Output = Output>,
    LhsPrev: 'a + FusedPull,
    RhsPrev: 'a + FusedPull,
    LhsState: 'static + Clone,
    RhsState: 'static + Clone,
    Output: Merge<Output>,
{
    /// Creates a new `LatticeBimorphismPull`.
    pub fn new(
        lhs_prev: LhsPrev,
        rhs_prev: RhsPrev,
        func: Func,
        lhs_state: &'a RefCell<LhsState>,
        rhs_state: &'a RefCell<RhsState>,
    ) -> Self {
        Self {
            lhs_prev,
            rhs_prev,
            func,
            lhs_state,
            rhs_state,
            output: None,
        }
    }
}

impl<'a, Func, LhsPrev, RhsPrev, LhsState, RhsState, Output> Pull
    for LatticeBimorphismPull<'a, Func, LhsPrev, RhsPrev, LhsState, RhsState, Output>
where
    Func: 'a
        + LatticeBimorphism<LhsState, RhsPrev::Item, Output = Output>
        + LatticeBimorphism<LhsPrev::Item, RhsState, Output = Output>,
    LhsPrev: 'a + FusedPull,
    RhsPrev: 'a + FusedPull,
    LhsState: 'static + Clone,
    RhsState: 'static + Clone,
    Output: Merge<Output>,
{
    type Ctx<'ctx> = <LhsPrev::Ctx<'ctx> as Context<'ctx>>::Merged<RhsPrev::Ctx<'ctx>>;

    type Item = Output;
    type Meta = ();
    type CanPend = <LhsPrev::CanPend as Toggle>::Or<RhsPrev::CanPend>;
    type CanEnd = <LhsPrev::CanEnd as Toggle>::And<RhsPrev::CanEnd>;

    fn pull(
        self: Pin<&mut Self>,
        ctx: &mut Self::Ctx<'_>,
    ) -> Step<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let mut this = self.project();

        loop {
            let lhs_step = this
                .lhs_prev
                .as_mut()
                .pull(<LhsPrev::Ctx<'_> as Context<'_>>::unmerge_self(ctx));
            let lhs_pending = matches!(lhs_step, Step::Pending(_));
            let mut live = false;

            if let Step::Ready(lhs_item, _meta) = lhs_step {
                live = true;
                let delta = this.func.call(lhs_item, this.rhs_state.borrow().clone());
                if let Some(output) = this.output.as_mut() {
                    output.merge(delta);
                } else {
                    this.output.replace(delta);
                }
            }

            let rhs_step = this
                .rhs_prev
                .as_mut()
                .pull(<LhsPrev::Ctx<'_> as Context<'_>>::unmerge_other(ctx));
            let rhs_pending = matches!(rhs_step, Step::Pending(_));
            if let Step::Ready(rhs_item, _meta) = rhs_step {
                live = true;
                let delta = this.func.call(this.lhs_state.borrow().clone(), rhs_item);
                if let Some(output) = this.output.as_mut() {
                    output.merge(delta);
                } else {
                    this.output.replace(delta);
                }
            }

            if rhs_pending && lhs_pending {
                return Step::Pending(Toggle::convert_from(Yes));
            }

            if !live && !rhs_pending && !lhs_pending {
                return if let Some(output) = this.output.take() {
                    Step::Ready(output, ())
                } else {
                    Step::Ended(Toggle::convert_from(Yes))
                };
            }
            // Both streams may continue to be polled EOS (`None`) on subsequent loops or calls, so they must be fused.
        }
    }
}
