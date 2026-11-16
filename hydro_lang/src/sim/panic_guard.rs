//! Carrying program panics out of the simulation dylib.
//!
//! A compiled simulation is a `cdylib` with its own copy of the standard library, so a panic
//! raised by program code inside it (say, an `unwrap` in a `map` closure) is a *foreign*
//! exception to the host. Unwinding it into a host `catch_unwind` (bolero's, or a test's) aborts
//! the whole process with "Rust cannot catch foreign exceptions", so `#[should_panic]` and the
//! fuzzer never see it.
//!
//! Generated code therefore wraps every poll of every DFIR tick in [`guard_tick`], and the
//! program's instantiation in [`guard_runtime`]. Both run in the dylib, so their `catch_unwind`
//! is the dylib's own and catches the panic. They lend its message to the host's
//! [`PanicHandler`], which copies it and raises it again as an ordinary host panic with a
//! `String` payload. That panic unwinds out of the handler, through the guard's frame (no longer
//! catching, and dropping the dylib's copy of the message), and into the host like any other.
//!
//! Only the message crosses the boundary, as a borrowed `&str`, never the caught
//! `Box<dyn Any + Send>` itself. That box's vtable (`type_id`, `drop_in_place`) lives in the
//! dylib's image, so the host must not hold it past the dylib's lifetime, and a panic payload
//! routinely does: in a `#[should_panic]` test, unwinding drops the `CompiledSim` (and with it
//! the `libloading::Library`) before the test harness downcasts the payload to check `expected`.
//! Nothing is lost by converting in the dylib: `&str` and `String` are the only payload types the
//! host could downcast anyway (`String` is the same sysroot `alloc` type on both sides), while a
//! type from any other crate is compiled separately for the dylib and so has a different `TypeId`
//! than the host's.
//!
//! The message is lent rather than given (`&str`, not `String`) so that no heap allocation
//! changes owner across the boundary: the host and the dylib each have their own global
//! allocator, and freeing one side's allocation with the other's is only sound while both happen
//! to be the default `System` allocator (see hydro-project/hydro#3379). Like the
//! `println_handler: fn(fmt::Arguments<'_>)`, the host copies what it needs while the dylib's
//! frame is still live.

use std::any::Any;
use std::future::Future;
use std::panic::AssertUnwindSafe;

use dfir_rs::scheduled::context::{Context, TickClosure, TickClosureFn};

/// The host function the dylib reports a program panic to, lending its message. It raises the
/// panic on the host, so it never returns.
pub type PanicHandler = fn(&str) -> !;

/// The message of a caught panic payload, as `panic!` formats it.
///
/// Runs in the dylib. The payload must be consumed here rather than handed to the host (see the
/// module docs).
fn panic_message(payload: Box<dyn Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_owned()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "simulated program panicked with a non-string payload (see the `panicked at` message printed above for its location)"
            .to_owned()
    }
    // `payload` is dropped here, by the code that allocated it.
}

/// Runs `f`, reporting a panic to `handler` instead of letting it unwind out of the dylib.
fn guarded<R>(handler: PanicHandler, f: impl FnOnce() -> R) -> R {
    match std::panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(r) => r,
        Err(payload) => {
            let message = panic_message(payload);
            handler(&message)
            // `handler` never returns; the host's panic unwinds back through this frame, whose
            // landing pad drops `message` with the dylib's allocator.
        }
    }
}

/// Wraps a DFIR tick closure so a panic raised while polling it is reported to `handler`.
/// Called from generated code, through
/// [`Dfir::map_tick_closure`](dfir_rs::scheduled::context::Dfir::map_tick_closure).
pub fn guard_tick<Tick>(
    mut tick: Tick,
    handler: PanicHandler,
) -> impl TickClosure<Metrics = Tick::Metrics> + 'static
where
    Tick: TickClosure + 'static,
{
    TickClosureFn::new(async move |ctx: &mut Context<Tick::Metrics>| -> bool {
        let mut fut = std::pin::pin!(tick.call_tick(ctx));
        std::future::poll_fn(move |cx| guarded(handler, || fut.as_mut().poll(cx))).await
    })
}

/// Runs `f` (the dylib's instantiation of the program), reporting a panic to `handler`.
pub fn guard_runtime<R>(handler: PanicHandler, f: impl FnOnce() -> R) -> R {
    guarded(handler, f)
}

/// The host's [`PanicHandler`]. Copies the message into a host-owned `String` before raising.
pub(crate) fn host_handler(message: &str) -> ! {
    std::panic::resume_unwind(Box::new(message.to_owned()))
}
