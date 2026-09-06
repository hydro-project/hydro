//! Runtime helpers for driving embedded Hydro dataflow graphs from handwritten Rust.
//!
//! The embedded backend (`hydro_lang::compile::embedded`, available with the `build`
//! feature) generates one function per location. Each generated function takes
//! [`futures::Stream`] inputs and `FnMut` output callbacks, and returns a DFIR graph that
//! *borrows* from them. Storing the graph together with its inputs and outputs in a single
//! owned value therefore requires a self-referential struct, which cannot be expressed in
//! safe Rust. This module provides the building blocks for such a holder, with all `unsafe`
//! confined to this module so that downstream integration code is safe:
//!
//! - [`InputBuffer`] / [`InputStream`]: an address-stable queue that backs a
//!   [`futures::Stream`] input parameter of a generated function.
//! - [`CallbackSlot`]: an address-stable slot holding a caller-provided `FnMut(T)` for the
//!   duration of a run, backing an output (or network-out) parameter.
//! - [`AliasedBox`] / [`OwnedErasedBox`]: owned heap allocations that, unlike [`Box`], may
//!   be moved while raw pointers into their contents exist.
//! - [`DfirRunnable`]: object-safe erasure for the unnameable `Dfir<impl TickClosure>`.
//! - [`embedded_flow!`](crate::embedded_flow): a macro assembling the above into a safe
//!   holder struct.
//!
//! The same primitives support local and networked channels alike: a network-out is just a
//! `CallbackSlot<(TaglessMemberId, Bytes)>` and a network-in is just an
//! `InputBuffer<Result<(TaglessMemberId, BytesMut), io::Error>>`.
//!
//! # Threading
//!
//! All of these types are single-threaded: they are `!Sync`, and `!Send` wherever a raw
//! pointer may reference them. A holder assembled from them must live and run on one thread.
//!
//! # `no_std`
//!
//! The core primitives ([`InputBuffer`], [`CallbackSlot`]) only use `core` items, have
//! `const fn new` constructors, and communicate address stability through [`Pin`], so they
//! are compatible with future `no_std` targets using statically-allocated (fixed-size)
//! storage. Only the owning-allocation helpers ([`AliasedBox`], [`OwnedErasedBox`]) and the
//! [`embedded_flow!`](crate::embedded_flow) macro require `alloc`.

use core::cell::{Cell, UnsafeCell};
use core::fmt;
use core::marker::{PhantomData, PhantomPinned};
use core::ops::Deref;
use core::pin::Pin;
use core::ptr::NonNull;
use core::task::{Context, Poll, Waker};
use std::collections::VecDeque;

use dfir_rs::scheduled::context::{Dfir, TickClosure};

pub use crate::embedded_flow;

/// Implementation details of [`embedded_flow!`](crate::embedded_flow). Not stable API.
///
/// These re-exports exist so that the macro expansion does not depend on the invoking
/// crate's prelude, and so a future `no_std` version of `hydro_lang` can swap `std` paths
/// for `alloc` paths without changing (or breaking) the macro itself.
#[doc(hidden)]
pub mod __private {
    pub use core::cell::RefCell;
    pub use std::boxed::Box;
    pub use std::vec::Vec;
}

/// Object-safe trait for running a DFIR flow synchronously.
///
/// Generated embedded functions return `Dfir<impl TickClosure + 'a>`, whose type parameter
/// cannot be named by the caller. Boxing the graph as `Box<dyn DfirRunnable>` erases that
/// type with a single virtual call per [`run_available_sync`](Self::run_available_sync)
/// invocation. (This is cheaper than `Dfir::into_erased`, which boxes a future on every
/// tick, and also works for tick closures that only implement `TickClosure`.)
pub trait DfirRunnable {
    /// Runs ticks as long as work is available, then returns.
    ///
    /// See `Dfir::run_available_sync`; panics if a tick yields asynchronously.
    fn run_available_sync(&mut self);
}

impl<Tick: TickClosure> DfirRunnable for Dfir<Tick> {
    fn run_available_sync(&mut self) {
        Dfir::run_available_sync(self)
    }
}

/// An owned heap allocation, like [`Box`], but without `Box`'s unique-aliasing guarantee.
///
/// Moving a `Box` asserts that it is the *only* pointer to its contents (`noalias`),
/// invalidating raw pointers previously derived from them (e.g. under Stacked Borrows).
/// `AliasedBox` stores only a raw pointer, so it may be freely moved — such as into the
/// holder struct built by [`embedded_flow!`](crate::embedded_flow) — while [`InputStream`]s
/// and sink closures hold `NonNull` pointers into its contents.
///
/// The contents are never moved and never exposed as `&mut` (only [`Deref`]), so their
/// address is stable from construction until drop; this is what makes
/// [`as_pin`](Self::as_pin) sound.
pub struct AliasedBox<T> {
    ptr: NonNull<T>,
    /// Declare ownership of a `T` for drop-check purposes.
    _owns: PhantomData<T>,
}

