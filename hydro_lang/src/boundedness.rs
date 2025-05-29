use sealed::sealed;

/// A marker trait indicating whether a stream’s length is bounded (finite) or unbounded (potentially infinite).
///
/// Implementors of this trait use it to signal the boundedness property of a stream.
#[sealed]
pub trait Boundedness {}

/// Marks the stream as being unbounded, which means that it is not
/// guaranteed to be complete in finite time.
pub enum Unbounded {}

#[sealed]
impl Boundedness for Unbounded {}

/// Marks the stream as being bounded, which means that it is guaranteed
/// to be complete in finite time.
pub enum Bounded {}

#[sealed]
impl Boundedness for Bounded {}
