#[derive(Copy, Clone)]
pub struct NonDet;

pub fn local_nondet(_: &'static str) -> NonDet {
    NonDet
}

pub fn partial_exposed_nondet(_: &'static str, _nondet: NonDet) -> NonDet {
    NonDet
}
