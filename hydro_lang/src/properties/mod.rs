//! Types for reasoning about algebraic properties for Rust closures.

use std::marker::PhantomData;

use stageleft::properties::Property;

use crate::live_collections::boundedness::Boundedness;
use crate::live_collections::keyed_singleton::KeyedSingletonBound;
use crate::live_collections::singleton::SingletonBound;
use crate::live_collections::stream::{ExactlyOnce, Ordering, Retries, TotalOrder};
use crate::sim_hooks::OrderingHook;

/// A trait for proof mechanisms that can validate commutativity.
///
/// `T` and `B` name the element type and boundedness of the stream the commutative
/// function consumes. The simulator does not trust commutativity proofs — it still
/// explores the input ordering — so a proof may carry an [`OrderingHook`] for scripting
/// that exploration, surfaced through [`Self::take_hook`].
#[sealed::sealed]
pub trait CommutativeProof<T, B: Boundedness> {
    /// Registers the expression with the proof mechanism.
    ///
    /// This should not perform any blocking analysis; it is only intended to record the expression for later processing.
    fn register_proof(&self, expr: &syn::Expr);

    /// Takes the simulator ordering hook attached to this proof, if any.
    fn take_hook(&mut self) -> Option<OrderingHook<T, B>>;
}

/// A trait for proof mechanisms that can validate idempotence.
#[sealed::sealed]
pub trait IdempotentProof {
    /// Registers the expression with the proof mechanism.
    ///
    /// This should not perform any blocking analysis; it is only intended to record the expression for later processing.
    fn register_proof(&self, expr: &syn::Expr);
}

/// A trait for proof mechanisms that can validate monotonicity.
#[sealed::sealed]
pub trait MonotoneProof {
    /// Registers the expression with the proof mechanism.
    ///
    /// This should not perform any blocking analysis; it is only intended to record the expression for later processing.
    fn register_proof(&self, expr: &syn::Expr);
}

/// A trait for proof mechanisms that can validate order-preservation (monotonicity of a map function).
#[sealed::sealed]
pub trait OrderPreservingProof {
    /// Registers the expression with the proof mechanism.
    ///
    /// This should not perform any blocking analysis; it is only intended to record the expression for later processing.
    fn register_proof(&self, expr: &syn::Expr);
}

/// A trait for proof mechanisms that can validate consistency of a collection.
#[sealed::sealed]
pub trait ConsistencyProof {}

/// A hand-written human proof of the correctness property.
///
/// To create a manual proof, use the [`manual_proof!`] macro, which takes in a doc comment
/// explaining why the property holds.
///
/// Manual proofs are not trusted by the simulator, which still explores the guarded
/// non-determinism. `H` is the simulator hook payload (like [`crate::nondet::NonDet`]) so a
/// commutativity proof can carry an ordering hook for scripting that exploration.
pub struct ManualProof<H = ()> {
    hook: H,
}

impl<H> ManualProof<H> {
    #[doc(hidden)]
    pub fn unhooked() -> Self
    where
        H: Default,
    {
        ManualProof { hook: H::default() }
    }
}

impl<T, B: Boundedness> ManualProof<Option<OrderingHook<T, B>>> {
    #[doc(hidden)]
    pub fn hooked(hook: impl Into<Option<OrderingHook<T, B>>>) -> Self {
        ManualProof { hook: hook.into() }
    }
}

#[sealed::sealed]
impl<T, B: Boundedness> CommutativeProof<T, B> for ManualProof<Option<OrderingHook<T, B>>> {
    fn register_proof(&self, _expr: &syn::Expr) {}

    fn take_hook(&mut self) -> Option<OrderingHook<T, B>> {
        self.hook.take()
    }
}

#[sealed::sealed]
impl<T, B: Boundedness> CommutativeProof<T, B> for ManualProof {
    fn register_proof(&self, _expr: &syn::Expr) {}

    fn take_hook(&mut self) -> Option<OrderingHook<T, B>> {
        None
    }
}
#[sealed::sealed]
impl IdempotentProof for ManualProof {
    fn register_proof(&self, _expr: &syn::Expr) {}
}
#[sealed::sealed]
impl MonotoneProof for ManualProof {
    fn register_proof(&self, _expr: &syn::Expr) {}
}
#[sealed::sealed]
impl OrderPreservingProof for ManualProof {
    fn register_proof(&self, _expr: &syn::Expr) {}
}
#[sealed::sealed]
impl ConsistencyProof for ManualProof {}

#[doc(inline)]
pub use crate::__manual_proof__ as manual_proof;

