use std::cell::{RefCell, UnsafeCell};
use std::collections::HashMap;
use std::fs;
use std::process::{Command, Stdio};
use std::rc::Rc;

use dfir_lang::graph::{DfirGraph, FlatGraphBuilder};
use libloading::Library;
use quote::quote;
use sha2::{Digest, Sha256};
use syn::visit_mut::VisitMut;
use trybuild_internals_api::{cargo, dependencies, path};

use super::compiled::{CompiledSim, CompiledSimInstance};
use crate::compile::deploy_provider::{Deploy, DynSourceSink, Node, RegisterPort};
use crate::compile::ir::{HydroRoot, SimBuilder};
#[cfg(stageleft_runtime)]
use crate::deploy::trybuild::{
    CONCURRENT_TEST_LOCK, IS_TEST, TrybuildConfig, create_trybuild, write_atomic,
};
#[cfg(stageleft_runtime)]
use crate::deploy::trybuild_rewriters::UseTestModeStaged;
use crate::staging_util::Invariant;

#[derive(Clone)]
pub struct SimNode {}

impl Node for SimNode {
    type Port = ();
    type Meta = ();
    type InstantiateEnv = ();

    fn next_port(&self) -> Self::Port {
        todo!()
    }

    fn update_meta(&mut self, _meta: &Self::Meta) {}

    fn instantiate(
        &self,
        _env: &mut Self::InstantiateEnv,
        _meta: &mut Self::Meta,
        _graph: DfirGraph,
        _extra_stmts: Vec<syn::Stmt>,
    ) {
    }
}

#[derive(Clone)]
pub struct SimExternal {
    pub(crate) external_ports: Rc<RefCell<(Vec<usize>, usize)>>,
    pub(crate) registered: RefCell<HashMap<usize, usize>>,
}

impl Node for SimExternal {
    type Port = ();
    type Meta = ();
    type InstantiateEnv = ();

    fn next_port(&self) -> Self::Port {
        todo!()
    }

    fn update_meta(&mut self, _meta: &Self::Meta) {
        todo!()
    }

    fn instantiate(
        &self,
        _env: &mut Self::InstantiateEnv,
        _meta: &mut Self::Meta,
        _graph: DfirGraph,
        _extra_stmts: Vec<syn::Stmt>,
    ) {
    }
}

impl<'a> RegisterPort<'a, SimDeploy> for SimExternal {
    fn register(&self, key: usize, port: usize) {
        self.registered.borrow_mut().insert(key, port);
    }

    fn raw_port(&self, _key: usize) -> () {
        todo!()
    }

    fn as_bytes_bidi(
        &self,
        _key: usize,
    ) -> impl Future<
        Output = DynSourceSink<
            Result<bytes::BytesMut, std::io::Error>,
            bytes::Bytes,
            std::io::Error,
        >,
    > + 'a {
        async { todo!() }
    }

    fn as_bincode_bidi<InT, OutT>(
        &self,
        _key: usize,
    ) -> impl Future<Output = DynSourceSink<OutT, InT, std::io::Error>> + 'a
    where
        InT: serde::Serialize + 'static,
        OutT: serde::de::DeserializeOwned + 'static,
    {
        async { todo!() }
    }

    fn as_bincode_sink<T>(
        &self,
        _key: usize,
    ) -> impl Future<Output = std::pin::Pin<Box<dyn futures::Sink<T, Error = std::io::Error>>>> + 'a
    where
        T: serde::Serialize + 'static,
    {
        async { todo!() }
    }

    fn as_bincode_source<T>(
        &self,
        _key: usize,
    ) -> impl Future<Output = std::pin::Pin<Box<dyn futures::Stream<Item = T>>>> + 'a
    where
        T: serde::de::DeserializeOwned + 'static,
    {
        async { todo!() }
    }
}

struct SimDeploy {}
impl<'a> Deploy<'a> for SimDeploy {
    type InstantiateEnv = ();
    type CompileEnv = ();
    type Process = SimNode;
    type Cluster = SimNode;
    type External = SimExternal;
    type Port = usize;
    type ExternalRawPort = ();
    type Meta = ();
    type GraphId = ();

