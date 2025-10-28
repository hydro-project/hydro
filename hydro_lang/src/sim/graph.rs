use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::process::{Command, Stdio};
use std::rc::Rc;

use dfir_lang::graph::DfirGraph;
use proc_macro2::Span;
use quote::quote;
use sha2::{Digest, Sha256};
use syn::visit_mut::VisitMut;
use tempfile::TempPath;
use trybuild_internals_api::{cargo, dependencies, path};

use crate::compile::deploy_provider::{Deploy, DynSourceSink, Node, RegisterPort};
use crate::compile::trybuild::generate::{
    CONCURRENT_TEST_LOCK, IS_TEST, TrybuildConfig, create_trybuild, write_atomic,
};
use crate::compile::trybuild::rewriters::UseTestModeStaged;
use crate::location::dynamic::LocationId;

#[derive(Clone)]
pub struct SimNode {
    /// Counter for port IDs, must be global across all nodes to prevent collisions.
    pub port_counter: Rc<Cell<usize>>,
}

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

    #[expect(clippy::manual_async_fn, reason = "false positive, involves lifetimes")]
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

    #[expect(clippy::manual_async_fn, reason = "false positive, involves lifetimes")]
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

    #[expect(clippy::manual_async_fn, reason = "false positive, involves lifetimes")]
    fn as_bincode_sink<T>(
        &self,
        _key: usize,
    ) -> impl Future<Output = std::pin::Pin<Box<dyn futures::Sink<T, Error = std::io::Error>>>> + 'a
    where
        T: serde::Serialize + 'static,
    {
        async { todo!() }
    }

    #[expect(clippy::manual_async_fn, reason = "false positive, involves lifetimes")]
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

