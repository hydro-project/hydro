//! [`DemuxMap`] and related items.
use core::fmt::Debug;
use core::hash::Hash;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::collections::HashMap;

use crate::{Sink, ready_both};

/// Sink which receives keys paired with items `(Key, Item)`, and pushes to the corresponding output sink in a [`HashMap`] of sinks.
pub struct DemuxMap<Key, Si> {
    sinks: HashMap<Key, Si>,
}

impl<Key, Si> DemuxMap<Key, Si> {
    /// Create with the given next `sinks` map.
    pub fn new<Item>(sinks: impl Into<HashMap<Key, Si>>) -> Self
    where
        Self: Sink<(Key, Item)>,
    {
        Self {
            sinks: sinks.into(),
        }
    }
}

impl<Key, Si, Item> Sink<(Key, Item)> for DemuxMap<Key, Si>
where
    Key: Eq + Hash + Debug + Unpin,
    Si: Sink<Item> + Unpin,
{
    type Error = Si::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.get_mut()
            .sinks
            .values_mut()
            .try_fold(Poll::Ready(()), |poll, sink| {
                ready_both!(poll, Pin::new(sink).poll_ready(cx)?);
                Poll::Ready(Ok(()))
            })
    }

    fn start_send(self: Pin<&mut Self>, item: (Key, Item)) -> Result<(), Self::Error> {
        let sink = self
            .get_mut()
            .sinks
            .get_mut(&item.0)
            .unwrap_or_else(|| panic!("`DemuxMap` missing key {:?}", item.0));
        Pin::new(sink).start_send(item.1)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.get_mut()
            .sinks
            .values_mut()
            .try_fold(Poll::Ready(()), |poll, sink| {
                ready_both!(poll, Pin::new(sink).poll_flush(cx)?);
                Poll::Ready(Ok(()))
            })
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.get_mut()
            .sinks
            .values_mut()
            .try_fold(Poll::Ready(()), |poll, sink| {
                ready_both!(poll, Pin::new(sink).poll_close(cx)?);
                Poll::Ready(Ok(()))
            })
    }
}

/// Creates a `DemuxMap` sink that sends each item to one of many outputs, depending on the key.
///
/// This requires sinks `Si` to be `Unpin`. If your sinks are not `Unpin`, first wrap them in `Box::pin` to make them `Unpin`.
pub fn demux_map<Key, Si, Item>(sinks: impl Into<HashMap<Key, Si>>) -> DemuxMap<Key, Si>
where
    Key: Eq + Hash + Debug + Unpin,
    Si: Sink<Item> + Unpin,
{
    DemuxMap::new(sinks)
}
