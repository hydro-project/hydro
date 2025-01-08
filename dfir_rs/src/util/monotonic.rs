/// A wrapper christening a closure as a [monotonic function](https://hydro.run/docs/hydroflow/lattices_crate/lattice_math#the-calm-theorem-and-monotonicity)
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct MonotonicFn<F>(pub F);

/// A wrapper christening a closure as a [lattice morphism](https://hydro.run/docs/hydroflow/lattices_crate/lattice_math#lattice-morphism)
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Morphism<F>(pub F);
