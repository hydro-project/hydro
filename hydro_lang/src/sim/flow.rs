//! Entrypoint for compiling and running Hydro simulations.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::panic::RefUnwindSafe;
use std::rc::Rc;

use dfir_lang::graph::{DfirGraph, FlatGraphBuilder, FlatGraphBuilderOutput};
use libloading::Library;
use slotmap::{SecondaryMap, SparseSecondaryMap};

use super::builder::SimBuilder;
use super::compiled::{CompiledSim, CompiledSimInstance};
use super::graph::{SimDeploy, SimExternal, SimNode, compile_sim, create_sim_graph_trybuild};
use crate::compile::builder::StmtId;
use crate::compile::ir::HydroRoot;
use crate::location::LocationKey;
use crate::location::dynamic::LocationId;
use crate::prelude::Cluster;
use crate::sim::graph::SimExternalPortRegistry;
use crate::staging_util::Invariant;

/// A not-yet-compiled simulator for a Hydro program.
pub struct SimFlow<'a> {
    pub(crate) ir: Vec<HydroRoot>,

    /// SimNode for each Process.
    pub(crate) processes: SparseSecondaryMap<LocationKey, SimNode>,
    /// SimNode for each Cluster.
    pub(crate) clusters: SparseSecondaryMap<LocationKey, SimNode>,
    /// SimExternal for each External.
    pub(crate) externals: SparseSecondaryMap<LocationKey, SimExternal>,

    /// Max size of each cluster.
    pub(crate) cluster_max_sizes: SparseSecondaryMap<LocationKey, usize>,
    /// Handle to state handling `external`s' ports.
    pub(crate) externals_port_registry: Rc<RefCell<SimExternalPortRegistry>>,

    /// The program version each location belongs to (all `0` for a single-version flow). Every
    /// location has an entry.
    pub(crate) location_version: SecondaryMap<LocationKey, u32>,

    /// Maps each location to the root key of its cross-version correspondence group: version 0 of
    /// the same logical location. Every location has an entry (its own key unless it is a
    /// `next_version` successor); populated eagerly at location creation.
    pub(crate) location_version_group_root: SecondaryMap<LocationKey, LocationKey>,

    /// When true, the simulator only tests safety properties (not liveness).
    pub(crate) test_safety_only: bool,

    /// When true, consistency assertions are skipped (treated as identity no-ops).
    /// When false (default), encountering a consistency assertion panics because
    /// validating consistency assertions is not yet supported in the simulator.
    pub(crate) skip_consistency_assertions: bool,

    /// Number of iterations to use for fuzzing, defaults to 8192
    pub(crate) unit_test_fuzz_iterations: usize,

    pub(crate) _phantom: Invariant<'a>,
}

impl<'a> SimFlow<'a> {
    /// Sets the maximum size of the given cluster in the simulation.
    pub fn with_cluster_size<C>(mut self, cluster: &Cluster<'a, C>, max_size: usize) -> Self {
        self.cluster_max_sizes.insert(cluster.key, max_size);
        self
    }

    /// Opts in to safety-only testing, which is required when using
    /// [`lossy_delayed_forever`](crate::networking::NetworkingConfig::lossy_delayed_forever)
    /// networking.
    ///
    /// The simulator models dropped messages as indefinitely delayed, which means
    /// it only tests safety properties—not liveness—since messages may never arrive.
    /// Calling this method acknowledges that the simulation will not verify that the
    /// program eventually makes progress.
    pub fn test_safety_only(mut self) -> Self {
        self.test_safety_only = true;
        self
    }

    /// Opts in to skipping consistency assertions. When enabled, `assert_is_consistent`
    /// nodes are treated as identity no-ops in the simulator. When disabled (the default),
    /// encountering a consistency assertion will panic because validating consistency
    /// assertions is not yet supported in the simulator.
    pub fn skip_consistency_assertions(mut self) -> Self {
        self.skip_consistency_assertions = true;
        self
    }

