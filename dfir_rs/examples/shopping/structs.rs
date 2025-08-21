use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq, Hash)]
pub enum ClientClass {
    Basic,
    Prime,
}

#[derive(PartialEq, Eq, Clone, Hash, Serialize, Deserialize, Debug, Default)]
pub struct LineItem {
    pub name: String,
    pub qty: i16,
}

#[derive(PartialEq, Eq, Clone, Hash, Serialize, Deserialize, Debug)]
pub enum Request {
    ClLineItem { client: usize, li: LineItem },
    Checkout { client: usize },
}