impl<T> AliasedBox<T> {
    /// Heap-allocates `value`.
    pub fn new(value: T) -> Self {
        // SAFETY: `Box::into_raw` never returns null.
        let ptr = unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(value))) };
        Self {
            ptr,
            _owns: PhantomData,
        }
    }

    /// Returns the contents as a pinned reference.
    ///
    /// This is safe because the contents are heap-allocated, never moved, and never
    /// exposed mutably, so they remain at the same address until `self` is dropped
    /// (at which point their destructor runs, upholding the [`Pin`] drop guarantee).
    pub fn as_pin(&self) -> Pin<&T> {
        // SAFETY: see doc comment above.
        unsafe { Pin::new_unchecked(self.ptr.as_ref()) }
    }
}

impl<T> Deref for AliasedBox<T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: allocated in `new`, freed only in `drop`.
        unsafe { self.ptr.as_ref() }
    }
}

impl<T> Drop for AliasedBox<T> {
    fn drop(&mut self) {
        // SAFETY: `ptr` came from `Box::into_raw` in `new` and is dropped exactly once.
        drop(unsafe { Box::from_raw(self.ptr.as_ptr()) });
    }
}

impl<T: fmt::Debug> fmt::Debug for AliasedBox<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        T::fmt(self, f)
    }
}

/// A type-erased owned heap allocation, dropped (running `T`'s destructor) when this value
/// is dropped.
///
/// [`new`](Self::new) hands back a `&'static mut T` into the allocation, which can be lent
/// to a generated embedded function while the `OwnedErasedBox` — whose type does not mention
/// `T`, useful when `T` is unnameable (e.g. a struct of `impl FnMut` sinks) — is stored
/// alongside the flow to keep the allocation alive. Like [`AliasedBox`], only a raw pointer
/// is stored, so moving this value does not invalidate the outstanding reference.
pub struct OwnedErasedBox {
    ptr: NonNull<u8>,
    /// Drops the allocation as its original `T`.
    drop_fn: unsafe fn(NonNull<u8>),
}

impl OwnedErasedBox {
    /// Heap-allocates `value`, returning the type-erased owner along with a
    /// `&'static mut T` to the allocation.
    ///
    /// # Safety
    ///
    /// The returned reference (and anything derived from it) must not be used after the
    /// returned `OwnedErasedBox` is dropped, despite the `'static` lifetime.
    pub unsafe fn new<T>(value: T) -> (Self, &'static mut T) {
        /// SAFETY: must be called at most once, with a pointer originating from
        /// `Box::into_raw::<T>`.
        unsafe fn drop_impl<T>(ptr: NonNull<u8>) {
            // SAFETY: per this function's contract.
            drop(unsafe { Box::from_raw(ptr.cast::<T>().as_ptr()) });
        }

        let raw: *mut T = Box::into_raw(Box::new(value));
        // SAFETY: `Box::into_raw` never returns null.
        let ptr = unsafe { NonNull::new_unchecked(raw) };
        (
            Self {
                ptr: ptr.cast(),
                drop_fn: drop_impl::<T>,
            },
            // SAFETY: fresh exclusive allocation, freed only by `drop_fn`; the caller
            // promises not to use the reference after that.
            unsafe { &mut *raw },
        )
    }
}

impl Drop for OwnedErasedBox {
    fn drop(&mut self) {
        // SAFETY: `ptr` and `drop_fn` were created as a pair in `new`, and this runs once.
        unsafe { (self.drop_fn)(self.ptr) }
    }
}

impl fmt::Debug for OwnedErasedBox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OwnedErasedBox").finish_non_exhaustive()
    }
}

/// An address-stable FIFO queue that backs a [`futures::Stream`] input parameter of a
/// generated embedded function.
///
/// The queue is filled from application code via [`push`](Self::push) and drained by the
/// dataflow graph through an [`InputStream`] created with [`stream`](Self::stream). The
/// stream never terminates: while the queue is empty it returns [`Poll::Pending`], and it
/// is woken by the next `push`.
pub struct InputBuffer<T> {
    /// Queue of pending items.
    ///
    /// Interior mutability is required because [`push`](Self::push) is called through
    /// `&self` (from application code) while an [`InputStream`] holding a raw pointer to
    /// this buffer is owned by the flow. Every access is scoped to a single expression
    /// that does not call back into user code, so overlapping borrows cannot occur; all
    /// accesses happen on one thread because `InputBuffer` is `!Sync` and `!Send`.
    buf: UnsafeCell<VecDeque<T>>,
    /// The waker of the task that last polled an empty [`InputStream`], woken on `push`.
    /// Same access discipline as `buf`, except that foreign waker code is only ever run
    /// *after* ending the access (by moving the waker out of the cell).
    waker: UnsafeCell<Option<Waker>>,
    /// `!Unpin`: [`InputStream`] holds a raw pointer to this buffer, so it must not move.
    _pin: PhantomPinned,
    /// `!Send`: an [`InputStream`] on the original thread may access this buffer without
    /// synchronization, so the buffer must not be pushed to from another thread.
    _not_send: PhantomData<*mut ()>,
}

