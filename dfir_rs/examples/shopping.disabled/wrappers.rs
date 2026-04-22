use crate::lattices::{BoundedPrefix, SealedSetOfIndexedValues};
use crate::structs::{LineItem, Request};

pub fn tuple_wrap<'a>(
    it: impl 'a + Iterator<Item = Request>,
) -> impl 'a + Iterator<Item = (usize, LineItem)> {
    it.scan(false, |checked_out, item| {
        if *checked_out {
            None
        } else {
            *checked_out = matches!(item, Request::Checkout { .. });
            Some(item)
        }
    })
    .map(|item| match item {
        Request::ClLineItem { client, li } => (client, li),
        Request::Checkout { client } => (client, LineItem::default()),
    })
}

pub fn ssiv_wrap<'a>(
    it: impl 'a + Iterator<Item = Request>,
) -> impl 'a + Iterator<Item = SealedSetOfIndexedValues<Request>> {
    it.scan(false, |checked_out, item| {
        if *checked_out {
            None
        } else {
            *checked_out = matches!(item, Request::Checkout { .. });
            Some(item)
        }
    })
    .enumerate()
    .map(|(idx, item)| match item {
        Request::ClLineItem { client, li } => SealedSetOfIndexedValues::<Request> {
            set: std::iter::once((idx, Request::ClLineItem { client, li })).collect(),
            len: None,
        },
        Request::Checkout { .. } => SealedSetOfIndexedValues::<Request> {
            set: Default::default(),
            len: Some(idx + 1),
        },
    })
}

pub fn bp_wrap<'a>(
    it: impl 'a + Iterator<Item = Request>,
) -> impl 'a + Iterator<Item = BoundedPrefix<Request>> {
    let mut it = it.enumerate().peekable();
    let mut last: Vec<Request> = Default::default();
    std::iter::from_fn(move || {
        it.next().map(|(idx, x): (usize, Request)| {
            last.push(x);
            BoundedPrefix::<Request> {
                vec: last.clone(),
                len: if it.peek().is_some() {
                    None
                } else {
                    Some(idx + 1)
                },
            }
        })
    })
}