    fn allocate_process_port(_process: &Self::Process) -> Self::Port {
        0
    }

    fn allocate_cluster_port(_cluster: &Self::Cluster) -> Self::Port {
        todo!()
    }

    fn allocate_external_port(external: &Self::External) -> Self::Port {
        let mut borrowed = external.external_ports.borrow_mut();
        let port_id = borrowed.1;
        borrowed.0.push(port_id);
        borrowed.1 += 1;

        port_id
    }

    fn o2o_sink_source(
        _compile_env: &Self::CompileEnv,
        _p1: &Self::Process,
        _p1_port: &Self::Port,
        _p2: &Self::Process,
        _p2_port: &Self::Port,
    ) -> (syn::Expr, syn::Expr) {
        todo!()
    }

    fn o2o_connect(
        _p1: &Self::Process,
        _p1_port: &Self::Port,
        _p2: &Self::Process,
        _p2_port: &Self::Port,
    ) -> Box<dyn FnOnce()> {
        todo!()
    }

    fn o2m_sink_source(
        _compile_env: &Self::CompileEnv,
        _p1: &Self::Process,
        _p1_port: &Self::Port,
        _c2: &Self::Cluster,
        _c2_port: &Self::Port,
    ) -> (syn::Expr, syn::Expr) {
        todo!()
    }

    fn o2m_connect(
        _p1: &Self::Process,
        _p1_port: &Self::Port,
        _c2: &Self::Cluster,
        _c2_port: &Self::Port,
    ) -> Box<dyn FnOnce()> {
        todo!()
    }

    fn m2o_sink_source(
        _compile_env: &Self::CompileEnv,
        _c1: &Self::Cluster,
        _c1_port: &Self::Port,
        _p2: &Self::Process,
        _p2_port: &Self::Port,
    ) -> (syn::Expr, syn::Expr) {
        todo!()
    }

    fn m2o_connect(
        _c1: &Self::Cluster,
        _c1_port: &Self::Port,
        _p2: &Self::Process,
        _p2_port: &Self::Port,
    ) -> Box<dyn FnOnce()> {
        todo!()
    }

    fn m2m_sink_source(
        _compile_env: &Self::CompileEnv,
        _c1: &Self::Cluster,
        _c1_port: &Self::Port,
        _c2: &Self::Cluster,
        _c2_port: &Self::Port,
    ) -> (syn::Expr, syn::Expr) {
        todo!()
    }

    fn m2m_connect(
        _c1: &Self::Cluster,
        _c1_port: &Self::Port,
        _c2: &Self::Cluster,
        _c2_port: &Self::Port,
    ) -> Box<dyn FnOnce()> {
        todo!()
    }

    fn e2o_many_source(
        _compile_env: &Self::CompileEnv,
        _extra_stmts: &mut Vec<syn::Stmt>,
        _p2: &Self::Process,
        _p2_port: &Self::Port,
        _codec_type: &syn::Type,
        _shared_handle: String,
    ) -> syn::Expr {
        todo!()
    }

    fn e2o_many_sink(_shared_handle: String) -> syn::Expr {
        todo!()
    }

    fn e2o_source(
        _compile_env: &Self::CompileEnv,
        _p1: &Self::External,
        p1_port: &Self::Port,
        _p2: &Self::Process,
        _p2_port: &Self::Port,
    ) -> syn::Expr {
        let ident = syn::Ident::new(
            &format!("__hydro_external_ports"),
            proc_macro2::Span::call_site(),
        );
        syn::parse_quote!(
            Box::<dyn ::std::any::Any>::downcast::<__root_dfir_rs::tokio_stream::wrappers::UnboundedReceiverStream<_>>(
                #ident.remove(&#p1_port).unwrap()
            ).unwrap()
        )
    }

    fn e2o_connect(
        _p1: &Self::External,
        _p1_port: &Self::Port,
        _p2: &Self::Process,
        _p2_port: &Self::Port,
        _many: bool,
        _server_hint: crate::location::NetworkHint,
    ) -> Box<dyn FnOnce()> {
        Box::new(|| {})
    }