impl<T> InputBuffer<T> {
    /// Creates an empty buffer. Does not allocate.
    pub const fn new() -> Self {
        Self {
            buf: UnsafeCell::new(VecDeque::new()),
            waker: UnsafeCell::new(None),
            _pin: PhantomPinned,
            _not_send: PhantomData,
        }
    }

    /// Pushes an item onto the queue and wakes the [`InputStream`] if it is waiting.
    ///
    /// Note that the item is only *queued*; it is processed the next time the flow runs.
    pub fn push(&self, item: T) {
        // SAFETY: scoped, single-threaded access; see `Self::buf`.
        unsafe { (*self.buf.get()).push_back(item) };
        // Move the waker out of the cell *before* invoking it, so that no access to the
        // cell is in progress while running arbitrary waker code (which could re-enter
        // `push`). Consuming the waker also coalesces wakes: subsequent pushes before the
        // next poll skip the wake, and `poll_next` re-registers on `Pending`.
        // SAFETY: scoped, single-threaded access; see `Self::waker`.
        let waker = unsafe { (*self.waker.get()).take() };
        if let Some(waker) = waker {
            waker.wake();
        }
    }

    /// Returns the number of items currently queued.
    pub fn len(&self) -> usize {
        // SAFETY: scoped, single-threaded access; see `Self::buf`.
        unsafe { (*self.buf.get()).len() }
    }

    /// Returns `true` if no items are currently queued.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Creates a never-terminating [`futures::Stream`] that drains this buffer, for
    /// passing to a generated embedded function.
    ///
    /// Multiple streams over one buffer are allowed (though rarely useful): items are
    /// delivered to whichever stream polls first, and only the most recent waker is woken.
    ///
    /// # Safety
    ///
    /// The buffer must outlive the returned stream. (The buffer cannot move while the
    /// stream exists, which is guaranteed by `self` being pinned, and the stream cannot be
    /// sent to another thread, as it is `!Send`.)
    pub unsafe fn stream(self: Pin<&Self>) -> InputStream<T> {
        InputStream {
            ptr: NonNull::from(self.get_ref()),
        }
    }
}

impl<T> Default for InputBuffer<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> fmt::Debug for InputBuffer<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InputBuffer")
            .field("len", &self.len())
            .finish_non_exhaustive()
    }
}

/// A never-terminating [`futures::Stream`] that drains an [`InputBuffer`].
///
/// Created via [`InputBuffer::stream`]; see the safety requirements there.
pub struct InputStream<T> {
    ptr: NonNull<InputBuffer<T>>,
}

/// The *buffer* is what must stay pinned; the stream itself moves freely.
impl<T> Unpin for InputStream<T> {}

impl<T> futures::Stream for InputStream<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T>> {
        // SAFETY: the `InputBuffer::stream` contract guarantees the buffer is still alive.
        let buffer = unsafe { self.ptr.as_ref() };

        // Fast path: an item is available, no waker bookkeeping needed.
        // SAFETY: scoped, single-threaded access; see `InputBuffer::buf`.
        if let Some(item) = unsafe { (*buffer.buf.get()).pop_front() } {
            return Poll::Ready(Some(item));
        }

        // Register (or refresh) the waker. The old waker is moved out of the cell first so
        // that no access is in progress while `clone_from` runs foreign waker code.
        // (`Waker::clone_from` skips the clone when the old waker `will_wake` the new one.)
        // SAFETY: scoped, single-threaded accesses; see `InputBuffer::waker`.
        let mut waker = unsafe { (*buffer.waker.get()).take() };
        match &mut waker {
            Some(waker) => waker.clone_from(cx.waker()),
            None => waker = Some(cx.waker().clone()),
        }
        // SAFETY: as above.
        unsafe { *buffer.waker.get() = waker };

        // Re-check the queue: a (pathological) re-entrant `push` from inside the waker
        // clone above would have found the waker cell empty and not woken anyone, so
        // returning `Pending` without this check could lose a wakeup.
        // SAFETY: scoped, single-threaded access; see `InputBuffer::buf`.
        match unsafe { (*buffer.buf.get()).pop_front() } {
            Some(item) => Poll::Ready(Some(item)),
            None => Poll::Pending,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // SAFETY: the `InputBuffer::stream` contract guarantees the buffer is still alive.
        let buffered = unsafe { self.ptr.as_ref() }.len();
        (buffered, None) // Never terminates, so there is no upper bound.
    }
}

impl<T> fmt::Debug for InputStream<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InputStream").finish_non_exhaustive()
    }
}

