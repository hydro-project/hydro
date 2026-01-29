//! Interfaces for compiled Hydro simulators and concrete simulation instances.

use core::{fmt, panic};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::fmt::Debug;
use std::panic::RefUnwindSafe;
use std::path::Path;
use std::pin::{Pin, pin};
use std::rc::Rc;
use std::task::ready;

use bytes::Bytes;
use colored::Colorize;
use dfir_rs::scheduled::graph::Dfir;
use futures::{Stream, StreamExt};
use libloading::Library;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tempfile::TempPath;
use tokio::sync::Mutex;
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::wrappers::UnboundedReceiverStream;

use super::runtime::{Hooks, InlineHooks};
use super::{SimReceiver, SimSender};
use crate::compile::builder::ExternalPortId;
use crate::live_collections::stream::{ExactlyOnce, NoOrder, Ordering, Retries, TotalOrder};
use crate::location::dynamic::LocationId;
use crate::sim::graph::{SimExternalPort, SimExternalPortRegistry};

struct SimConnections {
    input_senders: HashMap<SimExternalPort, Rc<UnboundedSender<Bytes>>>,
    output_receivers: HashMap<SimExternalPort, Rc<Mutex<UnboundedReceiverStream<Bytes>>>>,
    external_registered: HashMap<ExternalPortId, SimExternalPort>,
}

tokio::task_local! {
    static CURRENT_SIM_CONNECTIONS: RefCell<SimConnections>;
}

/// A handle to a compiled Hydro simulation, which can be instantiated and run.
pub struct CompiledSim {
    pub(super) _path: TempPath,
    pub(super) lib: Library,
    pub(super) externals_port_registry: SimExternalPortRegistry,
}

#[sealed::sealed]
/// A trait implemented by closures that can instantiate a compiled simulation.
///
/// This is needed to ensure [`RefUnwindSafe`] so instances can be created during fuzzing.
pub trait Instantiator<'a>: RefUnwindSafe + Fn() -> CompiledSimInstance<'a> {}
#[sealed::sealed]
impl<'a, T: RefUnwindSafe + Fn() -> CompiledSimInstance<'a>> Instantiator<'a> for T {}

fn null_handler(_args: fmt::Arguments) {}

fn println_handler(args: fmt::Arguments) {
    println!("{}", args);
}

fn eprintln_handler(args: fmt::Arguments) {
    eprintln!("{}", args);
}

