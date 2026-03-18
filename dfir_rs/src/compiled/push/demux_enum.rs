// TODO(mingwei): Move this & derive macro to separate crate ([`sinktools`])
use std::pin::Pin;

use dfir_pipes::push::{Push, PushStep};
use pin_project_lite::pin_project;

use crate::util::demux_enum::DemuxEnumPush;

pin_project! {
    /// Special push operator for the `demux_enum` operator.
    #[must_use = "`Push`es do nothing unless items are pushed into them"]
    pub struct DemuxEnum<Outputs> {
        outputs: Outputs,
    }
}

impl<Outputs> DemuxEnum<Outputs> {
    /// Creates with the given `Outputs`.
    pub const fn new(outputs: Outputs) -> Self {
        Self { outputs }
    }
}

impl<Outputs, Item, Meta: Copy> Push<Item, Meta> for DemuxEnum<Outputs>
where
    Item: DemuxEnumPush<Outputs, Meta>,
{
    type Ctx<'ctx> = <Item as DemuxEnumPush<Outputs, Meta>>::Ctx<'ctx>;

    type CanPend = <Item as DemuxEnumPush<Outputs, Meta>>::CanPend;

    fn poll_ready(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        Item::poll_ready(self.project().outputs, ctx)
    }

    fn start_send(self: Pin<&mut Self>, item: Item, meta: Meta) {
        Item::start_send(item, meta, self.project().outputs);
    }

    fn poll_flush(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        Item::poll_flush(self.project().outputs, ctx)
    }

    fn size_hint(self: Pin<&mut Self>, hint: (usize, Option<usize>)) {
        Item::size_hint(self.project().outputs, hint)
    }
}
