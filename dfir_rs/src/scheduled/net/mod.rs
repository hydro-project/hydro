//! This module previously contained networking code.
//!
//! ## How Tokio interacts with the DFIR runtime (Mingwei 2021-12-07)
//!
//! [Tokio](https://tokio.rs/) is a Rust async runtime. In Rust's async/await
//! system, `Future`s must be spawned or sent to an async runtime in order to
//! run. Tokio is the most popular provider of one of these runtimes, with
//! [async-std](https://async.rs/) (mirrors std lib) and [smol](https://github.com/smol-rs/smol)
//! (minimal runtime) as commonly used alternatives.
//!
//! Fundamentally, an async runtime's job is to poll futures (read: run tasks)
//! when they are ready to make progress. However async runtimes also provide a
//! `Future`s abstraction for async events such as timers, network IO, and
//! filesystem IO. To do this, [Tokio](https://tokio.rs/) uses [Mio](https://github.com/tokio-rs/mio)
//! which is a low-level non-blocking API for IO event notification/polling.
//! A user of Mio can write an event loop, i.e. something like: wait for
//! events, run computations responding to those events, repeat. Tokio provides
//! the higher-level async/await slash `Future` abstraction on top of Mio, as
//! well as the runtime to execute those `Future`s. Essentially, the Tokio
//! async runtime essentially replaces the low-level event loop a user might
//! handwrite when using Mio.
//!
//! For context, both Mio and Tokio provide socket/UDP/TCP-level network
//! abstractions, which is probably the right layer for us. There are also
//! libraries built on top of Tokio providing nice server/client HTTP APIs
//! like [Hyper](https://hyper.rs/).
//!
//! The DFIR scheduled layer scheduler is essentially the same as a simple
//! event loop: it runs subgraphs when they have data. We have also let it
//! respond to external asynchonous events by providing a threadsafe channel
//! through which subgraphs can be externally scheduled.
//!
//! In order to add networking to DFIR, in our current implementation we
//! use Tokio and have a compatibility mechanism for working with `Future`s.
//! A `Future` provides a `Waker` mechanism to notify when it had work to do,
//! so we have hooked these Wakers up with DFIR's threadsafe external
//! scheduling channel. This essentially turns DFIR into a simple async
//! runtime.
//!
//! However in some situations, we still need to run futures outside of
//! DFIR's basic runtime. It's not a goal for DFIR to provide all
//! the features of a full runtime like Tokio. Currently for this situation we
//! run DFIR as a task (`Future`) within the Tokio runtime. In DFIR's
//! event loop we do all available work, then rather than block and wait for
//! external events to schedule more tasks, we temporarily yield back to the
//! Tokio runtime. Tokio will then respond to any outstanding events it has
//! before once again running the DFIR scheduler task.
//!
//! This works perfectly well but maybe isn't the best solution long-term.
//! In the future we may want to remove the extra Tokio runtime layer and
//! interface with Mio directly. In this case we would have to do our own
//! socket-style polling within the DFIR scheduler's event loop, which
//! would require some extra work and thought. But for now interfacing with
//! Tokio works and I don't think the overhead of the extra runtime loop is
//! significant.