#[macro_export]
/// Fulfills a proof parameter by declaring a human-written justification for why
/// the algebraic property (e.g. commutativity, idempotence) holds.
///
/// The argument must be a doc comment explaining why the property is satisfied.
///
/// # Examples
/// ```rust
/// # #[cfg(feature = "deploy")] {
/// # use hydro_lang::prelude::*;
/// # use hydro_lang::live_collections::stream::NoOrder;
/// # use futures::StreamExt;
/// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
/// # let stream = process.source_iter(q!(vec![1, 2, 3])).weaken_ordering::<NoOrder>();
/// // stream: [1, 2, 3] (unordered)
/// stream
///     .fold(
///         q!(|| 0),
///         q!(
///             |acc, x| *acc += x,
///             commutative = manual_proof!(/** integer addition is commutative */)
///         ),
///     )
///     .into_stream()
/// # }, |mut stream| async move {
/// # assert_eq!(stream.next().await.unwrap(), 6);
/// # }));
/// # }
/// ```
/// An optional trailing `hook = ...` argument attaches a **simulator ordering hook** to a
/// commutativity proof (see `hydro_lang::sim::hooks`). The simulator does not trust manual
/// proofs — it still explores the input ordering — so the hook lets a simulation test
/// script that exploration:
///
/// ```rust,ignore
/// commutative = manual_proof!(/** set insert is commutative */ hook = my_ordering_hook)
/// ```
macro_rules! __manual_proof__ {
    ($(#[doc = $doc:expr])+hook = $hook:expr $(,)?) => {
        $crate::properties::ManualProof::hooked($hook)
    };
    ($(#[doc = $doc:expr])+) => {
        $crate::properties::ManualProof::<()>::unhooked()
    };
}

/// Marks that the property is not proved.
pub enum NotProved {}

/// Marks that the property is proven.
pub enum Proved {}

/// Algebraic properties for an aggregation function of type (T, &mut A) -> ().
///
/// Commutativity:
/// ```rust,ignore
/// let mut state = ???;
/// f(a, &mut state); f(b, &mut state) // produces same final state as
/// f(b, &mut state); f(a, &mut state)
/// ```
///
/// Idempotence:
/// ```rust,ignore
/// let mut state = ???;
/// f(a, &mut state);
/// let state1 = *state;
/// f(a, &mut state);
/// // state1 must be equal to state
/// ```
pub struct AggFuncAlgebra<
    T = (),
    B: Boundedness = crate::live_collections::boundedness::Unbounded,
    Commutative = NotProved,
    Idempotent = NotProved,
    Monotone = NotProved,
>(
    Option<Box<dyn CommutativeProof<T, B>>>,
    Option<Box<dyn IdempotentProof>>,
    Option<Box<dyn MonotoneProof>>,
    PhantomData<(Commutative, Idempotent, Monotone)>,
);

impl<T, B: Boundedness, C, I, M> AggFuncAlgebra<T, B, C, I, M> {
    /// Marks the function as being commutative, with the given proof mechanism.
    pub fn commutative(
        self,
        proof: impl CommutativeProof<T, B> + 'static,
    ) -> AggFuncAlgebra<T, B, Proved, I, M> {
        AggFuncAlgebra(Some(Box::new(proof)), self.1, self.2, PhantomData)
    }

    /// Marks the function as being idempotent, with the given proof mechanism.
    pub fn idempotent(
        self,
        proof: impl IdempotentProof + 'static,
    ) -> AggFuncAlgebra<T, B, C, Proved, M> {
        AggFuncAlgebra(self.0, Some(Box::new(proof)), self.2, PhantomData)
    }

    /// Marks the function as being monotone, with the given proof mechanism.
    pub fn monotone(
        self,
        proof: impl MonotoneProof + 'static,
    ) -> AggFuncAlgebra<T, B, C, I, Proved> {
        AggFuncAlgebra(self.0, self.1, Some(Box::new(proof)), PhantomData)
    }

    /// Registers the expression with the underlying proof mechanisms, and takes the
    /// simulator ordering hook attached to the commutativity proof, if any.
    pub(crate) fn register_proof(self, expr: &syn::Expr) -> Option<OrderingHook<T, B>> {
        let mut hook = None;
        if let Some(mut comm_proof) = self.0 {
            comm_proof.register_proof(expr);
            hook = comm_proof.take_hook();
        }

        if let Some(idem_proof) = self.1 {
            idem_proof.register_proof(expr);
        }

        if let Some(monotone_proof) = self.2 {
            monotone_proof.register_proof(expr);
        }

        hook
    }
}

impl<T, B: Boundedness, C, I, M> Property for AggFuncAlgebra<T, B, C, I, M> {
    type Root = AggFuncAlgebra<T, B>;

    fn make_root(_target: &mut Option<Self>) -> Self::Root {
        AggFuncAlgebra(None, None, None, PhantomData)
    }
}

/// Algebraic properties for a singleton map function of type T -> U.
///
/// Order-preserving means that if the input grows monotonically, the output also grows monotonically.
pub struct SingletonMapFuncAlgebra<
    T = (),
    B: Boundedness = crate::live_collections::boundedness::Unbounded,
    OrderPreserving = NotProved,
    Commutative = NotProved,
    Idempotent = NotProved,
>(
    Option<Box<dyn OrderPreservingProof>>,
    Option<Box<dyn CommutativeProof<T, B>>>,
    Option<Box<dyn IdempotentProof>>,
    PhantomData<(OrderPreserving, Commutative, Idempotent)>,
);

impl<T, B: Boundedness, O, C, I> SingletonMapFuncAlgebra<T, B, O, C, I> {
    /// Marks the function as being order-preserving, with the given proof mechanism.
    pub fn order_preserving(
        self,
        proof: impl OrderPreservingProof + 'static,
    ) -> SingletonMapFuncAlgebra<T, B, Proved, C, I> {
        SingletonMapFuncAlgebra(Some(Box::new(proof)), self.1, self.2, PhantomData)
    }

    /// Marks the function as being commutative, with the given proof mechanism.
    pub fn commutative(
        self,
        proof: impl CommutativeProof<T, B> + 'static,
    ) -> SingletonMapFuncAlgebra<T, B, O, Proved, I> {
        SingletonMapFuncAlgebra(self.0, Some(Box::new(proof)), self.2, PhantomData)
    }

    /// Marks the function as being idempotent, with the given proof mechanism.
    pub fn idempotent(
        self,
        proof: impl IdempotentProof + 'static,
    ) -> SingletonMapFuncAlgebra<T, B, O, C, Proved> {
        SingletonMapFuncAlgebra(self.0, self.1, Some(Box::new(proof)), PhantomData)
    }

    /// Registers the expression with the underlying proof mechanisms, and takes the
    /// simulator ordering hook attached to the commutativity proof, if any.
    pub(crate) fn register_proof(self, expr: &syn::Expr) -> Option<OrderingHook<T, B>> {
        if let Some(proof) = self.0 {
            proof.register_proof(expr);
        }
        self.1.and_then(|mut proof| {
            proof.register_proof(expr);
            proof.take_hook()
        })
    }
}

impl<T, B: Boundedness, O, C, I> Property for SingletonMapFuncAlgebra<T, B, O, C, I> {
    type Root = SingletonMapFuncAlgebra<T, B>;

    fn make_root(_target: &mut Option<Self>) -> Self::Root {
        SingletonMapFuncAlgebra(None, None, None, PhantomData)
    }
}

/// Algebraic properties for a stream map function of type T -> U.
pub struct StreamMapFuncAlgebra<
    T = (),
    B: Boundedness = crate::live_collections::boundedness::Unbounded,
    Commutative = NotProved,
    Idempotent = NotProved,
>(
    Option<Box<dyn CommutativeProof<T, B>>>,
    Option<Box<dyn IdempotentProof>>,
    PhantomData<(Commutative, Idempotent)>,
);

impl<T, B: Boundedness, C, I> StreamMapFuncAlgebra<T, B, C, I> {
    /// Marks the function as being commutative, with the given proof mechanism.
    pub fn commutative(
        self,
        proof: impl CommutativeProof<T, B> + 'static,
    ) -> StreamMapFuncAlgebra<T, B, Proved, I> {
        StreamMapFuncAlgebra(Some(Box::new(proof)), self.1, PhantomData)
    }

    /// Marks the function as being idempotent, with the given proof mechanism.
    pub fn idempotent(
        self,
        proof: impl IdempotentProof + 'static,
    ) -> StreamMapFuncAlgebra<T, B, C, Proved> {
        StreamMapFuncAlgebra(self.0, Some(Box::new(proof)), PhantomData)
    }

    /// Registers the expression with the underlying proof mechanisms, and takes the
    /// simulator ordering hook attached to the commutativity proof, if any.
    pub(crate) fn register_proof(self, expr: &syn::Expr) -> Option<OrderingHook<T, B>> {
        let hook = self.0.and_then(|mut proof| {
            proof.register_proof(expr);
            proof.take_hook()
        });
        if let Some(proof) = self.1 {
            proof.register_proof(expr);
        }
        hook
    }
}

impl<T, B: Boundedness, C, I> Property for StreamMapFuncAlgebra<T, B, C, I> {
    type Root = StreamMapFuncAlgebra<T, B>;

    fn make_root(_target: &mut Option<Self>) -> Self::Root {
        StreamMapFuncAlgebra(None, None, PhantomData)
    }
}

/// Marker trait identifying that the commutativity property is valid for the given stream ordering.
#[diagnostic::on_unimplemented(
    message = "Because the input stream has ordering `{O}`, the closure must demonstrate commutativity with a `commutative = ...` annotation.",
    label = "required for this call",
    note = "To intentionally process the stream by observing a non-deterministic (shuffled) order of elements, use `.assume_ordering`. This introduces non-determinism so avoid unless necessary."
)]
#[sealed::sealed]
pub trait ValidCommutativityFor<O: Ordering> {}
#[sealed::sealed]
impl ValidCommutativityFor<TotalOrder> for NotProved {}
#[sealed::sealed]
impl<O: Ordering> ValidCommutativityFor<O> for Proved {}

