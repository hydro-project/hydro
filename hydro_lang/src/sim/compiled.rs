//! Interfaces for compiled Hydro simulators and concrete simulation instances.

use std::collections::{HashMap, HashSet};
use std::panic::RefUnwindSafe;
use std::path::Path;

use bytes::Bytes;
use colored::Colorize;
use dfir_rs::scheduled::graph::Dfir;
use futures::{Stream, StreamExt};
use libloading::Library;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tempfile::TempPath;
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::wrappers::UnboundedReceiverStream;

use super::runtime::SimHook;
use crate::location::external_process::{ExternalBincodeSink, ExternalBincodeStream};

/// A handle to a compiled Hydro simulation, which can be instantiated and run.
pub struct CompiledSim {
    pub(super) _path: TempPath,
    pub(super) lib: Library,
    pub(super) external_ports: Vec<usize>,
}

#[sealed::sealed]
/// A trait implemented by closures that can instantiate a compiled simulation.
///
/// This is needed to ensure [`RefUnwindSafe`] so instances can be created during fuzzing.
pub trait Instantiator<'a>: RefUnwindSafe + Fn() -> CompiledSimInstance<'a> {}
#[sealed::sealed]
impl<'a, T: RefUnwindSafe + Fn() -> CompiledSimInstance<'a>> Instantiator<'a> for T {}

