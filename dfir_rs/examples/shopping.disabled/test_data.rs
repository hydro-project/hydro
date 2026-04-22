use crate::structs::{ClientClass, LineItem, Request};

const APPLE: &str = "apple";
const BANANA: &str = "banana";
const FERRARI: &str = "ferrari";
const POTATO: &str = "potato";

pub fn client1_vec() -> Vec<Request> {
    vec![
        Request::ClLineItem {
            client: 1,
            li: LineItem {
                name: APPLE.to_owned(),
                qty: 1,
            },
        },
        Request::ClLineItem {
            client: 1,
            li: LineItem {
                name: BANANA.to_owned(),
                qty: 6,
            },
        },
        Request::Checkout { client: 1 },
    ]
}

pub fn client2_vec() -> Vec<Request> {
    vec![
        Request::ClLineItem {
            client: 2,
            li: LineItem {
                name: APPLE.to_owned(),
                qty: 1,
            },
        },
        Request::ClLineItem {
            client: 2,
            li: LineItem {
                name: APPLE.to_owned(),
                qty: -1,
            },
        },
        Request::Checkout { client: 2 },
    ]
}

pub fn client100_vec() -> Vec<Request> {
    vec![
        Request::ClLineItem {
            client: 100,
            li: LineItem {
                name: POTATO.to_owned(),
                qty: 1,
            },
        },
        Request::ClLineItem {
            client: 100,
            li: LineItem {
                name: FERRARI.to_owned(),
                qty: 1,
            },
        },
        Request::Checkout { client: 100 },
    ]
}

pub fn client_class_iter() -> impl Iterator<Item = (usize, ClientClass)> {
    [
        (1, ClientClass::Basic),
        (2, ClientClass::Basic),
        (100, ClientClass::Prime),
    ]
    .into_iter()
}
