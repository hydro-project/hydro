use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq, Hash)]
pub enum ClientClass {
    Basic,
    Prime,
}

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize, Debug, Default)]
pub struct LineItem {
    name: String,
    qty: i16,
}

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize, Debug)]
pub struct ClLineItem {
    pub client: usize,
    pub li: LineItem,
}

pub struct Checkout {
    pub client: usize,
}

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize, Debug)]
pub enum Request {
    ClLineItem { client: usize, li: LineItem },
    Checkout { client: usize },
}