/// An address-stable slot holding a caller-provided callback for the duration of a run,
/// backing an output (or network-out) parameter of a generated embedded function.
///
/// The flow side holds a sink closure (created via [`sink`](Self::sink)) that forwards each
/// emitted item to whatever callback is currently installed. The application side
/// temporarily installs a callback around each run via [`invoke_with`](Self::invoke_with):
///
/// ```ignore
/// slot.invoke_with(&mut |item| results.push(item), || flow.run_available_sync());
/// ```
///
/// Items emitted while no callback is installed are **silently dropped**.
pub struct CallbackSlot<T: 'static> {
    /// The currently-installed callback, if any. A [`Cell`] (the pointer is `Copy`), so no
    /// borrow of the slot is ever held across a call into user code.
    cell: Cell<Option<NonNull<dyn FnMut(T)>>>,
    /// `!Unpin`: sink closures hold a raw pointer to this slot, so it must not move.
    _pin: PhantomPinned,
}

impl<T: 'static> CallbackSlot<T> {
    /// Creates a slot with no callback installed.
    pub const fn new() -> Self {
        Self {
            cell: Cell::new(None),
            _pin: PhantomPinned,
        }
    }

    /// Creates a sink closure that forwards each item to the currently-installed callback,
    /// for passing to a generated embedded function.
    ///
    /// # Safety
    ///
    /// The slot must outlive the returned closure. (The slot cannot move while the closure
    /// exists, which is guaranteed by `self` being pinned, and the closure cannot be sent
    /// to another thread, as it is `!Send`.)
    pub unsafe fn sink(self: Pin<&Self>) -> impl FnMut(T) + use<T> {
        let ptr = NonNull::from(self.get_ref());
        move |item| {
            // SAFETY: the `sink` contract guarantees the slot is still alive.
            let slot = unsafe { ptr.as_ref() };
            slot.invoke(item);
        }
    }

    /// Invokes the currently-installed callback with `item`, or drops `item` if no
    /// callback is installed.
    ///
    /// The callback is moved out of the slot while it runs (and restored afterwards, even
    /// on panic), so a re-entrant invocation of the same slot finds it empty and drops its
    /// item, rather than aliasing the already-running callback.
    pub fn invoke(&self, item: T) {
        let Some(mut callback) = self.cell.take() else {
            return;
        };
        let restore = RestoreOnDrop {
            slot: self,
            prev: Some(callback),
        };
        // SAFETY: the pointer was installed by `invoke_with`, which keeps the callback
        // alive (and its `&mut` borrow active) until it uninstalls the pointer on exit.
        // Taking the pointer out of the cell above makes this `&mut` reborrow exclusive.
        unsafe { (callback.as_mut())(item) };
        drop(restore);
    }

    /// Installs `callback` for the duration of `f`, then restores whatever was previously
    /// installed — even if `f` panics. Items the flow emits to this slot while `f` runs
    /// are passed to `callback`.
    ///
    /// Calls may be nested (e.g. to install callbacks on several slots around a single
    /// run); the innermost installation wins for its duration.
    pub fn invoke_with<R>(&self, callback: &mut dyn FnMut(T), f: impl FnOnce() -> R) -> R {
        let ptr = NonNull::from(callback);
        // SAFETY(transmute): erases the (unnameable) lifetime of the trait object; this
        // only changes the type-level lifetime, not the pointer. The pointer is only
        // dereferenced by `invoke` while installed, and the guard below uninstalls it
        // before `invoke_with` returns or unwinds — i.e., strictly within the lifetime of
        // the `&mut callback` borrow.
        let ptr: NonNull<dyn FnMut(T) + 'static> = unsafe { core::mem::transmute(ptr) };
        let _restore = RestoreOnDrop {
            slot: self,
            prev: self.cell.replace(Some(ptr)),
        };
        f()
    }
}

impl<T: 'static> Default for CallbackSlot<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: 'static> fmt::Debug for CallbackSlot<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CallbackSlot")
            .field("installed", &self.cell.get().is_some())
            .finish()
    }
}

/// Restores a [`CallbackSlot`]'s previous contents on drop (including during unwinding).
struct RestoreOnDrop<'a, T: 'static> {
    slot: &'a CallbackSlot<T>,
    prev: Option<NonNull<dyn FnMut(T)>>,
}

impl<T: 'static> Drop for RestoreOnDrop<'_, T> {
    fn drop(&mut self) {
        self.slot.cell.set(self.prev);
    }
}

/// Implementation detail of [`embedded_flow!`](crate::embedded_flow): nests
/// [`CallbackSlot::invoke_with`] calls around a body expression.
#[doc(hidden)]
#[macro_export]
macro_rules! __embedded_invoke_nested {
    ((), $body:expr) => {
        $body
    };
    (($slot:expr => $cb:expr, $($rest:tt)*), $body:expr) => {
        $slot.invoke_with($cb, move || $crate::__embedded_invoke_nested!(($($rest)*), $body))
    };
}