/// Creates a simulation instance, returning:
/// - A list of async DFIRs to run (all process / cluster logic outside a tick)
/// - A list of tick DFIRs to run (where the &'static str is for the tick location id)
/// - A mapping of hooks for non-deterministic decisions at tick-input boundaries
/// - A mapping of inline hooks for non-deterministic decisions inside ticks
type SimLoaded<'a> = libloading::Symbol<
    'a,
    unsafe extern "Rust" fn(
        should_color: bool,
        // usize: SimExternalPort
        external_out: HashMap<usize, UnboundedSender<Bytes>>,
        // usize: SimExternalPort
        external_in: HashMap<usize, UnboundedReceiverStream<Bytes>>,
        println_handler: fn(fmt::Arguments<'_>),
        eprintln_handler: fn(fmt::Arguments<'_>),
    ) -> (
        Vec<(&'static str, Option<u32>, Dfir<'static>)>,
        Vec<(&'static str, Option<u32>, Dfir<'static>)>,
        Hooks<&'static str>,
        InlineHooks<&'static str>,
    ),
>;

impl CompiledSim {
    /// Executes the given closure with a single instance of the compiled simulation.
    pub fn with_instance<T>(&self, thunk: impl FnOnce(CompiledSimInstance) -> T) -> T {
        self.with_instantiator(|instantiator| thunk(instantiator()), true)
    }

    /// Executes the given closure with an [`Instantiator`], which can be called to create
    /// independent instances of the simulation. This is useful for fuzzing, where we need to
    /// re-execute the simulation several times with different decisions.
    ///
    /// The `always_log` parameter controls whether to log tick executions and stream releases. If
    /// it is `true`, logging will always be enabled. If it is `false`, logging will only be
    /// enabled if the `HYDRO_SIM_LOG` environment variable is set to `1`.
    pub fn with_instantiator<T>(
        &self,
        thunk: impl FnOnce(&dyn Instantiator) -> T,
        always_log: bool,
    ) -> T {
        let func: SimLoaded = unsafe { self.lib.get(b"__hydro_runtime").unwrap() };
        let log = always_log || std::env::var("HYDRO_SIM_LOG").is_ok_and(|v| v == "1");
        thunk(
            &(|| CompiledSimInstance {
                func: func.clone(),
                externals_port_registry: self.externals_port_registry.clone(),
                input_ports: HashMap::new(),
                output_ports: HashMap::new(),
                log,
            }),
        )
    }

    /// Uses a fuzzing strategy to explore possible executions of the simulation. The provided
    /// closure will be repeatedly executed with instances of the Hydro program where the
    /// batching boundaries, order of messages, and retries are varied.
    ///
    /// During development, you should run the test that invokes this function with the `cargo sim`
    /// command, which will use `libfuzzer` to intelligently explore the execution space. If a
    /// failure is found, a minimized test case will be produced in a `sim-failures` directory.
    /// When running the test with `cargo test` (such as in CI), if a reproducer is found it will
    /// be executed, and if no reproducer is found a small number of random executions will be
    /// performed.
    pub fn fuzz(&self, mut thunk: impl AsyncFn() + RefUnwindSafe) {
        let caller_fn = crate::compile::ir::backtrace::Backtrace::get_backtrace(0)
            .elements()
            .into_iter()
            .find(|e| {
                !e.fn_name.starts_with("hydro_lang::sim::compiled")
                    && !e.fn_name.starts_with("hydro_lang::sim::flow")
                    && !e.fn_name.starts_with("fuzz<")
                    && !e.fn_name.starts_with("<hydro_lang::sim")
            })
            .unwrap();

        let caller_path = Path::new(&caller_fn.filename.unwrap()).to_path_buf();
        let repro_folder = caller_path.parent().unwrap().join("sim-failures");

        let caller_fuzz_repro_path = repro_folder
            .join(caller_fn.fn_name.replace("::", "__"))
            .with_extension("bin");

        if std::env::var("BOLERO_FUZZER").is_ok() {
            let corpus_dir = std::env::current_dir().unwrap().join(".fuzz-corpus");
            std::fs::create_dir_all(&corpus_dir).unwrap();
            let libfuzzer_args = format!(
                "{} {} -artifact_prefix={}/ -handle_abrt=0",
                corpus_dir.to_str().unwrap(),
                corpus_dir.to_str().unwrap(),
                corpus_dir.to_str().unwrap(),
            );

            std::fs::create_dir_all(&repro_folder).unwrap();

            if !std::env::var("HYDRO_NO_FAILURE_OUTPUT").is_ok_and(|v| v == "1") {
                unsafe {
                    std::env::set_var(
                        "BOLERO_FAILURE_OUTPUT",
                        caller_fuzz_repro_path.to_str().unwrap(),
                    );
                }
            }

            unsafe {
                std::env::set_var("BOLERO_LIBFUZZER_ARGS", libfuzzer_args);
            }

            self.with_instantiator(
                |instantiator| {
                    bolero::test(bolero::TargetLocation {
                        package_name: "",
                        manifest_dir: "",
                        module_path: "",
                        file: "",
                        line: 0,
                        item_path: "<unknown>::__bolero_item_path__",
                        test_name: None,
                    })
                    .run_with_replay(move |is_replay| {
                        let mut instance = instantiator();

                        if instance.log {
                            eprintln!(
                                "{}",
                                "\n==== New Simulation Instance ===="
                                    .color(colored::Color::Cyan)
                                    .bold()
                            );
                        }

                        if is_replay {
                            instance.log = true;
                        }

                        tokio::runtime::Builder::new_current_thread()
                            .build()
                            .unwrap()
                            .block_on(async { instance.run(&mut thunk).await })
                    })
                },
                false,
            );
        } else if let Ok(existing_bytes) = std::fs::read(&caller_fuzz_repro_path) {
            self.fuzz_repro(existing_bytes, async |compiled| {
                compiled.launch();
                thunk().await
            });
        } else {
            eprintln!(
                "Running a fuzz test without `cargo sim` and no reproducer found at {}, defaulting to 8192 iterations with random inputs.",
                caller_fuzz_repro_path.display()
            );
            self.with_instantiator(
                |instantiator| {
                    bolero::test(bolero::TargetLocation {
                        package_name: "",
                        manifest_dir: "",
                        module_path: "",
                        file: ".",
                        line: 0,
                        item_path: "<unknown>::__bolero_item_path__",
                        test_name: None,
                    })
                    .with_iterations(8192)
                    .run(move || {
                        let instance = instantiator();
                        tokio::runtime::Builder::new_current_thread()
                            .build()
                            .unwrap()
                            .block_on(async { instance.run(&mut thunk).await })
                    })
                },
                false,
            );
        }
    }

    /// Executes the given closure with a single instance of the compiled simulation, using the
    /// provided bytes as the source of fuzzing decisions. This can be used to manually reproduce a
    /// failure found during fuzzing.
    pub fn fuzz_repro<'a>(
        &'a self,
        bytes: Vec<u8>,
        thunk: impl AsyncFnOnce(CompiledSimInstance) + RefUnwindSafe,
    ) {
        self.with_instance(|instance| {
            bolero::bolero_engine::any::scope::with(
                Box::new(bolero::bolero_engine::driver::object::Object(
                    bolero::bolero_engine::driver::bytes::Driver::new(bytes, &Default::default()),
                )),
                || {
                    tokio::runtime::Builder::new_current_thread()
                        .build()
                        .unwrap()
                        .block_on(async { instance.run_without_launching(thunk).await })
                },
            )
        });
    }

    /// Exhaustively searches all possible executions of the simulation. The provided
    /// closure will be repeatedly executed with instances of the Hydro program where the
    /// batching boundaries, order of messages, and retries are varied.
    ///
    /// Exhaustive searching is feasible when the inputs to the Hydro program are finite and there
    /// are no dataflow loops that generate infinite messages. Exhaustive searching provides a
    /// stronger guarantee of correctness than fuzzing, but may take a long time to complete.
    /// Because no fuzzer is involved, you can run exhaustive tests with `cargo test`.
    ///
    /// Returns the number of distinct executions explored.
    pub fn exhaustive(&self, mut thunk: impl AsyncFnMut() + RefUnwindSafe) -> usize {
        if std::env::var("BOLERO_FUZZER").is_ok() {
            eprintln!(
                "Cannot run exhaustive tests with a fuzzer. Please use `cargo test` instead of `cargo sim`."
            );
            std::process::abort();
        }

        let mut count = 0;
        let count_mut = &mut count;

        self.with_instantiator(
            |instantiator| {
                bolero::test(bolero::TargetLocation {
                    package_name: "",
                    manifest_dir: "",
                    module_path: "",
                    file: "",
                    line: 0,
                    item_path: "<unknown>::__bolero_item_path__",
                    test_name: None,
                })
                .exhaustive()
                .run_with_replay(move |is_replay| {
                    *count_mut += 1;

                    let mut instance = instantiator();
                    if instance.log {
                        eprintln!(
                            "{}",
                            "\n==== New Simulation Instance ===="
                                .color(colored::Color::Cyan)
                                .bold()
                        );
                    }

                    if is_replay {
                        instance.log = true;
                    }

                    tokio::runtime::Builder::new_current_thread()
                        .build()
                        .unwrap()
                        .block_on(async { instance.run(&mut thunk).await })
                })
            },
            false,
        );

        count
    }
}

/// A single instance of a compiled Hydro simulation, which provides methods to interactively
/// execute the simulation, feed inputs, and receive outputs.
pub struct CompiledSimInstance<'a> {
    func: SimLoaded<'a>,
    externals_port_registry: SimExternalPortRegistry,
    output_ports: HashMap<SimExternalPort, UnboundedSender<Bytes>>,
    input_ports: HashMap<SimExternalPort, UnboundedReceiverStream<Bytes>>,
    log: bool,
}