/// Marker trait identifying that the idempotence property is valid for the given stream ordering.
#[diagnostic::on_unimplemented(
    message = "Because the input stream has retries `{R}`, the closure must demonstrate idempotence with an `idempotent = ...` annotation.",
    label = "required for this call",
    note = "To intentionally process the stream by observing non-deterministic (randomly duplicated) retries, use `.assume_retries`. This introduces non-determinism so avoid unless necessary."
)]
#[sealed::sealed]
pub trait ValidIdempotenceFor<R: Retries> {}
#[sealed::sealed]
impl ValidIdempotenceFor<ExactlyOnce> for NotProved {}
#[sealed::sealed]
impl<R: Retries> ValidIdempotenceFor<R> for Proved {}

/// Marker trait identifying that the commutativity property is valid for the given stream ordering.
#[sealed::sealed]
#[diagnostic::on_unimplemented(
    message = "Because the input stream has ordering `{O}`, the closure must demonstrate commutativity with a `commutative = ...` annotation.",
    label = "required for this call",
    note = "To intentionally process the stream by observing a non-deterministic (shuffled) order of elements, use `.assume_ordering`. This introduces non-determinism so avoid unless necessary."
)]
pub trait ValidMutCommutativityFor<F: FnMut(In) -> Out, In, Out, O: Ordering, const WAS_MUT: bool> {}
#[sealed::sealed]
impl<In, Out, F: FnMut(In) -> Out> ValidMutCommutativityFor<F, In, Out, TotalOrder, true>
    for NotProved
{
}
#[sealed::sealed]
impl<In, Out, F: Fn(In) -> Out, O: Ordering> ValidMutCommutativityFor<F, In, Out, O, false>
    for NotProved
{
}
#[sealed::sealed]
impl<In, Out, F: FnMut(In) -> Out, O: Ordering> ValidMutCommutativityFor<F, In, Out, O, true>
    for Proved
{
}
#[sealed::sealed]
impl<In, Out, F: Fn(In) -> Out, O: Ordering> ValidMutCommutativityFor<F, In, Out, O, false>
    for Proved
{
}