/// Declares a holder struct that owns a generated embedded DFIR flow (see
/// `hydro_lang::compile::embedded`) *together with* the input buffers and output callback
/// slots it borrows from, exposing an entirely safe API.
///
/// The generated struct has:
/// - `fn new(...)` — constructs the buffers/slots and the flow (extra `build` parameters
///   become parameters of `new`; a `self_id` section adds a leading `self_id` parameter).
/// - one accessor per input / network-in / membership entry returning
///   [`&InputBuffer<_>`](crate::embedded::InputBuffer), to [`push`](crate::embedded::InputBuffer::push) items into;
/// - one accessor per output / network-out entry returning
///   [`&CallbackSlot<_>`](crate::embedded::CallbackSlot);
/// - `fn run(&self)` — runs the flow until no more work is available (outputs emitted with
///   no callback installed are dropped);
/// - `fn run_with(&self, ...)` — runs the flow with a `&mut dyn FnMut(_)` callback
///   installed for each output, then each network-out, in declaration order.
///
/// # Sections
///
/// All sections are optional except `build`, but must appear in the order shown. Each
/// `Path as binder` clause names a struct generated by the embedded backend (e.g.
/// `my_fn::EmbeddedOutputs`) and binds the value passed to the generated function to a
/// local usable inside `build`. Input names are bound directly as stream locals.
///
/// ```ignore
/// hydro_lang::embedded_flow! {
///     /// Holder for my flow.
///     pub struct MyFlow {
///         // For cluster locations: the member id of this instance.
///         self_id(my_id: TaglessMemberId);
///         // Cluster membership streams; field names are the cluster fn names.
///         membership(my_fn::EmbeddedMembershipStreams as memberships) { other_cluster }
///         inputs { events: MyEvent }
///         network_in(my_fn::EmbeddedNetworkIn as net_in) {
///             requests: Result<BytesMut, std::io::Error>,
///         }
///         outputs(my_fn::EmbeddedOutputs as outputs) { results: MyResult }
///         network_out(my_fn::EmbeddedNetworkOut as net_out) { responses: Bytes }
///         build(some_config: usize) {
///             my_module::my_fn(my_id, memberships, events, outputs, net_in, net_out)
///         }
///     }
/// }
///
/// let flow = MyFlow::new(member_id, 42);
/// flow.events().push(event);
/// flow.run_with(
///     &mut |result| println!("{result:?}"),
///     &mut |response| network.send(response),
/// );
/// ```
///
/// Item types must match the generated function's parameter types exactly: `network_in`
/// items are `Result<BytesMut, io::Error>` (tagged: `Result<(TaglessMemberId, BytesMut),
/// io::Error>`) unless the channel uses external serialization, `network_out` items are
/// `Bytes` (tagged: `(TaglessMemberId, Bytes)`), and membership items are always
/// `(TaglessMemberId, MembershipEvent)`.
///
/// # Caveats
///
/// - The holder is single-threaded (`!Send`); construct and run it on one thread.
/// - `run` / `run_with` panic if called re-entrantly from within an output callback
///   (output callbacks *may* push new inputs, which are processed later in the same run).
/// - Entry names are used as field and accessor names, so they must be distinct and must
///   not be named `flow` or `run`; `build` parameters must not be named `self_id`,
///   `membership`, `inputs`, `network_in`, `outputs`, or `network_out`.
/// - The fields of the generated struct are self-referential; code in the defining module
///   must never mutate or replace them (use the generated accessors instead).
#[macro_export]
macro_rules! embedded_flow {
    (
        $(#[$struct_meta:meta])*
        $vis:vis struct $name:ident {
            $( self_id($self_ref:ident : $self_ty:ty); )?
            $( membership($mem_path:path as $membership_ref:ident) {
                $($mem_name:ident),* $(,)?
            } )?
            $( inputs { $($in_name:ident : $in_ty:ty),* $(,)? } )?
            $( network_in($nin_path:path as $network_in_ref:ident) {
                $($nin_name:ident : $nin_ty:ty),* $(,)?
            } )?
            $( outputs($outputs_path:path as $outputs_ref:ident) {
                $($out_name:ident : $out_ty:ty),* $(,)?
            } )?
            $( network_out($nout_path:path as $network_out_ref:ident) {
                $($nout_name:ident : $nout_ty:ty),* $(,)?
            } )?
            build($($param:ident : $param_ty:ty),* $(,)?) $build_body:block
        }
    ) => {
        $(#[$struct_meta])*
        $vis struct $name {
            /// The DFIR flow. Declared first so it is dropped first: it borrows from all
            /// of the fields below. Wrapped in a `RefCell` so the flow can be run through
            /// a shared `&self` borrow, letting output callbacks re-enter the holder to
            /// push more input during the same run.
            flow: $crate::embedded::__private::RefCell<
                $crate::embedded::__private::Box<dyn $crate::embedded::DfirRunnable>,
            >,
            /// Type-erased allocations lent to the flow (outputs / network-out structs and
            /// the self id); dropped after the flow.
            _owned: $crate::embedded::__private::Vec<$crate::embedded::OwnedErasedBox>,
            $($(
                $in_name: $crate::embedded::AliasedBox<$crate::embedded::InputBuffer<$in_ty>>,
            )*)?
            $($(
                $nin_name: $crate::embedded::AliasedBox<$crate::embedded::InputBuffer<$nin_ty>>,
            )*)?
            $($(
                $mem_name: $crate::embedded::AliasedBox<$crate::embedded::InputBuffer<(
                    $crate::location::member_id::TaglessMemberId,
                    $crate::location::MembershipEvent,
                )>>,
            )*)?
            $($(
                $out_name: $crate::embedded::AliasedBox<$crate::embedded::CallbackSlot<$out_ty>>,
            )*)?
            $($(
                $nout_name: $crate::embedded::AliasedBox<$crate::embedded::CallbackSlot<$nout_ty>>,
            )*)?
        }

        impl ::core::fmt::Debug for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.debug_struct(::core::stringify!($name)).finish_non_exhaustive()
            }
        }

        impl $name {
            /// Constructs the flow along with the buffers and slots it reads from and
            /// writes to. No ticks are run yet; call
            #[doc = ::core::concat!("[`run`](", ::core::stringify!($name), "::run) or [`run_with`](", ::core::stringify!($name), "::run_with) to process queued inputs.")]
            #[allow(
                unused_unsafe,
                unused_mut,
                reason = "usage depends on which macro sections are present"
            )]
            $vis fn new($( self_id: $self_ty, )? $($param: $param_ty),*) -> Self {
                $($(
                    let $in_name = $crate::embedded::AliasedBox::new(
                        $crate::embedded::InputBuffer::<$in_ty>::new(),
                    );
                )*)?
                $($(
                    let $nin_name = $crate::embedded::AliasedBox::new(
                        $crate::embedded::InputBuffer::<$nin_ty>::new(),
                    );
                )*)?
                $($(
                    let $mem_name = $crate::embedded::AliasedBox::new(
                        $crate::embedded::InputBuffer::<(
                            $crate::location::member_id::TaglessMemberId,
                            $crate::location::MembershipEvent,
                        )>::new(),
                    );
                )*)?
                $($(
                    let $out_name = $crate::embedded::AliasedBox::new(
                        $crate::embedded::CallbackSlot::<$out_ty>::new(),
                    );
                )*)?
                $($(
                    let $nout_name = $crate::embedded::AliasedBox::new(
                        $crate::embedded::CallbackSlot::<$nout_ty>::new(),
                    );
                )*)?

                let mut __owned: $crate::embedded::__private::Vec<$crate::embedded::OwnedErasedBox> =
                    $crate::embedded::__private::Vec::new();

                // SAFETY: the buffers and slots are heap allocations owned by the
                // `AliasedBox` fields of the returned struct (which, unlike `Box`, may be
                // moved while the flow holds pointers into their contents), and the
                // outputs / network-out structs and self id are owned by `_owned`. Field
                // declaration order guarantees the flow is dropped before all of them.
                // Everything stays on a single thread: none of these types are `Send`.
                let flow = unsafe {
                    $(
                        let (__self_id_owner, __self_id_mut) =
                            $crate::embedded::OwnedErasedBox::new(self_id);
                        __owned.push(__self_id_owner);
                        let $self_ref: &'static $self_ty = &*__self_id_mut;
                    )?
                    $($(
                        let $in_name = $crate::embedded::InputBuffer::stream($in_name.as_pin());
                    )*)?
                    $(
                        $(
                            let $mem_name =
                                $crate::embedded::InputBuffer::stream($mem_name.as_pin());
                        )*
                        use $mem_path as __EmbeddedMembership;
                        let $membership_ref = __EmbeddedMembership { $($mem_name),* };
                    )?
                    $(
                        $(
                            let $nin_name =
                                $crate::embedded::InputBuffer::stream($nin_name.as_pin());
                        )*
                        use $nin_path as __EmbeddedNetworkIn;
                        let $network_in_ref = __EmbeddedNetworkIn { $($nin_name),* };
                    )?
                    $(
                        $(
                            let $out_name = $crate::embedded::CallbackSlot::sink($out_name.as_pin());
                        )*
                        use $outputs_path as __EmbeddedOutputs;
                        let (__outputs_owner, $outputs_ref) =
                            $crate::embedded::OwnedErasedBox::new(
                                __EmbeddedOutputs { $($out_name),* },
                            );
                        __owned.push(__outputs_owner);
                    )?
                    $(
                        $(
                            let $nout_name =
                                $crate::embedded::CallbackSlot::sink($nout_name.as_pin());
                        )*
                        use $nout_path as __EmbeddedNetworkOut;
                        let (__network_out_owner, $network_out_ref) =
                            $crate::embedded::OwnedErasedBox::new(
                                __EmbeddedNetworkOut { $($nout_name),* },
                            );
                        __owned.push(__network_out_owner);
                    )?

                    let __dfir = $build_body;
                    let __flow: $crate::embedded::__private::Box<dyn $crate::embedded::DfirRunnable> =
                        $crate::embedded::__private::Box::new(__dfir);
                    __flow
                };

                Self {
                    flow: $crate::embedded::__private::RefCell::new(flow),
                    _owned: __owned,
                    $($( $in_name, )*)?
                    $($( $nin_name, )*)?
                    $($( $mem_name, )*)?
                    $($( $out_name, )*)?
                    $($( $nout_name, )*)?
                }
            }

            /// Runs the flow until no more work is available.
            ///
            /// Items emitted to outputs with no callback installed are dropped; use
            #[doc = ::core::concat!("[`run_with`](", ::core::stringify!($name), "::run_with) (or `CallbackSlot::invoke_with`) to capture outputs.")]
            ///
            /// # Panics
            ///
            /// Panics if called re-entrantly from within an output callback.
            #[inline]
            $vis fn run(&self) {
                self.flow.borrow_mut().run_available_sync();
            }

            /// Runs the flow with the given callbacks installed: one per `outputs` entry,
            /// then one per `network_out` entry, in declaration order.
            ///
            /// # Panics
            ///
            /// Panics if called re-entrantly from within an output callback.
            $vis fn run_with(
                &self
                $($(, $out_name: &mut dyn ::core::ops::FnMut($out_ty))*)?
                $($(, $nout_name: &mut dyn ::core::ops::FnMut($nout_ty))*)?
            ) {
                $crate::__embedded_invoke_nested!(
                    (
                        $($( self.$out_name => $out_name, )*)?
                        $($( self.$nout_name => $nout_name, )*)?
                    ),
                    self.run()
                )
            }

            $($(
                /// Input buffer: push items here, then call
                #[doc = ::core::concat!("[`run_with`](", ::core::stringify!($name), "::run_with) to process them.")]
                $vis fn $in_name(&self) -> &$crate::embedded::InputBuffer<$in_ty> {
                    &self.$in_name
                }
            )*)?
            $($(
                /// Network-in buffer: push received network messages here, then call
                #[doc = ::core::concat!("[`run_with`](", ::core::stringify!($name), "::run_with) to process them.")]
                $vis fn $nin_name(&self) -> &$crate::embedded::InputBuffer<$nin_ty> {
                    &self.$nin_name
                }
            )*)?
            $($(
                /// Cluster membership buffer: push membership events here, then call
                #[doc = ::core::concat!("[`run_with`](", ::core::stringify!($name), "::run_with) to process them.")]
                $vis fn $mem_name(
                    &self,
                ) -> &$crate::embedded::InputBuffer<(
                    $crate::location::member_id::TaglessMemberId,
                    $crate::location::MembershipEvent,
                )> {
                    &self.$mem_name
                }
            )*)?
            $($(
                /// Output slot: install a callback via `invoke_with` around a run, or use
                #[doc = ::core::concat!("[`run_with`](", ::core::stringify!($name), "::run_with).")]
                $vis fn $out_name(&self) -> &$crate::embedded::CallbackSlot<$out_ty> {
                    &self.$out_name
                }
            )*)?
            $($(
                /// Network-out slot: install a callback via `invoke_with` around a run, or
                #[doc = ::core::concat!("use [`run_with`](", ::core::stringify!($name), "::run_with).")]
                $vis fn $nout_name(&self) -> &$crate::embedded::CallbackSlot<$nout_ty> {
                    &self.$nout_name
                }
            )*)?
        }
    };
}

