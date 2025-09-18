use dfir_rs::DemuxEnum;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize, Debug, Hash, Copy, DemuxEnum)]
pub enum Msg {
    Prepare(u16),
    Commit(u16),
    Abort(u16),
    AckP2(u16),
    End(u16),
    Ended(u16),
}