impl<'a> CompiledSimInstance<'a> {
    async fn run(self, thunk: impl AsyncFnOnce() + RefUnwindSafe) {
        self.run_without_launching(async |instance| {
            instance.launch();
            thunk().await;
        })
        .await;
    }

    async fn run_without_launching(
        mut self,
        thunk: impl AsyncFnOnce(CompiledSimInstance) + RefUnwindSafe,
    ) {
        let mut input_senders = HashMap::new();
        let mut output_receivers = HashMap::new();
        for registered_port in self.externals_port_registry.port_counter.range_up_to() {
            {
                let (sender, receiver) = dfir_rs::util::unbounded_channel::<Bytes>();
                self.output_ports.insert(registered_port, sender);
                output_receivers.insert(
                    registered_port,
                    Rc::new(Mutex::new(UnboundedReceiverStream::new(
                        receiver.into_inner(),
                    ))),
                );
            }

            {
                let (sender, receiver) = dfir_rs::util::unbounded_channel::<Bytes>();
                self.input_ports.insert(registered_port, receiver);
                input_senders.insert(registered_port, Rc::new(sender));
            }
        }

        let local_set = tokio::task::LocalSet::new();
        local_set
            .run_until(CURRENT_SIM_CONNECTIONS.scope(
                RefCell::new(SimConnections {
                    input_senders,
                    output_receivers,
                    external_registered: self.externals_port_registry.registered.clone(),
                }),
                async move {
                    thunk(self).await;
                },
            ))
            .await;
    }