#[cfg(test)]
mod tests {
    use core::sync::atomic::{AtomicUsize, Ordering};
    use core::task::Context;
    use std::rc::Rc;
    use std::sync::Arc;
    use std::task::Wake;

    use futures::Stream;

    use super::*;

    /// Waker that counts how many times it has been woken.
    struct CountingWaker(AtomicUsize);

    impl Wake for CountingWaker {
        fn wake(self: Arc<Self>) {
            self.wake_by_ref();
        }

        fn wake_by_ref(self: &Arc<Self>) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn poll_next<T>(stream: &mut InputStream<T>, cx: &mut Context<'_>) -> Poll<Option<T>> {
        Pin::new(stream).poll_next(cx)
    }

    #[test]
    fn input_buffer_delivers_in_order_and_wakes() {
        let buffer = AliasedBox::new(InputBuffer::new());
        // SAFETY: `buffer` outlives `stream`.
        let mut stream = unsafe { InputBuffer::stream(buffer.as_pin()) };

        let wake_count = Arc::new(CountingWaker(AtomicUsize::new(0)));
        let waker = Waker::from(Arc::clone(&wake_count));
        let mut cx = Context::from_waker(&waker);

        // Empty: pending, waker registered.
        assert_eq!(poll_next(&mut stream, &mut cx), Poll::Pending);
        assert_eq!(wake_count.0.load(Ordering::Relaxed), 0);

        // Push wakes exactly once (coalesced), even for multiple pushes.
        buffer.push(1);
        buffer.push(2);
        assert_eq!(wake_count.0.load(Ordering::Relaxed), 1);
        assert_eq!(buffer.len(), 2);
        assert_eq!(stream.size_hint(), (2, None));

        // FIFO delivery.
        assert_eq!(poll_next(&mut stream, &mut cx), Poll::Ready(Some(1)));
        assert_eq!(poll_next(&mut stream, &mut cx), Poll::Ready(Some(2)));
        assert_eq!(poll_next(&mut stream, &mut cx), Poll::Pending);
        assert!(buffer.is_empty());

        // Waker was re-registered after going pending again.
        buffer.push(3);
        assert_eq!(wake_count.0.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn input_buffer_stable_across_moves() {
        // Moving the `AliasedBox` (e.g. into a struct) must not invalidate the stream.
        let buffer = AliasedBox::new(InputBuffer::new());
        // SAFETY: `buffer` outlives `stream` (both live to the end of this function).
        let mut stream = unsafe { InputBuffer::stream(buffer.as_pin()) };

        let moved = [buffer];
        moved[0].push("hello");

        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);
        assert_eq!(poll_next(&mut stream, &mut cx), Poll::Ready(Some("hello")));
    }

    #[test]
    fn callback_slot_invoke_with_installs_and_restores() {
        let slot: CallbackSlot<u32> = CallbackSlot::new();

        // No callback installed: items are dropped.
        slot.invoke(1);

        let mut seen = Vec::new();
        slot.invoke_with(&mut |x| seen.push(x), || {
            slot.invoke(2);
            slot.invoke(3);
        });
        assert_eq!(seen, vec![2, 3]);

        // Uninstalled again after `invoke_with` returns.
        slot.invoke(4);
        assert_eq!(seen, vec![2, 3]);
    }

    #[test]
    fn callback_slot_nested_invoke_with_restores_outer() {
        let slot: CallbackSlot<u32> = CallbackSlot::new();
        let mut outer = Vec::new();
        let mut inner = Vec::new();

        slot.invoke_with(&mut |x| outer.push(x), || {
            slot.invoke(1);
            slot.invoke_with(&mut |x| inner.push(x), || slot.invoke(2));
            // The outer callback must be restored after the nested installation.
            slot.invoke(3);
        });

        assert_eq!(outer, vec![1, 3]);
        assert_eq!(inner, vec![2]);
    }

    #[test]
    fn callback_slot_reentrant_invoke_drops_item() {
        let slot: Rc<CallbackSlot<u32>> = Rc::new(CallbackSlot::new());
        let slot2 = Rc::clone(&slot);

        let mut seen = Vec::new();
        let mut callback = |x: u32| {
            seen.push(x);
            // Re-entrant invocation while the callback is running: the callback has been
            // taken out of the slot, so this must be a no-op rather than aliasing UB.
            if x == 1 {
                slot2.invoke(99);
            }
        };
        slot.invoke_with(&mut callback, || {
            slot.invoke(1);
            slot.invoke(2);
        });
        assert_eq!(seen, vec![1, 2]);
    }

    #[test]
    fn callback_slot_uninstalls_on_panic() {
        let slot: Rc<CallbackSlot<u32>> = Rc::new(CallbackSlot::new());
        let slot2 = Rc::clone(&slot);

        let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            slot.invoke_with(&mut |_| {}, || panic!("boom"));
        }));
        assert!(panicked.is_err());

