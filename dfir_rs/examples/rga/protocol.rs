use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct Timestamp {
    pub(crate) node_ts: usize,
    pub(crate) node_id: usize,
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ts{}_{}", self.node_ts, self.node_id)
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Ord, PartialOrd)]
pub(crate) struct Token {
    pub(crate) ts: Timestamp,
    pub(crate) value: char,
}