/// Marker trait identifying that the idempotence property is valid for the given stream ordering.
#[diagnostic::on_unimplemented(
    message = "Because the input stream has retries `{R}`, the closure must demonstrate idempotence with an `idempotent = ...` annotation.",
    label = "required for this call",
    note = "To intentionally process the stream by observing non-deterministic (randomly duplicated) retries, use `.assume_retries`. This introduces non-determinism so avoid unless necessary."
)]
#[sealed::sealed]
pub trait ValidMutIdempotenceFor<F: FnMut(In) -> Out, In, Out, R: Retries, const WAS_MUT: bool> {}
#[sealed::sealed]
impl<In, Out, F: FnMut(In) -> Out> ValidMutIdempotenceFor<F, In, Out, ExactlyOnce, true>
    for NotProved
{
}
#[sealed::sealed]
impl<In, Out, F: Fn(In) -> Out, R: Retries> ValidMutIdempotenceFor<F, In, Out, R, false>
    for NotProved
{
}
#[sealed::sealed]
impl<In, Out, F: FnMut(In) -> Out, R: Retries> ValidMutIdempotenceFor<F, In, Out, R, true>
    for Proved
{
}
#[sealed::sealed]
impl<In, Out, F: Fn(In) -> Out, R: Retries> ValidMutIdempotenceFor<F, In, Out, R, false>
    for Proved
{
}