    fn o2e_sink(
        _compile_env: &Self::CompileEnv,
        _p1: &Self::Process,
        _p1_port: &Self::Port,
        _p2: &Self::External,
        _p2_port: &Self::Port,
    ) -> syn::Expr {
        todo!()
    }

    fn o2e_connect(
        _p1: &Self::Process,
        _p1_port: &Self::Port,
        _p2: &Self::External,
        _p2_port: &Self::Port,
    ) -> Box<dyn FnOnce()> {
        todo!()
    }

    #[expect(unreachable_code, reason = "todo!() is unreachable")]
    fn cluster_ids(
        _env: &Self::CompileEnv,
        _of_cluster: usize,
    ) -> impl stageleft::QuotedWithContext<'a, &'a [u32], ()> + Copy + 'a {
        todo!();
        stageleft::q!(todo!())
    }

    #[expect(unreachable_code, reason = "todo!() is unreachable")]
    fn cluster_self_id(
        _env: &Self::CompileEnv,
    ) -> impl stageleft::QuotedWithContext<'a, u32, ()> + Copy + 'a {
        todo!();
        stageleft::q!(todo!())
    }
}

pub struct SimFlow<'a> {
    // We need to grab an `&mut` reference to the IR in `preview_compile` even though
    // that function does not modify the IR. Using an `UnsafeCell` allows us to do this
    // while still being able to lend out immutable references to the IR.
    pub(crate) ir: UnsafeCell<Vec<HydroRoot>>,

    pub(crate) external_ports: Rc<RefCell<(Vec<usize>, usize)>>,

    pub(crate) processes: HashMap<usize, SimNode>,
    pub(crate) clusters: HashMap<usize, SimNode>,
    pub(crate) externals: HashMap<usize, SimExternal>,

    /// Lists all the processes that were created in the flow, same ID as `processes`
    /// but with the type name of the tag.
    pub(crate) _process_id_name: Vec<(usize, String)>,
    pub(crate) _external_id_name: Vec<(usize, String)>,
    pub(crate) _cluster_id_name: Vec<(usize, String)>,

    pub(crate) _phantom: Invariant<'a>,
}

impl<'a> SimFlow<'a> {
    pub fn ir(&self) -> &Vec<HydroRoot> {
        unsafe {
            // SAFETY: even when we grab this as mutable in `preview_compile`, we do not modify it
            &*self.ir.get()
        }
    }

    pub fn with_instance<T>(self, thunk: impl FnOnce(CompiledSimInstance) -> T) -> T {
        self.compiled().with_instance(thunk)
    }

    pub fn compiled(mut self) -> CompiledSim {
        use std::collections::BTreeMap;

        use dfir_lang::graph::{eliminate_extra_unions_tees, partition_graph};

        let mut sim_emit = SimBuilder {
            async_level: FlatGraphBuilder::new(),
        };

        let mut seen_tees_instantiate: HashMap<_, _> = HashMap::new();
        let mut extra_stmts = BTreeMap::new();
        self.ir.get_mut().iter_mut().for_each(|leaf| {
            leaf.compile_network::<SimDeploy>(
                &(),
                &mut extra_stmts,
                &mut seen_tees_instantiate,
                &self.processes,
                &self.clusters,
                &self.externals,
            );
        });

        let mut built_tees = HashMap::new();
        let mut next_stmt_id = 0;
        for leaf in self.ir.get_mut() {
            leaf.emit(&mut sim_emit, &mut built_tees, &mut next_stmt_id);
        }

        let (mut async_level_flat_graph, _, _) = sim_emit.async_level.build();
        eliminate_extra_unions_tees(&mut async_level_flat_graph);
        let async_level_graph =
            partition_graph(async_level_flat_graph).expect("Failed to partition (cycle detected).");

        // let mut extra_stmts = BTreeMap::new();
        // self.cluster_id_stmts(&mut extra_stmts, &());

        let (bin, trybuild) = create_sim_graph_trybuild(async_level_graph, vec![]);

        let out = compile_sim(bin, trybuild).unwrap();
        let lib = unsafe { Library::new(out).unwrap() };

        let external_ports = self.external_ports.take().0;
        CompiledSim {
            lib,
            external_ports,
        }
    }
}

