use std::cell::RefCell;
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
use crate::deploy::trybuild::{
    CONCURRENT_TEST_LOCK, IS_TEST, TrybuildConfig, create_trybuild, write_atomic,
};
use crate::deploy::trybuild_rewriters::UseTestModeStaged;
use crate::location::dynamic::LocationId;

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
    command.args(["--crate-type", "dylib"]);
    command.arg("--message-format=json-diagnostic-rendered-ansi");
    command.env("STAGELEFT_TRYBUILD_BUILD_STAGED", "1");
    command.env("TRYBUILD_LIB_NAME", &bin);

    if let Ok(fuzzer) = std::env::var("BOLERO_FUZZER") {
        command.env_remove("BOLERO_FUZZER");
        command.env_remove("RUSTFLAGS");

        if fuzzer == "libfuzzer" {
            command.args([
                "--",
                "-Cprefer-dynamic",
                "-Clink-arg=-undefined",
                "-Clink-arg=dynamic_lookup",
                "-Cpasses=sancov-module",
                "-Cllvm-args=-sanitizer-coverage-inline-8bit-counters",
                "-Cllvm-args=-sanitizer-coverage-level=4",
                "-Cllvm-args=-sanitizer-coverage-pc-table",
                "-Cllvm-args=-sanitizer-coverage-trace-compares",
            ]);
        }
    } else if IS_TEST.load(std::sync::atomic::Ordering::Relaxed) {
        command.env("RUSTFLAGS", "-C prefer-dynamic");
        command.args(["--", "-C rpath"]);
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
                let is_output = artifact.target.crate_types.contains(&"dylib".to_string());

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
    graph: DfirGraph,
    tick_graphs: BTreeMap<LocationId, DfirGraph>,
    extra_stmts: Vec<syn::Stmt>,
) -> (String, TrybuildConfig) {
    let source_dir = cargo::manifest_dir().unwrap();
    let source_manifest = dependencies::get_manifest(&source_dir).unwrap();
    let crate_name = &source_manifest.package.name.to_string().replace("-", "_");

    let is_test = IS_TEST.load(std::sync::atomic::Ordering::Relaxed);

    let generated_code =
        compile_sim_graph_trybuild(graph, tick_graphs, extra_stmts, crate_name.clone(), is_test);

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
        let staged_path = path!(project_dir / "src" / format!("__staged.rs"));
        {
            let _concurrent_test_lock = CONCURRENT_TEST_LOCK.lock().unwrap();
            write_atomic(inlined_staged.as_bytes(), &staged_path).unwrap();
        }
    }

    let crate_name_ident = syn::Ident::new(crate_name, Span::call_site());
    let lib_path = path!(project_dir / "src" / format!("lib.rs"));
    {
        let _concurrent_test_lock = CONCURRENT_TEST_LOCK.lock().unwrap();
        write_atomic(
            prettyplease::unparse(&syn::parse_quote! {
                #![allow(unused_imports, unused_crate_dependencies, missing_docs, non_snake_case)]

                #[cfg(feature = "hydro___test")]
                pub mod __staged;

                #[cfg(not(feature = "hydro___test"))]
                pub use #crate_name_ident::__staged;
            })
            .as_bytes(),
            &lib_path,
        )
        .unwrap();
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

fn compile_sim_graph_trybuild(
    partitioned_graph: DfirGraph,
    tick_graphs: BTreeMap<LocationId, DfirGraph>,
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

    let tick_dfir_epxrs = tick_graphs
        .into_iter()
        .map(|(lid, g)| {
            let mut dfir_expr: syn::Expr = syn::parse2(g.as_code(
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

            let ser_lid = serde_json::to_string(&lid).unwrap();
            syn::parse_quote!((#ser_lid, #dfir_expr))
        })
        .collect::<Vec<syn::Expr>>();

    let trybuild_crate_name = format!("{}_hydro_trybuild", crate_name);
    let trybuild_crate_name_ident = syn::Ident::new(&trybuild_crate_name, Span::call_site());

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
            hydro_lang::runtime_support::dfir_rs::scheduled::graph::Dfir<'a>,
            Vec<(&'static str, __root_dfir_rs::scheduled::graph::Dfir<'a>)>,
            ::std::collections::HashMap<&'static str, ::std::vec::Vec<Box<dyn hydro_lang::sim::runtime::SimHook>>>,
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

            let mut __hydro_hooks: ::std::collections::HashMap<&'static str, ::std::vec::Vec<Box<dyn hydro_lang::sim::runtime::SimHook>>> = ::std::collections::HashMap::new();
            #(#extra_stmts)*
            (#dfir_expr, vec![#(#tick_dfir_epxrs),*], __hydro_hooks)
        }

        #[unsafe(no_mangle)]
        unsafe extern "Rust" fn __hydro_runtime(
            should_color: bool,
            __hydro_external_out: ::std::collections::HashMap<usize, __root_dfir_rs::tokio::sync::mpsc::UnboundedSender<__root_dfir_rs::bytes::Bytes>>,
            __hydro_external_in: ::std::collections::HashMap<usize, __root_dfir_rs::tokio_stream::wrappers::UnboundedReceiverStream<__root_dfir_rs::bytes::Bytes>>,
            __println_handler: fn(::std::fmt::Arguments<'_>),
            __eprintln_handler: fn(::std::fmt::Arguments<'_>),
        ) -> (
            __root_dfir_rs::scheduled::graph::Dfir<'static>,
            Vec<(&'static str, __root_dfir_rs::scheduled::graph::Dfir<'static>)>,
            ::std::collections::HashMap<&'static str, ::std::vec::Vec<Box<dyn hydro_lang::sim::runtime::SimHook>>>,
        ) {
            hydro_lang::runtime_support::colored::control::set_override(should_color);
            __hydro_runtime_core(__hydro_external_out, __hydro_external_in, __println_handler, __eprintln_handler)
        }
    };
    source_ast
}
