pub mod bottom;
pub mod dom_pair;
pub mod map_union;
pub mod ord;
pub mod pair;
pub mod set_union;
pub mod top;

pub trait Lattice {}

pub trait LatticeRepr {
    type Lattice: Lattice;
    type Repr: Clone;
}

pub trait Merge<Delta: LatticeRepr>: LatticeRepr<Lattice = Delta::Lattice> {
    /// Merge `delta` into `this`. Return `true` if `this` changed, `false` if `this` was unchanged.
    fn merge(this: &mut Self::Repr, delta: Delta::Repr) -> bool;

    /// Merge `this` and `delta` together, returning the new value.
    fn merge_owned(mut this: Self::Repr, delta: Delta::Repr) -> Self::Repr {
        Self::merge(&mut this, delta);
        this
    }
}

pub trait Convert<Target: LatticeRepr<Lattice = Self::Lattice>>: LatticeRepr {
    fn convert(this: Self::Repr) -> Target::Repr;
}

pub trait Compare<Other: LatticeRepr<Lattice = Self::Lattice>>: LatticeRepr {
    fn compare(this: &Self::Repr, other: &Other::Repr) -> Option<std::cmp::Ordering>;
}

pub trait Debottom: LatticeRepr {
    fn is_bottom(this: &Self::Repr) -> bool;

    type DebottomLr: LatticeRepr<Lattice = Self::Lattice>;
    fn debottom(this: Self::Repr) -> Option<<Self::DebottomLr as LatticeRepr>::Repr>;
}

pub trait Top: LatticeRepr {
    fn is_top(this: &Self::Repr) -> bool;
    fn top() -> Self::Repr;
}
