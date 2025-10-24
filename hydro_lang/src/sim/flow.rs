//! Entrypoint for compiling and running Hydro simulations.

use std::cell::RefCell;
use std::collections::HashMap;
use std::panic::RefUnwindSafe;
use std::rc::Rc;

use libloading::Library;

use super::builder::SimBuilder;
use super::compiled::{CompiledSim, CompiledSimInstance};
use super::graph::{SimDeploy, SimExternal, SimNode, compile_sim, create_sim_graph_trybuild};
use crate::compile::ir::HydroRoot;
use crate::location::Location;
use crate::location::dynamic::LocationId;
use crate::prelude::Cluster;
use crate::staging_util::Invariant;

/// A not-yet-compiled simulator for a Hydro program.
pub struct SimFlow<'a> {
    pub(crate) ir: Vec<HydroRoot>,

    pub(crate) external_ports: Rc<RefCell<(Vec<usize>, usize)>>,

    pub(crate) processes: HashMap<usize, SimNode>,
    pub(crate) clusters: HashMap<usize, SimNode>,
    pub(crate) externals: HashMap<usize, SimExternal>,

    pub(crate) cluster_max_sizes: HashMap<LocationId, usize>,

    /// Lists all the processes that were created in the flow, same ID as `processes`
    /// but with the type name of the tag.
    pub(crate) _process_id_name: Vec<(usize, String)>,
    pub(crate) _external_id_name: Vec<(usize, String)>,
    pub(crate) _cluster_id_name: Vec<(usize, String)>,

    pub(crate) _phantom: Invariant<'a>,
}

impl<'a> SimFlow<'a> {
    /// Sets the maximum size of the given cluster in the simulation.
    pub fn with_cluster_size<C>(mut self, cluster: &Cluster<'a, C>, max_size: usize) -> Self {
        self.cluster_max_sizes.insert(cluster.id(), max_size);
        self
    }

    /// Executes the given closure with a single instance of the compiled simulation.
    pub fn with_instance<T>(self, thunk: impl FnOnce(CompiledSimInstance) -> T) -> T {
        self.compiled().with_instance(thunk)
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
    pub fn fuzz(self, thunk: impl AsyncFn(CompiledSimInstance) + RefUnwindSafe) {
        self.compiled().fuzz(thunk)
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
    pub fn exhaustive(self, thunk: impl AsyncFn(CompiledSimInstance) + RefUnwindSafe) -> usize {
        self.compiled().exhaustive(thunk)
    }

    /// Compiles the simulation into a dynamically loadable library, and returns a handle to it.
    pub fn compiled(mut self) -> CompiledSim {
        use std::collections::BTreeMap;

        use dfir_lang::graph::{eliminate_extra_unions_tees, partition_graph};

        let mut sim_emit = SimBuilder {
            process_graphs: BTreeMap::new(),
            cluster_graphs: BTreeMap::new(),
            process_tick_dfirs: BTreeMap::new(),
            cluster_tick_dfirs: BTreeMap::new(),
            extra_stmts_global: vec![],
            extra_stmts_cluster: BTreeMap::new(),
            next_hoff_id: 0,
        };

        let mut seen_tees_instantiate: HashMap<_, _> = HashMap::new();
        self.ir.iter_mut().for_each(|leaf| {
            leaf.compile_network::<SimDeploy>(
                &(),
                &mut BTreeMap::new(),
                &mut seen_tees_instantiate,
                &self.processes,
                &self.clusters,
                &self.externals,
            );
        });

        let mut built_tees = HashMap::new();
        let mut next_stmt_id = 0;
        for leaf in &mut self.ir {
            leaf.emit(&mut sim_emit, &mut built_tees, &mut next_stmt_id);
        }

        let process_graphs = sim_emit
            .process_graphs
            .into_iter()
            .map(|(l, g)| {
                let (mut flat_graph, _, _) = g.build();
                eliminate_extra_unions_tees(&mut flat_graph);
                (
                    l,
                    partition_graph(flat_graph).expect("Failed to partition (cycle detected)."),
                )
            })
            .collect::<BTreeMap<_, _>>();

        let cluster_graphs = sim_emit
            .cluster_graphs
            .into_iter()
            .map(|(l, g)| {
                let (mut flat_graph, _, _) = g.build();
                eliminate_extra_unions_tees(&mut flat_graph);
                (
                    l,
                    partition_graph(flat_graph).expect("Failed to partition (cycle detected)."),
                )
            })
            .collect::<BTreeMap<_, _>>();

        let process_tick_graphs = sim_emit
            .process_tick_dfirs
            .into_iter()
            .map(|(l, g)| {
                let (mut flat_graph, _, _) = g.build();
                eliminate_extra_unions_tees(&mut flat_graph);
                (
                    l,
                    partition_graph(flat_graph).expect("Failed to partition (cycle detected)."),
                )
            })
            .collect::<BTreeMap<_, _>>();

        let cluster_tick_graphs = sim_emit
            .cluster_tick_dfirs
            .into_iter()
            .map(|(l, g)| {
                let (mut flat_graph, _, _) = g.build();
                eliminate_extra_unions_tees(&mut flat_graph);
                (
                    l,
                    partition_graph(flat_graph).expect("Failed to partition (cycle detected)."),
                )
            })
            .collect::<BTreeMap<_, _>>();

        let (bin, trybuild) = create_sim_graph_trybuild(
            process_graphs,
            cluster_graphs,
            self.cluster_max_sizes,
            process_tick_graphs,
            cluster_tick_graphs,
            sim_emit.extra_stmts_global,
            sim_emit.extra_stmts_cluster,
        );

        let out = compile_sim(bin, trybuild).unwrap();
        let lib = unsafe { Library::new(&out).unwrap() };

        let external_ports = self.external_ports.take().0;
        CompiledSim {
            _path: out,
            lib,
            external_ports,
        }
    }
}
