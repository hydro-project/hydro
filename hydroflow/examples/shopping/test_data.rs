use crate::structs::{ClientClass, LineItem, Request};

const apple: &str = "apple";
const banana: &str = "banana";
const ferrari: &str = "ferrari";
const potato: &str = "potato";

pub fn client1_vec() -> Vec<Request> {
    vec![
        Request::ClLineItem {
            client: 1,
            li: LineItem {
                name: apple.to_string(),
                qty: 1,
            },
        },
        Request::ClLineItem {
            client: 1,
            li: LineItem {
                name: banana.to_string(),
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
                name: apple.to_string(),
                qty: 1,
            },
        },
        Request::ClLineItem {
            client: 2,
            li: LineItem {
                name: apple.to_string(),
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
                name: potato.to_string(),
                qty: 1,
            },
        },
        Request::ClLineItem {
            client: 100,
            li: LineItem {
                name: ferrari.to_string(),
                qty: 1,
            },
        },
        Request::Checkout { client: 100 },
    ]
}

pub fn all_clients_iter() -> impl Iterator<Item = Request> {
    client1_vec()
        .into_iter()
        .chain(client2_vec().into_iter())
        .chain(client100_vec().into_iter())
}

pub fn client_class_iter() -> impl Iterator<Item = (usize, ClientClass)> {
    [
        (1, ClientClass::Basic),
        (2, ClientClass::Basic),
        (100, ClientClass::Prime),
    ]
    .into_iter()
}
