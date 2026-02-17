use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::{Pull, Step, Yes};

pin_project! {
    pub struct Scan<Prev, State, Func> {
        #[pin]
        prev: Prev,
        state: Option<State>,
        func: Func,
    }
}

impl<Prev, State, Func> Scan<Prev, State, Func> {
    pub fn new(prev: Prev, initial_state: State, func: Func) -> Self {
        Self {
            prev,
            state: Some(initial_state),
            func,
        }
    }
}

impl<Prev, State, Func, Item> Pull for Scan<Prev, State, Func>
where
    Prev: Pull,
    Func: FnMut(&mut State, Prev::Item) -> Option<Item>,
{
    type Ctx<'ctx> = Prev::Ctx<'ctx>;

    type Item = Item;
    type Meta = Prev::Meta;
    type CanPend = Prev::CanPend;
    type CanEnd = Yes;

    fn pull(
        self: Pin<&mut Self>,
        ctx: &mut Self::Ctx<'_>,
    ) -> Step<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let mut this = self.project();

        let state = match this.state.as_mut() {
            Some(s) => s,
            None => return Step::Ended(Yes),
        };

        loop {
            return match this.prev.as_mut().pull(ctx) {
                Step::Ready(item, meta) => match (this.func)(state, item) {
                    Some(output) => Step::Ready(output, meta),
                    None => {
                        *this.state = None;
                        Step::Ended(Yes)
                    }
                },
                Step::Pending(can_pend) => Step::Pending(can_pend),
                Step::Ended(_) => Step::Ended(Yes),
            };
        }
    }

    fn size_hint(self: Pin<&Self>) -> (usize, Option<usize>) {
        let this = self.project_ref();
        if this.state.is_none() {
            (0, Some(0))
        } else {
            let (_, upper) = this.prev.size_hint();
            (0, upper)
        }
    }
}