    /// Sets the number of fuzz iterations for this test. Overrides the
    /// the default value of 8192
    pub fn unit_test_fuzz_iterations(mut self, iterations: usize) -> Self {
        self.unit_test_fuzz_iterations = iterations;
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
    pub fn fuzz(self, thunk: impl AsyncFn() + RefUnwindSafe) {
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
    pub fn exhaustive(self, thunk: impl AsyncFnMut() + RefUnwindSafe) -> usize {
        self.compiled().exhaustive(thunk)
    }

    /// Compiles the simulation into a dynamically loadable library, and returns a handle to it.
    pub fn compiled(mut self) -> CompiledSim {
        use dfir_lang::graph::{eliminate_extra_unions_tees, partition_graph};

        let is_multi_version = self.location_version.values().any(|&v| v > 0);

        let mut sim_emit = SimBuilder {
            process_graphs: BTreeMap::new(),
            cluster_graphs: BTreeMap::new(),
            process_tick_dfirs: BTreeMap::new(),
            cluster_tick_dfirs: BTreeMap::new(),
            extra_stmts_global: vec![],
            extra_stmts_cluster: BTreeMap::new(),
            next_hoff_id: crate::Counter::default(),
            test_safety_only: self.test_safety_only,
            skip_consistency_assertions: self.skip_consistency_assertions,
            channel_tables: BTreeMap::new(),
        };

        // Ensure the default (0) external is always present.
        self.externals.insert(
            LocationKey::FIRST,
            SimExternal {
                shared_inner: self.externals_port_registry.clone(),
            },
        );

        let mut seen_tees_instantiate: HashMap<_, _> = HashMap::new();
        let mut seen_cluster_members = HashSet::new();
        self.ir.iter_mut().for_each(|leaf| {
            leaf.compile_network::<SimDeploy>(
                &mut SparseSecondaryMap::new(),
                &mut seen_tees_instantiate,
                &mut seen_cluster_members,
                &self.processes,
                &self.clusters,
                &self.externals,
                &mut (),
            );
        });

        if is_multi_version {
            super::versioned_network::splice_versioned_networks(
                &mut self.ir,
                &self.location_version_group_root,
                &self.location_version,
            );
        }

        let mut seen_tees = HashMap::new();
        let mut built_tees = HashMap::new();
        let mut next_stmt_id = crate::Counter::<StmtId>::default();
        let mut fold_hooked_idents = HashSet::new();
        for leaf in &mut self.ir {
            leaf.emit(
                &mut sim_emit,
                &mut seen_tees,
                &mut built_tees,
                &mut next_stmt_id,
                &mut fold_hooked_idents,
            );
        }

        fn build_graphs(
            graphs: BTreeMap<LocationId, FlatGraphBuilder>,
        ) -> BTreeMap<LocationId, DfirGraph> {
            graphs
                .into_iter()
                .map(|(l, g)| {
                    let FlatGraphBuilderOutput { mut flat_graph, .. } =
                        g.build().expect("Failed to build DFIR flat graph.");
                    eliminate_extra_unions_tees(&mut flat_graph);
                    (
                        l,
                        partition_graph(flat_graph).expect("Failed to partition (cycle detected)."),
                    )
                })
                .collect()
        }

        let process_graphs = build_graphs(sim_emit.process_graphs);
        let cluster_graphs = build_graphs(sim_emit.cluster_graphs);
        let process_tick_graphs = build_graphs(sim_emit.process_tick_dfirs);
        let cluster_tick_graphs = build_graphs(sim_emit.cluster_tick_dfirs);

        #[expect(
            clippy::disallowed_methods,
            reason = "nondeterministic iteration order, fine for checks"
        )]
        for c in self.clusters.keys() {
            assert!(
                self.cluster_max_sizes.contains_key(c),
                "Cluster {:?} missing max size; call with_cluster_size() before compiled()",
                c
            );
        }

        let (cluster_max_sizes, cluster_member_ids) = self.cluster_sizing();

        let (bin, trybuild) = create_sim_graph_trybuild(
            process_graphs,
            cluster_graphs,
            cluster_max_sizes,
            cluster_member_ids,
            process_tick_graphs,
            cluster_tick_graphs,
            sim_emit.extra_stmts_global,
            sim_emit.extra_stmts_cluster,
        );

        let out = compile_sim(bin, trybuild).unwrap();
        let lib = unsafe { Library::new(&out).unwrap() };

        CompiledSim {
            _path: out,
            lib,
            externals_port_registry: self.externals_port_registry.take(),
            unit_test_fuzz_iterations: self.unit_test_fuzz_iterations,
        }
    }

    /// Computes each cluster's merged size and the global member-id slice it constructs.
    ///
    /// Corresponding clusters (a [`next_version`](crate::location::Cluster::next_version) chain)
    /// share a group key; the merged size for a group is the sum of its per-version sizes, and each
    /// version gets a contiguous member-id slice assigned in version order. A single-version
    /// cluster is the degenerate case: its own group, one version, slice `0..size`.
    fn cluster_sizing(
        &self,
    ) -> (
        SparseSecondaryMap<LocationKey, usize>,
        BTreeMap<LocationId, Vec<u32>>,
    ) {
        // Group corresponding clusters by their shared group root, recording each version's size.
        let mut sizes_by_group_root: BTreeMap<LocationKey, BTreeMap<u32, usize>> = BTreeMap::new();
        #[expect(
            clippy::disallowed_methods,
            reason = "each cluster key is unique; iteration order does not affect the result"
        )]
        for key in self.clusters.keys() {
            let group_root = self.location_version_group_root[key];
            let version = self.location_version[key];
            let size = *self.cluster_max_sizes.get(key).unwrap_or_else(|| {
                panic!(
                    "cluster {key:?} missing max size; `compiled()` asserts every cluster has one \
                     before calling `cluster_sizing`"
                )
            });
            let prev = sizes_by_group_root
                .entry(group_root)
                .or_default()
                .insert(version, size);
            assert!(
                prev.is_none(),
                "multi-version simulation has two corresponding clusters at the same version; \
                 each `next_version()` call must advance to a distinct version"
            );
        }

        // Each cluster location gets the merged total size (so its membership lists the union) and
        // its own contiguous slice of the global member-id range, assigned in version order.
        let mut cluster_sizes: SparseSecondaryMap<LocationKey, usize> = SparseSecondaryMap::new();
        let mut cluster_member_ids: BTreeMap<LocationId, Vec<u32>> = BTreeMap::new();
        #[expect(
            clippy::disallowed_methods,
            reason = "each cluster key is unique; iteration order does not affect the result"
        )]
        for key in self.clusters.keys() {
            let group_root = self.location_version_group_root[key];
            let version = self.location_version[key];
            let per_version = &sizes_by_group_root[&group_root];
            let merged_total: usize = per_version.values().sum();
            let offset: u32 = per_version.range(..version).map(|(_, &n)| n as u32).sum();
            let size = *per_version
                .get(&version)
                .expect("every (group, version) was recorded by the first pass above")
                as u32;
            cluster_sizes.insert(key, merged_total);
            cluster_member_ids.insert(LocationId::Cluster(key), (offset..offset + size).collect());
        }

        (cluster_sizes, cluster_member_ids)
    }
}