pub(super) struct SimDeploy {}
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

    fn allocate_process_port(process: &Self::Process) -> Self::Port {
        let port_id = process.port_counter.get();
        process.port_counter.set(port_id + 1);
        port_id
    }

    fn allocate_cluster_port(cluster: &Self::Cluster) -> Self::Port {
        let port_id = cluster.port_counter.get();
        cluster.port_counter.set(port_id + 1);
        port_id
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
        p1_port: &Self::Port,
        _p2: &Self::Process,
        p2_port: &Self::Port,
    ) -> (syn::Expr, syn::Expr) {
        let ident_sink =
            syn::Ident::new(&format!("__hydro_o2o_sink_{}", p1_port), Span::call_site());
        let ident_source = syn::Ident::new(
            &format!("__hydro_o2o_source_{}", p2_port),
            Span::call_site(),
        );
        (
            syn::parse_quote!(#ident_sink),
            syn::parse_quote!(#ident_source),
        )
    }

    fn o2o_connect(
        _p1: &Self::Process,
        _p1_port: &Self::Port,
        _p2: &Self::Process,
        _p2_port: &Self::Port,
    ) -> Box<dyn FnOnce()> {
        Box::new(|| {})
    }

    fn o2m_sink_source(
        _compile_env: &Self::CompileEnv,
        _p1: &Self::Process,
        p1_port: &Self::Port,
        _c2: &Self::Cluster,
        c2_port: &Self::Port,
    ) -> (syn::Expr, syn::Expr) {
        let ident_sink =
            syn::Ident::new(&format!("__hydro_o2m_sink_{}", p1_port), Span::call_site());
        let ident_source = syn::Ident::new(
            &format!("__hydro_o2m_source_{}", c2_port),
            Span::call_site(),
        );
        (
            syn::parse_quote!(#ident_sink),
            syn::parse_quote!(#ident_source),
        )
    }

    fn o2m_connect(
        _p1: &Self::Process,
        _p1_port: &Self::Port,
        _c2: &Self::Cluster,
        _c2_port: &Self::Port,
    ) -> Box<dyn FnOnce()> {
        Box::new(|| {})
    }

    fn m2o_sink_source(
        _compile_env: &Self::CompileEnv,
        _c1: &Self::Cluster,
        c1_port: &Self::Port,
        _p2: &Self::Process,
        p2_port: &Self::Port,
    ) -> (syn::Expr, syn::Expr) {
        let ident_sink =
            syn::Ident::new(&format!("__hydro_m2o_sink_{}", c1_port), Span::call_site());
        let ident_source = syn::Ident::new(
            &format!("__hydro_m2o_source_{}", p2_port),
            Span::call_site(),
        );
        (
            syn::parse_quote!(#ident_sink),
            syn::parse_quote!(#ident_source),
        )
    }

    fn m2o_connect(
        _c1: &Self::Cluster,
        _c1_port: &Self::Port,
        _p2: &Self::Process,
        _p2_port: &Self::Port,
    ) -> Box<dyn FnOnce()> {
        Box::new(|| {})
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
        let ident = syn::Ident::new("__hydro_external_in", Span::call_site());
        syn::parse_quote!(
            #ident.remove(&#p1_port).unwrap()
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
        p2_port: &Self::Port,
    ) -> syn::Expr {
        let ident = syn::Ident::new("__hydro_external_out", Span::call_site());
        syn::parse_quote!(
            #ident.remove(&#p2_port).unwrap()
        )
    }

    fn o2e_connect(
        _p1: &Self::Process,
        _p1_port: &Self::Port,
        _p2: &Self::External,
        _p2_port: &Self::Port,
    ) -> Box<dyn FnOnce()> {
        Box::new(|| {})
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

pub(super) fn compile_sim(bin: String, trybuild: TrybuildConfig) -> Result<TempPath, ()> {
    let mut command = Command::new("cargo");
    command.current_dir(&trybuild.project_dir);
    command.args(["rustc", "--locked"]);
    command.args(["--example", "sim-dylib"]);
    command.args(["--target-dir", trybuild.target_dir.to_str().unwrap()]);
    if let Some(features) = &trybuild.features {
        command.args(["--features", &features.join(",")]);
    }
    command.args(["--config", "build.incremental = false"]);
    command.args(["--crate-type", "cdylib"]);
    command.arg("--message-format=json-diagnostic-rendered-ansi");
    command.env("STAGELEFT_TRYBUILD_BUILD_STAGED", "1");
    command.env("TRYBUILD_LIB_NAME", &bin);

    command.arg("--");

    let is_fuzz = std::env::var("BOLERO_FUZZER").is_ok();
    if is_fuzz {
        command.env(
            "RUSTFLAGS",
            std::env::var("RUSTFLAGS_OUTER").unwrap_or_default() + " -C prefer-dynamic",
        );
    } else {
        command.env(
            "RUSTFLAGS",
            std::env::var("RUSTFLAGS").unwrap_or_default() + " -C prefer-dynamic",
        );
    }

    if cfg!(target_os = "linux") {
        let debug_path = if let Ok(target) = std::env::var("CARGO_BUILD_TARGET")
            && !is_fuzz
        {
            path!(trybuild.target_dir / target / "debug")
        } else {
            path!(trybuild.target_dir / "debug")
        };

        command.args([&format!(
            "-Clink-arg=-Wl,-rpath,{}",
            debug_path.to_str().unwrap()
        )]);

        if cfg!(target_env = "gnu") {
            command.arg(
                // https://github.com/rust-lang/rust/issues/91979
                "-Clink-args=-Wl,-z,nodelete",
            );
        }
    }

    if let Ok(fuzzer) = std::env::var("BOLERO_FUZZER") {
        command.env_remove("BOLERO_FUZZER");
        command.env_remove("CARGO_BUILD_TARGET");

        if fuzzer == "libfuzzer" {
            #[cfg(target_os = "macos")]
            {
                command.args(["-Clink-arg=-undefined", "-Clink-arg=dynamic_lookup"]);
            }

            #[cfg(target_os = "linux")]
            {
                command.args(["-Clink-arg=-Wl,--unresolved-symbols=ignore-all"]);
            }

            command.args([
                "-Cpasses=sancov-module",
                "-Cllvm-args=-sanitizer-coverage-inline-8bit-counters",
                "-Cllvm-args=-sanitizer-coverage-level=4",
                "-Cllvm-args=-sanitizer-coverage-pc-table",
                "-Cllvm-args=-sanitizer-coverage-trace-compares",
            ]);
        }
    }

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
                // unlike dylib, cdylib only exports the explicitly exported symbols
                let is_output = artifact.target.is_example();

                if is_output {
                    use std::path::PathBuf;

                    let path = artifact.filenames.first().unwrap();
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

                eprintln!("{}", msg.message);
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

    let out_file = tempfile::NamedTempFile::new().unwrap().into_temp_path();
    fs::copy(out.as_ref().unwrap(), &out_file).unwrap();
    Ok(out_file)
}

pub(super) fn create_sim_graph_trybuild(
    process_graphs: BTreeMap<LocationId, DfirGraph>,
    cluster_graphs: BTreeMap<LocationId, DfirGraph>,
    cluster_max_sizes: HashMap<LocationId, usize>,
    process_tick_graphs: BTreeMap<LocationId, DfirGraph>,
    cluster_tick_graphs: BTreeMap<LocationId, DfirGraph>,
    extra_stmts_global: Vec<syn::Stmt>,
    extra_stmts_cluster: BTreeMap<LocationId, Vec<syn::Stmt>>,
) -> (String, TrybuildConfig) {
    let source_dir = cargo::manifest_dir().unwrap();
    let source_manifest = dependencies::get_manifest(&source_dir).unwrap();
    let crate_name = &source_manifest.package.name.to_string().replace("-", "_");

    let is_test = IS_TEST.load(std::sync::atomic::Ordering::Relaxed);

    let generated_code = compile_sim_graph_trybuild(
        process_graphs,
        cluster_graphs,
        cluster_max_sizes,
        process_tick_graphs,
        cluster_tick_graphs,
        extra_stmts_global,
        extra_stmts_cluster,
        crate_name.clone(),
        is_test,
    );

    let inlined_staged = if is_test {
        let gen_staged = stageleft_tool::gen_staged_trybuild(
            &path!(source_dir / "src" / "lib.rs"),
            &path!(source_dir / "Cargo.toml"),
            crate_name.clone(),
            Some("hydro___test".to_string()),
        );

        Some(prettyplease::unparse(&syn::parse_quote! {
            #![allow(
                unused,
                ambiguous_glob_reexports,
                clippy::suspicious_else_formatting,
                unexpected_cfgs,
                reason = "generated code"
            )]

            #gen_staged
        }))
    } else {
        None
    };

    let source = prettyplease::unparse(&generated_code);

    let hash = format!("{:X}", Sha256::digest(&source))
        .chars()
        .take(8)
        .collect::<String>();

    let bin_name = hash;

    let (project_dir, target_dir, mut cur_bin_enabled_features) = create_trybuild().unwrap();

    // TODO(shadaj): garbage collect this directory occasionally
    fs::create_dir_all(path!(project_dir / "src")).unwrap();
    fs::create_dir_all(path!(project_dir / "examples")).unwrap();

    let out_path = path!(project_dir / "examples" / format!("{bin_name}.rs"));
    {
        let _concurrent_test_lock = CONCURRENT_TEST_LOCK.lock().unwrap();
        write_atomic(source.as_ref(), &out_path).unwrap();
    }

    if let Some(inlined_staged) = inlined_staged {
        let staged_path = path!(project_dir / "src" / "__staged.rs");
        {
            let _concurrent_test_lock = CONCURRENT_TEST_LOCK.lock().unwrap();
            write_atomic(inlined_staged.as_bytes(), &staged_path).unwrap();
        }
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

#[expect(clippy::too_many_arguments, reason = "necessary for code generation")]
fn compile_sim_graph_trybuild(
    process_graphs: BTreeMap<LocationId, DfirGraph>,
    cluster_graphs: BTreeMap<LocationId, DfirGraph>,
    cluster_max_sizes: HashMap<LocationId, usize>,
    process_tick_graphs: BTreeMap<LocationId, DfirGraph>,
    cluster_tick_graphs: BTreeMap<LocationId, DfirGraph>,
    extra_stmts_global: Vec<syn::Stmt>,
    extra_stmts_cluster: BTreeMap<LocationId, Vec<syn::Stmt>>,
    crate_name: String,
    is_test: bool,
) -> syn::File {
    let mut diagnostics = Vec::new();

    let mut dfir_into_code = |g: &DfirGraph| {
        let mut dfir_expr: syn::Expr =
            syn::parse2(g.as_code(&quote! { __root_dfir_rs }, true, quote!(), &mut diagnostics))
                .unwrap();

        if is_test {
            UseTestModeStaged {
                crate_name: crate_name.clone(),
            }
            .visit_expr_mut(&mut dfir_expr);
        }

        dfir_expr
    };

    let process_dfir_exprs = process_graphs
        .into_iter()
        .map(|(lid, g)| {
            let dfir_expr = dfir_into_code(&g);
            let ser_lid = serde_json::to_string(&lid).unwrap();
            syn::parse_quote!((#ser_lid, None, #dfir_expr))
        })
        .collect::<Vec<syn::Expr>>();

    let mut cluster_ticks_grouped_by_root = cluster_tick_graphs.into_iter().fold::<BTreeMap<
        LocationId,
        Vec<(LocationId, DfirGraph)>,
    >, _>(
        BTreeMap::new(),
        |mut acc, (lid, g)| {
            let root = lid.root();
            acc.entry(root.clone()).or_default().push((lid, g));
            acc
        },
    );

    let cluster_dfir_stmts = cluster_graphs
        .into_iter()
        .map(|(lid, g)| {
            let dfir_expr = dfir_into_code(&g);

            let tick_dfir_stmts = cluster_ticks_grouped_by_root
                .remove(&lid)
                .unwrap_or_default()
                .into_iter()
                .map(|(tick_lid, tick_g)| {
                    let tick_dfir_expr = dfir_into_code(&tick_g);
                    let ser_tick_lid = serde_json::to_string(&tick_lid).unwrap();
                    syn::parse_quote! {
                        __tick_dfirs.push((
                            #ser_tick_lid,
                            Some(__current_cluster_id),
                            #tick_dfir_expr
                        ));
                    }
                })
                .collect::<Vec<syn::Stmt>>();

            let ser_lid = serde_json::to_string(&lid).unwrap();
            let extra_stmts_per_cluster =
                extra_stmts_cluster.get(&lid).cloned().unwrap_or_default();
            let max_size = cluster_max_sizes.get(&lid).cloned().unwrap() as u32;

            let cid = if let LocationId::Cluster(cid) = lid {
                cid
            } else {
                unreachable!()
            };

            let self_id_ident = syn::Ident::new(
                &format!("__hydro_lang_cluster_self_id_{}", cid),
                Span::call_site(),
            );

            syn::parse_quote! {
                for __current_cluster_id in 0..#max_size {
                    __async_dfirs.push((
                        #ser_lid,
                        Some(__current_cluster_id),
                        {
                            #(#extra_stmts_per_cluster)*
                            let #self_id_ident = __current_cluster_id;

                            #(#tick_dfir_stmts)*

                            #dfir_expr
                        }
                    ));
                }
            }
        })
        .collect::<Vec<syn::Stmt>>();

    let process_tick_dfir_exprs = process_tick_graphs
        .into_iter()
        .map(|(lid, g)| {
            let dfir_expr = dfir_into_code(&g);
            let ser_lid = serde_json::to_string(&lid).unwrap();
            syn::parse_quote!((#ser_lid, None, #dfir_expr))
        })
        .collect::<Vec<syn::Expr>>();

    let trybuild_crate_name_ident = quote::format_ident!("{}_hydro_trybuild", crate_name);

    let cluster_ids_stmts = cluster_max_sizes
        .iter()
        .map(|(lid, max_size)| {
            let ident = syn::Ident::new(
                &format!(
                    "__hydro_lang_cluster_ids_{}",
                    match lid {
                        LocationId::Cluster(cid) => cid.to_string(),
                        _ => panic!("Expected cluster location ID"),
                    }
                ),
                Span::call_site(),
            );

            let elements = (0..*max_size as u32)
                .map(|i| syn::parse_quote! { #i })
                .collect::<Vec<syn::Expr>>();

            syn::parse_quote! {
                let #ident: &'static [u32] = &[#(#elements),*];
            }
        })
        .collect::<Vec<syn::Stmt>>();

    let source_ast: syn::File = syn::parse_quote! {
        use hydro_lang::prelude::*;
        use hydro_lang::runtime_support::dfir_rs as __root_dfir_rs;
        pub use #trybuild_crate_name_ident::__staged;

        #[allow(unused)]
        fn __hydro_runtime_core<'a>(
            mut __hydro_external_out: ::std::collections::HashMap<usize, __root_dfir_rs::tokio::sync::mpsc::UnboundedSender<__root_dfir_rs::bytes::Bytes>>,
            mut __hydro_external_in: ::std::collections::HashMap<usize, __root_dfir_rs::tokio_stream::wrappers::UnboundedReceiverStream<__root_dfir_rs::bytes::Bytes>>,
            __println_handler: fn(::std::fmt::Arguments<'_>),
            __eprintln_handler: fn(::std::fmt::Arguments<'_>),
        ) -> (
            Vec<(&'static str, Option<u32>, __root_dfir_rs::scheduled::graph::Dfir<'a>)>,
            Vec<(&'static str, Option<u32>, __root_dfir_rs::scheduled::graph::Dfir<'a>)>,
            ::std::collections::HashMap<(&'static str, Option<u32>), ::std::vec::Vec<Box<dyn hydro_lang::sim::runtime::SimHook>>>,
        ) {
            macro_rules! println {
                ($($arg:tt)*) => ({
                    __println_handler(::std::format_args!($($arg)*));
                })
            }

            macro_rules! eprintln {
                ($($arg:tt)*) => ({
                    __eprintln_handler(::std::format_args!($($arg)*));
                })
            }

            // copy-pasted from std::dbg! so we can use the local eprintln! above
            macro_rules! dbg {
                // NOTE: We cannot use `concat!` to make a static string as a format argument
                // of `eprintln!` because `file!` could contain a `{` or
                // `$val` expression could be a block (`{ .. }`), in which case the `eprintln!`
                // will be malformed.
                () => {
                    eprintln!("[{}:{}:{}]", ::std::file!(), ::std::line!(), ::std::column!())
                };
                ($val:expr $(,)?) => {
                    // Use of `match` here is intentional because it affects the lifetimes
                    // of temporaries - https://stackoverflow.com/a/48732525/1063961
                    match $val {
                        tmp => {
                            eprintln!("[{}:{}:{}] {} = {:#?}",
                                ::std::file!(),
                                ::std::line!(),
                                ::std::column!(),
                                ::std::stringify!($val),
                                // The `&T: Debug` check happens here (not in the format literal desugaring)
                                // to avoid format literal related messages and suggestions.
                                &&tmp as &dyn ::std::fmt::Debug,
                            );
                            tmp
                        }
                    }
                };
                ($($val:expr),+ $(,)?) => {
                    ($(dbg!($val)),+,)
                };
            }

            let mut __hydro_hooks: ::std::collections::HashMap<(&'static str, Option<u32>), ::std::vec::Vec<Box<dyn hydro_lang::sim::runtime::SimHook>>> = ::std::collections::HashMap::new();
            #(#extra_stmts_global)*
            #(#cluster_ids_stmts)*

            let mut __async_dfirs = vec![#(#process_dfir_exprs),*];
            let mut __tick_dfirs = vec![#(#process_tick_dfir_exprs),*];
            #(#cluster_dfir_stmts)*
            (__async_dfirs, __tick_dfirs, __hydro_hooks)
        }

        #[unsafe(no_mangle)]
        unsafe extern "Rust" fn __hydro_runtime(
            should_color: bool,
            __hydro_external_out: ::std::collections::HashMap<usize, __root_dfir_rs::tokio::sync::mpsc::UnboundedSender<__root_dfir_rs::bytes::Bytes>>,
            __hydro_external_in: ::std::collections::HashMap<usize, __root_dfir_rs::tokio_stream::wrappers::UnboundedReceiverStream<__root_dfir_rs::bytes::Bytes>>,
            __println_handler: fn(::std::fmt::Arguments<'_>),
            __eprintln_handler: fn(::std::fmt::Arguments<'_>),
        ) -> (
            Vec<(&'static str, Option<u32>, __root_dfir_rs::scheduled::graph::Dfir<'static>)>,
            Vec<(&'static str, Option<u32>, __root_dfir_rs::scheduled::graph::Dfir<'static>)>,
            ::std::collections::HashMap<(&'static str, Option<u32>), ::std::vec::Vec<Box<dyn hydro_lang::sim::runtime::SimHook>>>,
        ) {
            hydro_lang::runtime_support::colored::control::set_override(should_color);
            __hydro_runtime_core(__hydro_external_out, __hydro_external_in, __println_handler, __eprintln_handler)
        }
    };
    source_ast
}
