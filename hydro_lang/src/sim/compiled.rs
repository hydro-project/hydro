//! Interfaces for compiled Hydro simulators and concrete simulation instances.

use core::fmt;
use std::collections::{HashMap, HashSet, VecDeque};
use std::marker::PhantomData;
use std::panic::RefUnwindSafe;
use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use bytes::Bytes;
use colored::Colorize;
use dfir_rs::scheduled::graph::Dfir;
use futures::{FutureExt, Stream, StreamExt};
use libloading::Library;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tempfile::TempPath;
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::wrappers::UnboundedReceiverStream;

use super::runtime::SimHook;
use crate::compile::deploy::ConnectableAsync;
use crate::live_collections::stream::{ExactlyOnce, NoOrder, Ordering, Retries, TotalOrder};
use crate::location::dynamic::LocationId;
use crate::location::external_process::{ExternalBincodeSink, ExternalBincodeStream};

/// A handle to a compiled Hydro simulation, which can be instantiated and run.
pub struct CompiledSim {
    pub(super) _path: TempPath,
    pub(super) lib: Library,
    pub(super) external_ports: Vec<usize>,
    pub(super) external_registered: HashMap<usize, usize>,
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

type SimLoaded<'a> = libloading::Symbol<
    'a,
    unsafe extern "Rust" fn(
        bool,
        HashMap<usize, UnboundedSender<Bytes>>,
        HashMap<usize, UnboundedReceiverStream<Bytes>>,
        fn(fmt::Arguments<'_>),
        fn(fmt::Arguments<'_>),
    ) -> (
        Vec<(&'static str, Option<u32>, Dfir<'static>)>,
        Vec<(&'static str, Option<u32>, Dfir<'static>)>,
        HashMap<(&'static str, Option<u32>), Vec<Box<dyn SimHook>>>,
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
                remaining_ports: self.external_ports.iter().cloned().collect(),
                external_registered: self.external_registered.clone(),
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
    pub fn fuzz<'a>(&'a self, thunk: impl AsyncFn(CompiledSimInstance) + RefUnwindSafe) {
        let caller_fn = crate::compile::ir::backtrace::Backtrace::get_backtrace(0)
            .elements()
            .into_iter()
            .find(|e| {
                !e.fn_name.starts_with("hydro_lang::sim::compiled")
                    && !e.fn_name.starts_with("hydro_lang::sim::flow")
                    && !e.fn_name.starts_with("fuzz<")
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

            unsafe {
                std::env::set_var(
                    "BOLERO_FAILURE_OUTPUT",
                    caller_fuzz_repro_path.to_str().unwrap(),
                );

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
                            .block_on(async {
                                let local_set = tokio::task::LocalSet::new();
                                local_set.run_until(thunk(instance)).await
                            })
                    })
                },
                false,
            );
        } else if let Ok(existing_bytes) = std::fs::read(&caller_fuzz_repro_path) {
            self.fuzz_repro(existing_bytes, thunk);
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
                            .block_on(async {
                                let local_set = tokio::task::LocalSet::new();
                                local_set.run_until(thunk(instance)).await
                            })
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
                        .block_on(async {
                            let local_set = tokio::task::LocalSet::new();
                            local_set.run_until(thunk(instance)).await
                        })
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
    pub fn exhaustive<'a>(
        &'a self,
        thunk: impl AsyncFn(CompiledSimInstance) + RefUnwindSafe,
    ) -> usize {
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
                        .block_on(async {
                            let local_set = tokio::task::LocalSet::new();
                            local_set.run_until(thunk(instance)).await;
                        })
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
    remaining_ports: HashSet<usize>,
    external_registered: HashMap<usize, usize>,
    output_ports: HashMap<usize, UnboundedSender<Bytes>>,
    input_ports: HashMap<usize, UnboundedReceiverStream<Bytes>>,
    log: bool,
}

impl<'a> CompiledSimInstance<'a> {
    #[deprecated(note = "Use `connect` instead")]
    /// Like the corresponding method on [`crate::compile::deploy::DeployResult`], connects to the
    /// given input port, and returns a closure that can be used to send messages to it.
    pub fn connect_sink_bincode<T: Serialize + 'static, M, O: Ordering, R: Retries>(
        &mut self,
        port: &ExternalBincodeSink<T, M, O, R>,
    ) -> SimSender<T, O, R> {
        self.connect(port)
    }

    #[deprecated(note = "Use `connect` instead")]
    /// Like the corresponding method on [`crate::compile::deploy::DeployResult`], connects to the
    /// given output port, and returns a stream that can be used to receive messages from it.
    pub fn connect_source_bincode<T: DeserializeOwned + 'static, O: Ordering, R: Retries>(
        &mut self,
        port: &ExternalBincodeStream<T, O, R>,
    ) -> SimReceiver<'a, T, O, R> {
        self.connect(port)
    }