    /// Launches the simulation, which will asynchronously simulate the Hydro program. This should
    /// be invoked but before receiving any messages.
    fn launch(self) {
        tokio::task::spawn_local(self.schedule_with_maybe_logger::<std::io::Empty>(None));
    }

    /// Returns a future that schedules simulation with the given logger for reporting the
    /// simulation trace.
    pub fn schedule_with_logger<W: std::io::Write>(
        self,
        log_writer: W,
    ) -> impl use<W> + Future<Output = ()> {
        self.schedule_with_maybe_logger(Some(log_writer))
    }

    fn schedule_with_maybe_logger<W: std::io::Write>(
        self,
        log_override: Option<W>,
    ) -> impl use<W> + Future<Output = ()> {
        let (async_dfirs, tick_dfirs, hooks, inline_hooks) = unsafe {
            (self.func)(
                colored::control::SHOULD_COLORIZE.should_colorize(),
                self.output_ports
                    .into_iter()
                    .map(|(k, v)| (k.into_inner(), v))
                    .collect(),
                self.input_ports
                    .into_iter()
                    .map(|(k, v)| (k.into_inner(), v))
                    .collect(),
                if self.log {
                    println_handler
                } else {
                    null_handler
                },
                if self.log {
                    eprintln_handler
                } else {
                    null_handler
                },
            )
        };

        let not_ready_observation = async_dfirs
            .iter()
            .map(|(lid, c_id, _)| (serde_json::from_str(lid).unwrap(), c_id.clone()))
            .collect();

        let mut launched = LaunchedSim {
            async_dfirs: async_dfirs
                .into_iter()
                .map(|(lid, c_id, dfir)| (serde_json::from_str(lid).unwrap(), c_id, dfir))
                .collect(),
            possibly_ready_ticks: vec![],
            not_ready_ticks: tick_dfirs
                .into_iter()
                .map(|(lid, c_id, dfir)| (serde_json::from_str(lid).unwrap(), c_id, dfir))
                .collect(),
            possibly_ready_observation: vec![],
            not_ready_observation,
            hooks: hooks
                .into_iter()
                .map(|((lid, cid), hs)| ((serde_json::from_str(lid).unwrap(), cid), hs))
                .collect(),
            inline_hooks: inline_hooks
                .into_iter()
                .map(|((lid, cid), hs)| ((serde_json::from_str(lid).unwrap(), cid), hs))
                .collect(),
            log: if self.log {
                if let Some(w) = log_override {
                    LogKind::Custom(w)
                } else {
                    LogKind::Stderr
                }
            } else {
                LogKind::Null
            },
        };

        async move { launched.scheduler().await }
    }
}

impl<T: Serialize + DeserializeOwned, O: Ordering, R: Retries> Clone for SimReceiver<T, O, R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Serialize + DeserializeOwned, O: Ordering, R: Retries> Copy for SimReceiver<T, O, R> {}

impl<T: Serialize + DeserializeOwned, O: Ordering, R: Retries> SimReceiver<T, O, R> {
    async fn with_stream<Out>(
        &self,
        thunk: impl AsyncFnOnce(&mut Pin<&mut dyn Stream<Item = T>>) -> Out,
    ) -> Out {
        let receiver = CURRENT_SIM_CONNECTIONS.with(|connections| {
            let connections = &mut *connections.borrow_mut();
            connections
                .output_receivers
                .get(connections.external_registered.get(&self.0).unwrap())
                .unwrap()
                .clone()
        });

        let mut receiver_stream = receiver.lock().await;
        thunk(&mut pin!(
            &mut receiver_stream
                .by_ref()
                .map(|b| bincode::deserialize(&b).unwrap())
        ))
        .await
    }

