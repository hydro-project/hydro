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
//! is the dylib's own and catches the panic. They hand its message to the host's
//! [`PanicHandler`], which raises it again as an ordinary host panic with a `String` payload.
//! That panic unwinds out of the handler, through the guard's frame (no longer catching), and
//! into the host like any other.

use std::any::Any;
use std::future::Future;
use std::panic::AssertUnwindSafe;

use dfir_rs::scheduled::context::Context;

/// The host function the dylib reports a program panic to, with its message. It raises the
/// panic on the host, so it never returns.
pub type PanicHandler = fn(String) -> !;

/// The message of a caught panic payload, as `panic!` formats it.
fn panic_message(payload: Box<dyn Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_owned()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "simulated program panicked with a non-string payload".to_owned()
    }
    // `payload` is dropped here, by the code that allocated it.
}

/// Runs `f`, reporting a panic to `handler` instead of letting it unwind out of the dylib.
fn guarded<R>(handler: PanicHandler, f: impl FnOnce() -> R) -> R {
    match std::panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(r) => r,
        Err(payload) => handler(panic_message(payload)),
    }
}

/// Wraps a DFIR tick closure so a panic raised while polling it is reported to `handler`.
/// Called from generated code, through
/// [`Dfir::map_tick_closure`](dfir_rs::scheduled::context::Dfir::map_tick_closure).
pub fn guard_tick<F>(
    mut tick: F,
    handler: PanicHandler,
) -> impl 'static + for<'a> AsyncFnMut(&'a mut Context) -> bool
where
    F: 'static + for<'a> AsyncFnMut(&'a mut Context) -> bool,
{
    async move |ctx: &mut Context| -> bool {
        let mut fut = std::pin::pin!(tick(ctx));
        std::future::poll_fn(move |cx| guarded(handler, || fut.as_mut().poll(cx))).await
    }
}

/// Runs `f` (the dylib's instantiation of the program), reporting a panic to `handler`.
pub fn guard_runtime<R>(handler: PanicHandler, f: impl FnOnce() -> R) -> R {
    guarded(handler, f)
}

/// The host's [`PanicHandler`].
pub(crate) fn host_handler(message: String) -> ! {
    std::panic::resume_unwind(Box::new(message))
}
