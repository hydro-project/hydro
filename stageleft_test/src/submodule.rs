use stageleft::{q, BorrowBounds, Quoted};

struct PrivateStruct {
    a: u32,
}

pub struct PublicStruct {
    // TODO(shadaj): right now, public structs must have public fields
    // because otherwise they may not be visible at splice time.
    // This should be documented and ideally have some tooling support.
    #[allow(clippy::allow_attributes, dead_code, reason = "// TODO(shadaj)")]
    pub a: u32,
}

#[stageleft::entry]
pub fn private_struct(_ctx: BorrowBounds<'_>) -> impl Quoted<u32> {
    q!({
        let my_struct = PrivateStruct { a: 1 };
        my_struct.a
    })
}

#[stageleft::entry]
pub fn public_struct(_ctx: BorrowBounds<'_>) -> impl Quoted<PublicStruct> {
    q!(PublicStruct { a: 1 })
}