type SimLoaded<'a> = libloading::Symbol<
    'a,
    unsafe extern "Rust" fn(
        bool,
        HashMap<usize, UnboundedSender<Bytes>>,
        HashMap<usize, UnboundedReceiverStream<Bytes>>,
    ) -> (
        Dfir<'static>,
        Vec<(&'static str, Dfir<'static>)>,
        HashMap<&'static str, Vec<Box<dyn SimHook>>>,
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
    /// The `log` parameter controls whether to log tick executions and stream releases.
    pub fn with_instantiator<T>(&self, thunk: impl FnOnce(&dyn Instantiator) -> T, log: bool) -> T {
        let func: SimLoaded = unsafe { self.lib.get(b"__hydro_runtime").unwrap() };
        thunk(
            &(|| CompiledSimInstance {
                func: func.clone(),
                remaining_ports: self.external_ports.iter().cloned().collect(),
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
    pub fn exhaustive<'a>(&'a self, thunk: impl AsyncFn(CompiledSimInstance) + RefUnwindSafe) {
        if std::env::var("BOLERO_FUZZER").is_ok() {
            eprintln!(
                "Cannot run exhaustive tests with a fuzzer. Please use `cargo test` instead of `cargo sim`."
            );
            std::process::abort();
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
                .exhaustive()
                .run(move || {
                    let instance = instantiator();
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
    }
}

/// A single instance of a compiled Hydro simulation, which provides methods to interactively
/// execute the simulation, feed inputs, and receive outputs.
pub struct CompiledSimInstance<'a> {
    func: SimLoaded<'a>,
    remaining_ports: HashSet<usize>,
    output_ports: HashMap<usize, UnboundedSender<Bytes>>,
    input_ports: HashMap<usize, UnboundedReceiverStream<Bytes>>,
    log: bool,
}

impl<'a> CompiledSimInstance<'a> {
    /// Like the corresponding method on [`crate::compile::deploy::DeployResult`], connects to the
    /// given input port, and returns a closure that can be used to send messages to it.
    pub fn connect_sink_bincode<T: 'static + Send + Serialize + DeserializeOwned>(
        &mut self,
        port: &ExternalBincodeSink<T>,
    ) -> impl Fn(T) -> Result<(), tokio::sync::mpsc::error::SendError<Bytes>> + 'a {
        assert!(self.remaining_ports.remove(&port.port_id));
        let (sink, source) = dfir_rs::util::unbounded_channel::<Bytes>();
        self.input_ports.insert(port.port_id, source);
        move |t| sink.send(bincode::serialize(&t).unwrap().into())
    }

    /// Like the corresponding method on [`crate::compile::deploy::DeployResult`], connects to the
    /// given output port, and returns a stream that can be used to receive messages from it.
    pub fn connect_source_bincode<T: 'static + Send + Serialize + DeserializeOwned>(
        &mut self,
        port: &ExternalBincodeStream<T>,
    ) -> impl Stream<Item = T> + 'a {
        assert!(self.remaining_ports.remove(&port.port_id));
        let (sink, source) = dfir_rs::util::unbounded_channel::<Bytes>();
        self.output_ports.insert(port.port_id, sink);
        source.map(|b| bincode::deserialize(&b).unwrap())
    }

    /// Launches the simulation, which will asynchronously simulate the Hydro program. This should
    /// be invoked after connecting all inputs and outputs, but before receiving any messages.
    pub fn launch(self) {
        let logger = if self.log
            || std::env::var("HYDRO_SIM_LOG")
                .map(|v| v == "1")
                .unwrap_or(false)
        {
            Some(std::io::stderr())
        } else {
            None
        };

        tokio::task::spawn_local(self.schedule_with_logger(logger));
    }

    /// Returns a future that schedules simulation with the given logger for reporting the
    /// simulation trace.
    ///
    /// See [`Self::launch`] for more details.
    pub fn schedule_with_logger<W: std::io::Write>(
        self,
        log_writer: Option<W>,
    ) -> impl use<W> + Future<Output = ()> {
        if !self.remaining_ports.is_empty() {
            panic!(
                "Cannot launch DFIR because some of the inputs / outputs have not been connected."
            )
        }

        let (async_dfir, ticks, hooks) = unsafe {
            (self.func)(
                colored::control::SHOULD_COLORIZE.should_colorize(),
                self.output_ports,
                self.input_ports,
            )
        };
        let mut launched = LaunchedSim {
            async_dfir,
            possibly_ready_ticks: vec![],
            not_ready_ticks: ticks.into_iter().collect(),
            hooks,
            log_writer,
        };

        async move { launched.scheduler().await }
    }
}

// via https://www.reddit.com/r/rust/comments/t69sld/is_there_a_way_to_allow_either_stdfmtwrite_or/
struct FmtWriter<W: std::io::Write>(W);
impl<W: std::io::Write> std::fmt::Write for FmtWriter<W> {
    fn write_str(&mut self, s: &str) -> Result<(), std::fmt::Error> {
        self.0.write_all(s.as_bytes()).map_err(|_| std::fmt::Error)
    }

    fn write_fmt(&mut self, args: std::fmt::Arguments<'_>) -> Result<(), std::fmt::Error> {
        self.0.write_fmt(args).map_err(|_| std::fmt::Error)
    }
}

/// A running simulation, which manages the async DFIR and tick DFIRs, and makes decisions
/// about scheduling ticks and choices for non-deterministic operators like batch.
struct LaunchedSim<W: std::io::Write> {
    async_dfir: Dfir<'static>,
    possibly_ready_ticks: Vec<(&'static str, Dfir<'static>)>,
    not_ready_ticks: Vec<(&'static str, Dfir<'static>)>,
    hooks: HashMap<&'static str, Vec<Box<dyn SimHook>>>,
    log_writer: Option<W>,
}

impl<W: std::io::Write> LaunchedSim<W> {
    async fn scheduler(&mut self) {
        loop {
            tokio::task::yield_now().await;
            if self.async_dfir.run_available().await {
                self.possibly_ready_ticks.append(&mut self.not_ready_ticks);
                continue;
            } else {
                use bolero::generator::*;

                let (ready, mut not_ready): (Vec<_>, Vec<_>) =
                    self.possibly_ready_ticks.drain(..).partition(|(name, _)| {
                        self.hooks.get(name).unwrap().iter().any(|hook| {
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
                    let mut removed: (&'static str, Dfir<'static>) =
                        self.possibly_ready_ticks.remove(next_tick);

                    if let Some(log_writer) = self.log_writer.as_mut() {
                        let _ = writeln!(
                            log_writer,
                            "\n{}",
                            "Running Tick".color(colored::Color::Magenta).bold()
                        );
                    }

                    let mut fmt_writer = self.log_writer.as_mut().map(FmtWriter);
                    let mut asterisk_indenter = |_line_no, write: &mut dyn std::fmt::Write| {
                        write.write_str(&"*".color(colored::Color::Magenta).bold())?;
                        write.write_str(" ")
                    };

                    let mut tick_decision_writer = fmt_writer.as_mut().map(|w| {
                        indenter::indented(w).with_format(indenter::Format::Custom {
                            inserter: &mut asterisk_indenter,
                        })
                    });

                    let hooks = self.hooks.get_mut(removed.0).unwrap();
                    let mut remaining_decision_count = hooks.len();
                    let mut made_nontrivial_decision = false;

                    // first, scan manual decisions
                    hooks.iter_mut().for_each(|hook| {
                        if let Some(is_nontrivial) = hook.current_decision() {
                            made_nontrivial_decision |= is_nontrivial;
                            remaining_decision_count -= 1;
                        }
                    });

                    bolero_generator::any::scope::borrow_with(|driver| {
                        hooks.iter_mut().for_each(|hook| {
                            if hook.current_decision().is_none() {
                                made_nontrivial_decision |= hook.autonomous_decision(
                                    driver,
                                    !made_nontrivial_decision && remaining_decision_count == 1,
                                );
                                remaining_decision_count -= 1;
                            }

                            if let Some(log_mut) = tick_decision_writer.as_mut() {
                                hook.release_decision(Some(log_mut));
                            } else {
                                hook.release_decision(None);
                            }
                        });
                    });

                    assert!(removed.1.run_tick().await);
                    self.possibly_ready_ticks.push(removed);
                }
            }
        }
    }
}