    /// Asserts that the stream has ended and no more messages can possibly arrive.
    pub fn assert_no_more(self) -> impl Future<Output = ()>
    where
        T: Debug,
    {
        FutureTrackingCaller {
            future: async move {
                self.with_stream(async |stream| {
                    if let Some(next) = stream.next().await {
                        Err(format!(
                            "Stream yielded unexpected message: {:?}, expected termination",
                            next
                        ))?;
                    }
                    Ok(())
                })
                .await
            },
        }
    }
}

impl<T: Serialize + DeserializeOwned> SimReceiver<T, TotalOrder, ExactlyOnce> {
    /// Receives the next message from the external bincode stream. This will wait until a message
    /// is available, or return `None` if no more messages can possibly arrive.
    pub async fn next(&self) -> Option<T> {
        self.with_stream(async |stream| stream.next().await).await
    }

    /// Collects all remaining messages from the external bincode stream into a collection. This
    /// will wait until no more messages can possibly arrive.
    pub async fn collect<C: Default + Extend<T>>(self) -> C {
        self.with_stream(async |stream| stream.collect().await)
            .await
    }

    /// Asserts that the stream yields exactly the expected sequence of messages, in order.
    /// This does not check that the stream ends, use [`Self::assert_yields_only`] for that.
    pub fn assert_yields<T2: Debug, I: IntoIterator<Item = T2>>(
        &self,
        expected: I,
    ) -> impl use<'_, T, T2, I> + Future<Output = ()>
    where
        T: Debug + PartialEq<T2>,
    {
        FutureTrackingCaller {
            future: async {
                let mut expected: VecDeque<T2> = expected.into_iter().collect();

                while !expected.is_empty() {
                    if let Some(next) = self.next().await {
                        let next_expected = expected.pop_front().unwrap();
                        if next != next_expected {
                            Err(format!(
                                "Stream yielded unexpected message: {:?}, expected: {:?}",
                                next, next_expected
                            ))?
                        }
                    } else {
                        Err(format!(
                            "Stream ended early, still expected: {:?}",
                            expected
                        ))?;
                    }
                }

                Ok(())
            },
        }
    }

    /// Asserts that the stream yields only the expected sequence of messages, in order,
    /// and then ends.
    pub fn assert_yields_only<T2: Debug, I: IntoIterator<Item = T2>>(
        &self,
        expected: I,
    ) -> impl use<'_, T, T2, I> + Future<Output = ()>
    where
        T: Debug + PartialEq<T2>,
    {
        ChainedFuture {
            first: self.assert_yields(expected),
            second: self.assert_no_more(),
            first_done: false,
        }
    }
}

pin_project_lite::pin_project! {
    // A future that tracks the location of the `.await` call for better panic messages.
    //
    // `#[track_caller]` is important for us to create assertion methods because it makes
    // the panic backtrace show up at that method (instead of inside the call tree within
    // that method). This is e.g. what `Option::unwrap` uses. Unfortunately, `#[track_caller]`
    // does not work correctly for async methods (or `dyn Future` either), so we have to
    // create these concrete future types that (1) have `#[track_caller]` on their `poll()`
    // method and (2) have the `panic!` triggered in their `poll()` method (or in a directly
    // nested concrete future).
    struct FutureTrackingCaller<F: Future<Output = Result<(), String>>> {
        #[pin]
        future: F,
    }
}

impl<F: Future<Output = Result<(), String>>> Future for FutureTrackingCaller<F> {
    type Output = ();

    #[track_caller]
    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match ready!(self.as_mut().project().future.poll(cx)) {
            Ok(()) => std::task::Poll::Ready(()),
            Err(e) => panic!("{}", e),
        }
    }
}

pin_project_lite::pin_project! {
    // A future that first awaits the first future, then the second, propagating caller info.
    //
    // See [`FutureTrackingCaller`] for context.
    struct ChainedFuture<F1: Future<Output = ()>, F2: Future<Output = ()>> {
        #[pin]
        first: F1,
        #[pin]
        second: F2,
        first_done: bool,
    }
}

impl<F1: Future<Output = ()>, F2: Future<Output = ()>> Future for ChainedFuture<F1, F2> {
    type Output = ();

    #[track_caller]
    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if !self.first_done {
            ready!(self.as_mut().project().first.poll(cx));
            *self.as_mut().project().first_done = true;
        }

        self.as_mut().project().second.poll(cx)
    }
}