        // The (now-dead) callback must not still be installed after the unwind.
        let mut seen = Vec::new();
        slot2.invoke(5); // dropped, not UB
        slot2.invoke_with(&mut |x| seen.push(x), || slot2.invoke(6));
        assert_eq!(seen, vec![6]);
    }

    #[test]
    fn owned_erased_box_drops_contents() {
        struct NoteDrop(Rc<Cell<bool>>);
        impl Drop for NoteDrop {
            fn drop(&mut self) {
                self.0.set(true);
            }
        }

        let dropped = Rc::new(Cell::new(false));
        // SAFETY: `reference` is not used after `erased` is dropped.
        let (erased, reference) = unsafe { OwnedErasedBox::new(NoteDrop(Rc::clone(&dropped))) };
        assert!(!reference.0.get());

        // Move the erased box around; the allocation must remain valid.
        let moved = [erased];
        assert!(!dropped.get());
        drop(moved);
        assert!(dropped.get());
    }

    #[test]
    fn aliased_box_derefs_and_drops() {
        struct NoteDrop(Rc<Cell<bool>>);
        impl Drop for NoteDrop {
            fn drop(&mut self) {
                self.0.set(true);
            }
        }

        let dropped = Rc::new(Cell::new(false));
        let boxed = AliasedBox::new(NoteDrop(Rc::clone(&dropped)));
        assert!(!boxed.0.get());
        drop(boxed);
        assert!(dropped.get());
    }
}