    /// Establishes a connection to the given input or output port, returning either a
    /// [`SimSender`] (for input ports) or a stream (for output ports). This should be invoked
    /// before calling [`Self::launch`], and should only be invoked once per port.
    pub fn connect<'b, P: ConnectableAsync<&'b mut Self>>(
        &'b mut self,
        port: P,
    ) -> <P as ConnectableAsync<&'b mut Self>>::Output {
        let mut pinned = std::pin::pin!(port.connect(self));
        if let Poll::Ready(v) = pinned.poll_unpin(&mut Context::from_waker(Waker::noop())) {
            v
        } else {
            panic!("Connect impl should not have used any async operations");
        }
    }

    /// Launches the simulation, which will asynchronously simulate the Hydro program. This should
    /// be invoked after connecting all inputs and outputs, but before receiving any messages.
    pub fn launch(self) {
        tokio::task::spawn_local(self.schedule_with_maybe_logger::<std::io::Empty>(None));
    }

    /// Returns a future that schedules simulation with the given logger for reporting the
    /// simulation trace.
    ///
    /// See [`Self::launch`] for more details.
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
        if !self.remaining_ports.is_empty() {
            panic!(
                "Cannot launch DFIR because some of the inputs / outputs have not been connected."
            )
        }

        let (async_dfirs, tick_dfirs, hooks) = unsafe {
            (self.func)(
                colored::control::SHOULD_COLORIZE.should_colorize(),
                self.output_ports,
                self.input_ports,
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
            hooks: hooks
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

/// A receiver for an external bincode stream in a simulation.
pub struct SimReceiver<'a, T, O: Ordering, R: Retries>(
    Pin<Box<dyn Stream<Item = T> + 'a>>,
    PhantomData<(O, R)>,
);

impl<'a, T, O: Ordering, R: Retries> SimReceiver<'a, T, O, R> {
    /// Asserts that the stream has ended and no more messages can possibly arrive.
    pub async fn assert_no_more(mut self)
    where
        T: std::fmt::Debug,
    {
        if let Some(next) = self.0.next().await {
            panic!("Stream yielded unexpected message: {:?}", next);
        }
    }
}

impl<'a, T> SimReceiver<'a, T, TotalOrder, ExactlyOnce> {
    /// Receives the next message from the external bincode stream. This will wait until a message
    /// is available, or return `None` if no more messages can possibly arrive.
    pub async fn next(&mut self) -> Option<T> {
        self.0.next().await
    }

    /// Collects all remaining messages from the external bincode stream into a collection. This
    /// will wait until no more messages can possibly arrive.
    pub async fn collect<C: Default + Extend<T>>(self) -> C {
        self.0.collect().await
    }

    /// Asserts that the stream yields exactly the expected sequence of messages, in order.
    /// This does not check that the stream ends, use [`Self::assert_yields_only`] for that.
    pub async fn assert_yields(&mut self, expected: impl IntoIterator<Item = T>)
    where
        T: std::fmt::Debug + PartialEq,
    {
        let mut expected: VecDeque<T> = expected.into_iter().collect();

        while !expected.is_empty() {
            if let Some(next) = self.next().await {
                assert_eq!(next, expected.pop_front().unwrap());
            } else {
                panic!("Stream ended early, still expected: {:?}", expected);
            }
        }
    }

    /// Asserts that the stream yields only the expected sequence of messages, in order,
    /// and then ends.
    pub async fn assert_yields_only(mut self, expected: impl IntoIterator<Item = T>)
    where
        T: std::fmt::Debug + PartialEq,
    {
        self.assert_yields(expected).await;
        self.assert_no_more().await;
    }
}

impl<'a, T> SimReceiver<'a, T, NoOrder, ExactlyOnce> {
    /// Collects all remaining messages from the external bincode stream into a collection,
    /// sorting them. This will wait until no more messages can possibly arrive.
    pub async fn collect_sorted<C: Default + Extend<T> + AsMut<[T]>>(self) -> C
    where
        T: Ord,
    {
        let mut collected: C = self.0.collect().await;
        collected.as_mut().sort();
        collected
    }

    /// Asserts that the stream yields exactly the expected sequence of messages, in some order.
    /// This does not check that the stream ends, use [`Self::assert_yields_only_unordered`] for that.
    pub async fn assert_yields_unordered(&mut self, expected: impl IntoIterator<Item = T>)
    where
        T: std::fmt::Debug + PartialEq,
    {
        let mut expected: Vec<T> = expected.into_iter().collect();

        while !expected.is_empty() {
            if let Some(next) = self.0.next().await {
                let idx = expected.iter().enumerate().find(|(_, e)| *e == &next);
                if let Some((i, _)) = idx {
                    expected.swap_remove(i);
                } else {
                    panic!("Stream yielded unexpected message: {:?}", next);
                }
            } else {
                panic!("Stream ended early, still expected: {:?}", expected);
            }
        }
    }

    /// Asserts that the stream yields only the expected sequence of messages, in some order,
    /// and then ends.
    pub async fn assert_yields_only_unordered(mut self, expected: impl IntoIterator<Item = T>)
    where
        T: std::fmt::Debug + PartialEq,
    {
        self.assert_yields_unordered(expected).await;
        self.assert_no_more().await;
    }
}

impl<'a, T: DeserializeOwned + 'static, O: Ordering, R: Retries>
    ConnectableAsync<&mut CompiledSimInstance<'a>> for &ExternalBincodeStream<T, O, R>
{
    type Output = SimReceiver<'a, T, O, R>;

    async fn connect(self, ctx: &mut CompiledSimInstance<'a>) -> Self::Output {
        let looked_up = ctx.external_registered.get(&self.port_id).unwrap();

        assert!(ctx.remaining_ports.remove(looked_up));
        let (sink, source) = dfir_rs::util::unbounded_channel::<Bytes>();
        ctx.output_ports.insert(*looked_up, sink);

        SimReceiver(
            Box::pin(source.map(|b| bincode::deserialize(&b).unwrap())),
            PhantomData,
        )
    }
}

/// A sender to an external bincode sink in a simulation.
pub struct SimSender<T, O: Ordering, R: Retries>(
    Box<dyn Fn(T) -> Result<(), tokio::sync::mpsc::error::SendError<Bytes>>>,
    PhantomData<(O, R)>,
);
impl<T> SimSender<T, TotalOrder, ExactlyOnce> {
    /// Sends a message to the external bincode sink. The message will be asynchronously processed
    /// as part of the simulation.
    pub fn send(&self, t: T) -> Result<(), tokio::sync::mpsc::error::SendError<Bytes>> {
        (self.0)(t)
    }

    /// Sends several messages to the external bincode sink. The messages will be asynchronously
    /// processed as part of the simulation.
    pub fn send_many<I: IntoIterator<Item = T>>(
        &self,
        iter: I,
    ) -> Result<(), tokio::sync::mpsc::error::SendError<Bytes>> {
        for t in iter {
            (self.0)(t)?;
        }
        Ok(())
    }
}

impl<T> SimSender<T, NoOrder, ExactlyOnce> {
    /// Sends several messages to the external bincode sink. The messages will be asynchronously
    /// processed as part of the simulation, in non-determinstic order.
    pub fn send_many_unordered<I: IntoIterator<Item = T>>(
        &self,
        iter: I,
    ) -> Result<(), tokio::sync::mpsc::error::SendError<Bytes>> {
        for t in iter {
            (self.0)(t)?;
        }
        Ok(())
    }
}

impl<'a, T: Serialize + 'static, M, O: Ordering, R: Retries>
    ConnectableAsync<&mut CompiledSimInstance<'a>> for &ExternalBincodeSink<T, M, O, R>
{
    type Output = SimSender<T, O, R>;

    async fn connect(self, ctx: &mut CompiledSimInstance<'a>) -> Self::Output {
        let looked_up = ctx.external_registered.get(&self.port_id).unwrap();

        assert!(ctx.remaining_ports.remove(looked_up));
        let (sink, source) = dfir_rs::util::unbounded_channel::<Bytes>();
        ctx.input_ports.insert(*looked_up, source);
        SimSender(
            Box::new(move |t| sink.send(bincode::serialize(&t).unwrap().into())),
            PhantomData,
        )
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

type Hooks = HashMap<(LocationId, Option<u32>), Vec<Box<dyn SimHook>>>;

/// A running simulation, which manages the async DFIR and tick DFIRs, and makes decisions
/// about scheduling ticks and choices for non-deterministic operators like batch.
struct LaunchedSim<W: std::io::Write> {
    async_dfirs: Vec<(LocationId, Option<u32>, Dfir<'static>)>,
    possibly_ready_ticks: Vec<(LocationId, Option<u32>, Dfir<'static>)>,
    not_ready_ticks: Vec<(LocationId, Option<u32>, Dfir<'static>)>,
    hooks: Hooks,
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
                }
            }

            if any_made_progress {
                continue;
            } else {
                use bolero::generator::*;

                let (ready, mut not_ready): (Vec<_>, Vec<_>) = self
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

                self.possibly_ready_ticks = ready;
                self.not_ready_ticks.append(&mut not_ready);

                if self.possibly_ready_ticks.is_empty() {
                    break;
                } else {
                    let next_tick = (0..self.possibly_ready_ticks.len()).any();
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

                    let mut tick_decision_writer =
                        indenter::indented(&mut self.log).with_format(indenter::Format::Custom {
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

                    assert!(removed.2.run_tick().await);
                    self.possibly_ready_ticks.push(removed);
                }
            }
        }
    }
}