impl<T: Serialize + DeserializeOwned> SimReceiver<T, NoOrder, ExactlyOnce> {
    /// Collects all remaining messages from the external bincode stream into a collection,
    /// sorting them. This will wait until no more messages can possibly arrive.
    pub async fn collect_sorted<C: Default + Extend<T> + AsMut<[T]>>(self) -> C
    where
        T: Ord,
    {
        self.with_stream(async |stream| {
            let mut collected: C = stream.collect().await;
            collected.as_mut().sort();
            collected
        })
        .await
    }

    /// Asserts that the stream yields exactly the expected sequence of messages, in some order.
    /// This does not check that the stream ends, use [`Self::assert_yields_only_unordered`] for that.
    pub fn assert_yields_unordered<T2: Debug, I: IntoIterator<Item = T2>>(
        &self,
        expected: I,
    ) -> impl use<'_, T, T2, I> + Future<Output = ()>
    where
        T: Debug + PartialEq<T2>,
    {
        FutureTrackingCaller {
            future: async {
                self.with_stream(async |stream| {
                    let mut expected: Vec<T2> = expected.into_iter().collect();

                    while !expected.is_empty() {
                        if let Some(next) = stream.next().await {
                            let idx = expected.iter().enumerate().find(|(_, e)| &next == *e);
                            if let Some((i, _)) = idx {
                                expected.swap_remove(i);
                            } else {
                                Err(format!("Stream yielded unexpected message: {:?}", next))?
                            }
                        } else {
                            Err(format!(
                                "Stream ended early, still expected: {:?}",
                                expected
                            ))?
                        }
                    }

                    Ok(())
                })
                .await
            },
        }
    }

    /// Asserts that the stream yields only the expected sequence of messages, in some order,
    /// and then ends.
    pub fn assert_yields_only_unordered<T2: Debug, I: IntoIterator<Item = T2>>(
        &self,
        expected: I,
    ) -> impl use<'_, T, T2, I> + Future<Output = ()>
    where
        T: Debug + PartialEq<T2>,
    {
        ChainedFuture {
            first: self.assert_yields_unordered(expected),
            second: self.assert_no_more(),
            first_done: false,
        }
    }
}

impl<T: Serialize + DeserializeOwned, O: Ordering, R: Retries> SimSender<T, O, R> {
    fn with_sink<Out>(
        &self,
        thunk: impl FnOnce(&dyn Fn(T) -> Result<(), tokio::sync::mpsc::error::SendError<Bytes>>) -> Out,
    ) -> Out {
        let sender = CURRENT_SIM_CONNECTIONS.with(|connections| {
            let connections = &mut *connections.borrow_mut();
            connections
                .input_senders
                .get(connections.external_registered.get(&self.0).unwrap())
                .unwrap()
                .clone()
        });

        thunk(&move |t| sender.send(bincode::serialize(&t).unwrap().into()))
    }
}

impl<T: Serialize + DeserializeOwned, O: Ordering> SimSender<T, O, ExactlyOnce> {
    /// Sends several messages to the external bincode sink. The messages will be asynchronously
    /// processed as part of the simulation, in non-deterministic order.
    pub fn send_many_unordered<I: IntoIterator<Item = T>>(&self, iter: I) {
        self.with_sink(|send| {
            for t in iter {
                send(t).unwrap();
            }
        })
    }
}

impl<T: Serialize + DeserializeOwned> SimSender<T, TotalOrder, ExactlyOnce> {
    /// Sends a message to the external bincode sink. The message will be asynchronously processed
    /// as part of the simulation.
    pub fn send(&self, t: T) {
        self.with_sink(|send| send(t)).unwrap();
    }

    /// Sends several messages to the external bincode sink. The messages will be asynchronously
    /// processed as part of the simulation.
    pub fn send_many<I: IntoIterator<Item = T>>(&self, iter: I) {
        self.with_sink(|send| {
            for t in iter {
                send(t).unwrap();
            }
        })
    }
}

enum LogKind<W: std::io::Write> {
    Null,
    Stderr,
    Custom(W),
}

// via https://www.reddit.com/r/rust/comments/t69sld/is_there_a_way_to_allow_either_stdfmtwrite_or/
impl<W: std::io::Write> std::fmt::Write for LogKind<W> {
    fn write_str(&mut self, s: &str) -> Result<(), std::fmt::Error> {
        match self {
            LogKind::Null => Ok(()),
            LogKind::Stderr => {
                eprint!("{}", s);
                Ok(())
            }
            LogKind::Custom(w) => w.write_all(s.as_bytes()).map_err(|_| std::fmt::Error),
        }
    }
}