#[cfg(stageleft_runtime)]
fn compile_sim(bin: String, trybuild: TrybuildConfig) -> Result<std::path::PathBuf, ()> {
    let mut command = Command::new("cargo");
    command.current_dir(&trybuild.project_dir);
    command.args(["rustc"]);
    command.args(["--lib"]);
    command.args(["--target-dir", trybuild.target_dir.to_str().unwrap()]);
    if let Some(features) = &trybuild.features {
        command.args(["--features", &features.join(",")]);
    }
    // command.args(["--target", "aarch64-apple-darwin"]);
    command.args(["--config", "build.incremental = false"]);
    command.args(["--crate-type", "cdylib"]);
    command.arg("--message-format=json-diagnostic-rendered-ansi");
    command.env("STAGELEFT_TRYBUILD_BUILD_STAGED", "1");
    command.env("TRYBUILD_LIB_NAME", &bin);
    // command.args(["--", "-Cllvm-args=-fsanitize=fuzzer-no-link"]);
    command.env_remove("CARGO_CFG_FUZZING_LIBFUZZER");
    command.env_remove("CARGO_CFG_FUZZING_LIBFUZZER_AFL");
    command.args([
        "--",
        "-Clink-arg=-undefined",
        "-Clink-arg=dynamic_lookup",
        "-Cpasses=sancov-module",
        "-Cllvm-args=-sanitizer-coverage-inline-8bit-counters",
        "-Cllvm-args=-sanitizer-coverage-level=4",
        "-Cllvm-args=-sanitizer-coverage-pc-table",
        "-Cllvm-args=-sanitizer-coverage-trace-compares",
    ]);
    let mut spawned = command
        .stdout(Stdio::piped())
        .stdin(Stdio::null())
        .spawn()
        .unwrap();
    let reader = std::io::BufReader::new(spawned.stdout.take().unwrap());

    let mut out = Err(());
    for message in cargo_metadata::Message::parse_stream(reader) {
        match message.unwrap() {
            cargo_metadata::Message::CompilerArtifact(artifact) => {
                let is_output = artifact.target.crate_types.contains(&"cdylib".to_string());

                if is_output {
                    use std::path::PathBuf;

                    let path = artifact.filenames.get(0).unwrap();
                    let path_buf: PathBuf = path.clone().into();
                    out = Ok(path_buf);
                }
            }
            cargo_metadata::Message::CompilerMessage(mut msg) => {
                // Update the path displayed to enable clicking in IDE.
                {
                    let full_path =
                        format!("(full path) {}", trybuild.project_dir.join("src").display());
                    if let Some(rendered) = msg.message.rendered.as_mut() {
                        *rendered = rendered.replace("src", &full_path);
                    }
                }

                eprintln!("{}", msg.message.to_string());
            }
            cargo_metadata::Message::TextLine(line) => {
                eprintln!("{}", line);
            }
            cargo_metadata::Message::BuildFinished(_) => {}
            cargo_metadata::Message::BuildScriptExecuted(_) => {}
            msg => panic!("Unexpected message type: {:?}", msg),
        }
    }

    spawned.wait().unwrap();
    out
}

