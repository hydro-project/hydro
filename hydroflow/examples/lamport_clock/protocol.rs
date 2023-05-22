use hydroflow::lattices::Max;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize, Debug)]
pub struct EchoMsg {
    pub payload: String,
    pub lamport_clock: Max<usize>,
}