/// A running simulation, which manages the async DFIR and tick DFIRs, and makes decisions
/// about scheduling ticks and choices for non-deterministic operators like batch.
struct LaunchedSim<W: std::io::Write> {
    async_dfirs: Vec<(LocationId, Option<u32>, Dfir<'static>)>,
    possibly_ready_ticks: Vec<(LocationId, Option<u32>, Dfir<'static>)>,
    not_ready_ticks: Vec<(LocationId, Option<u32>, Dfir<'static>)>,
    possibly_ready_observation: Vec<(LocationId, Option<u32>)>,
    not_ready_observation: Vec<(LocationId, Option<u32>)>,
    hooks: Hooks<LocationId>,
    inline_hooks: InlineHooks<LocationId>,
    log: LogKind<W>,
}

impl<W: std::io::Write> LaunchedSim<W> {
    async fn scheduler(&mut self) {
        loop {
            tokio::task::yield_now().await;
            let mut any_made_progress = false;
            for (loc, c_id, dfir) in &mut self.async_dfirs {
                if dfir.run_tick().await {
                    any_made_progress = true;
                    let (now_ready, still_not_ready): (Vec<_>, Vec<_>) = self
                        .not_ready_ticks
                        .drain(..)
                        .partition(|(tick_loc, tick_c_id, _)| {
                            let LocationId::Tick(_, outer) = tick_loc else {
                                unreachable!()
                            };
                            outer.as_ref() == loc && tick_c_id == c_id
                        });

                    self.possibly_ready_ticks.extend(now_ready);
                    self.not_ready_ticks.extend(still_not_ready);

                    let (now_ready_obs, still_not_ready_obs): (Vec<_>, Vec<_>) = self
                        .not_ready_observation
                        .drain(..)
                        .partition(|(obs_loc, obs_c_id)| obs_loc == loc && obs_c_id == c_id);

                    self.possibly_ready_observation.extend(now_ready_obs);
                    self.not_ready_observation.extend(still_not_ready_obs);
                }
            }

            if any_made_progress {
                continue;
            } else {
                use bolero::generator::*;

                let (ready_tick, mut not_ready_tick): (Vec<_>, Vec<_>) = self
                    .possibly_ready_ticks
                    .drain(..)
                    .partition(|(name, cid, _)| {
                        self.hooks
                            .get(&(name.clone(), *cid))
                            .unwrap()
                            .iter()
                            .any(|hook| {
                                hook.current_decision().unwrap_or(false)
                                    || hook.can_make_nontrivial_decision()
                            })
                    });

                self.possibly_ready_ticks = ready_tick;
                self.not_ready_ticks.append(&mut not_ready_tick);

                let (ready_obs, mut not_ready_obs): (Vec<_>, Vec<_>) = self
                    .possibly_ready_observation
                    .drain(..)
                    .partition(|(name, cid)| {
                        self.hooks
                            .get(&(name.clone(), *cid))
                            .into_iter()
                            .flatten()
                            .any(|hook| {
                                hook.current_decision().unwrap_or(false)
                                    || hook.can_make_nontrivial_decision()
                            })
                    });

                self.possibly_ready_observation = ready_obs;
                self.not_ready_observation.append(&mut not_ready_obs);

                if self.possibly_ready_ticks.is_empty()
                    && self.possibly_ready_observation.is_empty()
                {
                    break;
                } else {
                    let next_tick_or_obs = (0..(self.possibly_ready_ticks.len()
                        + self.possibly_ready_observation.len()))
                        .any();

                    if next_tick_or_obs < self.possibly_ready_ticks.len() {
                        let next_tick = next_tick_or_obs;
                        let mut removed = self.possibly_ready_ticks.remove(next_tick);

                        match &mut self.log {
                            LogKind::Null => {}
                            LogKind::Stderr => {
                                if let Some(cid) = &removed.1 {
                                    eprintln!(
                                        "\n{}",
                                        format!("Running Tick (Cluster Member {})", cid)
                                            .color(colored::Color::Magenta)
                                            .bold()
                                    )
                                } else {
                                    eprintln!(
                                        "\n{}",
                                        "Running Tick".color(colored::Color::Magenta).bold()
                                    )
                                }
                            }
                            LogKind::Custom(writer) => {
                                writeln!(
                                    writer,
                                    "\n{}",
                                    "Running Tick".color(colored::Color::Magenta).bold()
                                )
                                .unwrap();
                            }
                        }

                        let mut asterisk_indenter = |_line_no, write: &mut dyn std::fmt::Write| {
                            write.write_str(&"*".color(colored::Color::Magenta).bold())?;
                            write.write_str(" ")
                        };

                        let mut tick_decision_writer = indenter::indented(&mut self.log)
                            .with_format(indenter::Format::Custom {
                                inserter: &mut asterisk_indenter,
                            });

                        let hooks = self.hooks.get_mut(&(removed.0.clone(), removed.1)).unwrap();
                        let mut remaining_decision_count = hooks.len();
                        let mut made_nontrivial_decision = false;

                        bolero_generator::any::scope::borrow_with(|driver| {
                            // first, scan manual decisions
                            hooks.iter_mut().for_each(|hook| {
                                if let Some(is_nontrivial) = hook.current_decision() {
                                    made_nontrivial_decision |= is_nontrivial;
                                    remaining_decision_count -= 1;
                                } else if !hook.can_make_nontrivial_decision() {
                                    // if no nontrivial decision is possible, make a trivial one
                                    // (we need to do this in the first pass to force nontrivial decisions
                                    // on the remaining hooks)
                                    hook.autonomous_decision(driver, false);
                                    remaining_decision_count -= 1;
                                }
                            });

                            hooks.iter_mut().for_each(|hook| {
                                if hook.current_decision().is_none() {
                                    made_nontrivial_decision |= hook.autonomous_decision(
                                        driver,
                                        !made_nontrivial_decision && remaining_decision_count == 1,
                                    );
                                    remaining_decision_count -= 1;
                                }

                                hook.release_decision(&mut tick_decision_writer);
                            });
                        });

                        let run_tick_future = removed.2.run_tick();
                        if let Some(inline_hooks) =
                            self.inline_hooks.get_mut(&(removed.0.clone(), removed.1))
                        {
                            let mut run_tick_future_pinned = pin!(run_tick_future);

                            loop {
                                tokio::select! {
                                    biased;
                                    r = &mut run_tick_future_pinned => {
                                        assert!(r);
                                        break;
                                    }
                                    _ = async {} => {
                                        bolero_generator::any::scope::borrow_with(|driver| {
                                            for hook in inline_hooks.iter_mut() {
                                                if hook.pending_decision() {
                                                    if !hook.has_decision() {
                                                        hook.autonomous_decision(driver);
                                                    }

                                                    hook.release_decision(&mut tick_decision_writer);
                                                }
                                            }
                                        });
                                    }
                                }
                            }
                        } else {
                            assert!(run_tick_future.await);
                        }

                        self.possibly_ready_ticks.push(removed);
                    } else {
                        let next_obs = next_tick_or_obs - self.possibly_ready_ticks.len();
                        let mut default_hooks = vec![];
                        let hooks = self
                            .hooks
                            .get_mut(&self.possibly_ready_observation[next_obs])
                            .unwrap_or(&mut default_hooks);

                        let mut remaining_decision_count = hooks.len();
                        let mut made_nontrivial_decision = false;
                        bolero_generator::any::scope::borrow_with(|driver| {
                            // first, scan manual decisions
                            hooks.iter_mut().for_each(|hook| {
                                if let Some(is_nontrivial) = hook.current_decision() {
                                    made_nontrivial_decision |= is_nontrivial;
                                    remaining_decision_count -= 1;
                                } else if !hook.can_make_nontrivial_decision() {
                                    // if no nontrivial decision is possible, make a trivial one
                                    // (we need to do this in the first pass to force nontrivial decisions
                                    // on the remaining hooks)
                                    hook.autonomous_decision(driver, false);
                                    remaining_decision_count -= 1;
                                }
                            });

                            hooks.iter_mut().for_each(|hook| {
                                if hook.current_decision().is_none() {
                                    made_nontrivial_decision |= hook.autonomous_decision(
                                        driver,
                                        !made_nontrivial_decision && remaining_decision_count == 1,
                                    );
                                    remaining_decision_count -= 1;
                                }

                                hook.release_decision(&mut self.log);
                            });
                        });
                    }
                }
            }
        }
    }
}
