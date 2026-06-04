use crate::{Atomize, DeepReveal, IsBot, IsTop, LatticeFrom, LatticeOrd, Merge};

impl DeepReveal for () {
    type Revealed = ();

    fn deep_reveal(self) -> Self::Revealed {
        self
    }
}

impl Merge<Self> for () {
    fn merge(&mut self, _other: Self) -> bool {
        false
    }
}

impl LatticeOrd for () {}

impl LatticeFrom<Self> for () {
    fn lattice_from(other: Self) -> Self {
        other
    }
}

impl IsBot for () {
    fn is_bot(&self) -> bool {
        true
    }
}

impl IsTop for () {
    fn is_top(&self) -> bool {
        true
    }
}

impl Atomize for () {
    type Atom = Self;

    type AtomIter = core::iter::Empty<Self>;

    fn atomize(self) -> Self::AtomIter {
        core::iter::empty()
    }
}