#[cfg(stageleft_runtime)]
fn create_sim_graph_trybuild(
    graph: DfirGraph,
    extra_stmts: Vec<syn::Stmt>,
) -> (String, TrybuildConfig) {
    let source_dir = cargo::manifest_dir().unwrap();
    let source_manifest = dependencies::get_manifest(&source_dir).unwrap();
    let crate_name = &source_manifest.package.name.to_string().replace("-", "_");

    let is_test = IS_TEST.load(std::sync::atomic::Ordering::Relaxed);

    let generated_code =
        compile_sim_graph_trybuild(graph, extra_stmts, crate_name.clone(), is_test);

    let inlined_staged: syn::File = if is_test {
        let gen_staged = stageleft_tool::gen_staged_trybuild(
            &path!(source_dir / "src" / "lib.rs"),
            &path!(source_dir / "Cargo.toml"),
            crate_name.clone(),
            is_test,
        );

        syn::parse_quote! {
            #[allow(
                unused,
                ambiguous_glob_reexports,
                clippy::suspicious_else_formatting,
                unexpected_cfgs,
                reason = "generated code"
            )]
            pub mod __staged {
                #gen_staged
            }
        }
    } else {
        let crate_name_ident = syn::Ident::new(crate_name, proc_macro2::Span::call_site());
        syn::parse_quote!(
            pub use #crate_name_ident::__staged;
        )
    };

    let source = prettyplease::unparse(&syn::parse_quote! {
        #generated_code

        #inlined_staged
    });

    let hash = format!("{:X}", Sha256::digest(&source))
        .chars()
        .take(8)
        .collect::<String>();

    let bin_name = hash;

    let (project_dir, target_dir, mut cur_bin_enabled_features) = create_trybuild().unwrap();

    // TODO(shadaj): garbage collect this directory occasionally
    fs::create_dir_all(path!(project_dir / "src")).unwrap();

    let out_path = path!(project_dir / "src" / format!("{bin_name}.rs"));
    {
        let _concurrent_test_lock = CONCURRENT_TEST_LOCK.lock().unwrap();
        write_atomic(source.as_ref(), &out_path).unwrap();
    }

    let lib_path = path!(project_dir / "src" / format!("lib.rs"));
    {
        let _concurrent_test_lock = CONCURRENT_TEST_LOCK.lock().unwrap();
        write_atomic("#![allow(unused_imports, unused_crate_dependencies, missing_docs, non_snake_case)]\ninclude!(std::concat!(env!(\"TRYBUILD_LIB_NAME\"), \".rs\"));".as_bytes(), &lib_path).unwrap();
    }

    if is_test {
        if cur_bin_enabled_features.is_none() {
            cur_bin_enabled_features = Some(vec![]);
        }

        cur_bin_enabled_features
            .as_mut()
            .unwrap()
            .push("hydro___test".to_string());
    }

    (
        bin_name,
        TrybuildConfig {
            project_dir,
            target_dir,
            features: cur_bin_enabled_features,
        },
    )
}

#[cfg(stageleft_runtime)]
fn compile_sim_graph_trybuild(
    partitioned_graph: DfirGraph,
    extra_stmts: Vec<syn::Stmt>,
    crate_name: String,
    is_test: bool,
) -> syn::File {
    let mut diagnostics = Vec::new();
    let mut dfir_expr: syn::Expr = syn::parse2(partitioned_graph.as_code(
        &quote! { __root_dfir_rs },
        true,
        quote!(),
        &mut diagnostics,
    ))
    .unwrap();

    if is_test {
        UseTestModeStaged {
            crate_name: crate_name.clone(),
        }
        .visit_expr_mut(&mut dfir_expr);
    }

    let source_ast: syn::File = syn::parse_quote! {
        use hydro_lang::prelude::*;
        use hydro_lang::runtime_support::dfir_rs as __root_dfir_rs;

        #[allow(unused)]
        fn __hydro_runtime_core<'a>(mut __hydro_external_ports: ::std::collections::HashMap<usize, Box<dyn std::any::Any>>) -> hydro_lang::runtime_support::dfir_rs::scheduled::graph::Dfir<'a> {
            #(#extra_stmts)*
            #dfir_expr
        }

        #[unsafe(no_mangle)]
        unsafe extern "Rust" fn __hydro_runtime(__hydro_external_ports: ::std::collections::HashMap<usize, Box<dyn std::any::Any>>) -> hydro_lang::runtime_support::dfir_rs::scheduled::graph::Dfir<'static> {
            __hydro_runtime_core(__hydro_external_ports)
        }
    };
    source_ast
}