/// Marker trait for commutativity of closures that borrow their input (`FnMut(&In) -> Out`).
#[sealed::sealed]
#[diagnostic::on_unimplemented(
    message = "Because the input stream has ordering `{O}`, the closure must demonstrate commutativity with a `commutative = ...` annotation.",
    label = "required for this call",
    note = "To intentionally process the stream by observing a non-deterministic (shuffled) order of elements, use `.assume_ordering`. This introduces non-determinism so avoid unless necessary."
)]
pub trait ValidMutBorrowCommutativityFor<
    F: FnMut(&In) -> Out,
    In: ?Sized,
    Out,
    O: Ordering,
    const WAS_MUT: bool,
>
{
}
#[sealed::sealed]
impl<In: ?Sized, Out, F: FnMut(&In) -> Out>
    ValidMutBorrowCommutativityFor<F, In, Out, TotalOrder, true> for NotProved
{
}
#[sealed::sealed]
impl<In: ?Sized, Out, F: Fn(&In) -> Out, O: Ordering>
    ValidMutBorrowCommutativityFor<F, In, Out, O, false> for NotProved
{
}
#[sealed::sealed]
impl<In: ?Sized, Out, F: FnMut(&In) -> Out, O: Ordering>
    ValidMutBorrowCommutativityFor<F, In, Out, O, true> for Proved
{
}
#[sealed::sealed]
impl<In: ?Sized, Out, F: Fn(&In) -> Out, O: Ordering>
    ValidMutBorrowCommutativityFor<F, In, Out, O, false> for Proved
{
}

/// Marker trait for idempotence of closures that borrow their input (`FnMut(&In) -> Out`).
#[diagnostic::on_unimplemented(
    message = "Because the input stream has retries `{R}`, the closure must demonstrate idempotence with an `idempotent = ...` annotation.",
    label = "required for this call",
    note = "To intentionally process the stream by observing non-deterministic (randomly duplicated) retries, use `.assume_retries`. This introduces non-determinism so avoid unless necessary."
)]
#[sealed::sealed]
pub trait ValidMutBorrowIdempotenceFor<
    F: FnMut(&In) -> Out,
    In: ?Sized,
    Out,
    R: Retries,
    const WAS_MUT: bool,
>
{
}
#[sealed::sealed]
impl<In: ?Sized, Out, F: FnMut(&In) -> Out>
    ValidMutBorrowIdempotenceFor<F, In, Out, ExactlyOnce, true> for NotProved
{
}
#[sealed::sealed]
impl<In: ?Sized, Out, F: Fn(&In) -> Out, R: Retries>
    ValidMutBorrowIdempotenceFor<F, In, Out, R, false> for NotProved
{
}
#[sealed::sealed]
impl<In: ?Sized, Out, F: FnMut(&In) -> Out, R: Retries>
    ValidMutBorrowIdempotenceFor<F, In, Out, R, true> for Proved
{
}
#[sealed::sealed]
impl<In: ?Sized, Out, F: Fn(&In) -> Out, R: Retries>
    ValidMutBorrowIdempotenceFor<F, In, Out, R, false> for Proved
{
}

/// Marker trait identifying the boundedness of a singleton given a monotonicity property of
/// an aggregation on a stream.
#[sealed::sealed]
pub trait ApplyMonotoneStream<P, B2: SingletonBound> {}

#[sealed::sealed]
impl<B: Boundedness> ApplyMonotoneStream<NotProved, B> for B {}

#[sealed::sealed]
impl<B: Boundedness> ApplyMonotoneStream<Proved, B::StreamToMonotone> for B {}

/// Marker trait identifying the boundedness of a singleton given a monotonicity property of
/// an aggregation on a keyed stream.
#[sealed::sealed]
pub trait ApplyMonotoneKeyedStream<P, B2: KeyedSingletonBound> {}

#[sealed::sealed]
impl<B: Boundedness> ApplyMonotoneKeyedStream<NotProved, B::KeyedStreamToNonMonotone> for B {}

#[sealed::sealed]
impl<B: Boundedness> ApplyMonotoneKeyedStream<Proved, B::KeyedStreamToMonotone> for B {}

/// Marker trait identifying the boundedness of a singleton after a map operation,
/// given an order-preserving property.
#[sealed::sealed]
pub trait ApplyOrderPreservingSingleton<P, B2: SingletonBound> {}

#[sealed::sealed]
impl<B: SingletonBound> ApplyOrderPreservingSingleton<NotProved, B::UnderlyingBound> for B {}

#[sealed::sealed]
impl<B: SingletonBound> ApplyOrderPreservingSingleton<Proved, B> for B {}
