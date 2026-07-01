use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

#[cfg(any(feature = "deploy", feature = "maelstrom"))]
use dfir_lang::diagnostic::Diagnostics;
#[cfg(any(feature = "deploy", feature = "maelstrom"))]
use dfir_lang::graph::DfirGraph;
use sha2::{Digest, Sha256};
#[cfg(any(feature = "deploy", feature = "maelstrom"))]
use stageleft::internal::quote;
use trybuild_internals_api::cargo::{self, Metadata};
use trybuild_internals_api::env::Update;
use trybuild_internals_api::run::{PathDependency, Project};
use trybuild_internals_api::{Runner, dependencies, features, path};

pub const HYDRO_RUNTIME_FEATURES: &[&str] = &[
    "deploy_integration",
    "runtime_measure",
    "docker_runtime",
    "ecs_runtime",
    "maelstrom_runtime",
    "sim_runtime",
];

#[cfg(any(feature = "deploy", feature = "maelstrom"))]
/// Whether to use dynamic linking for the generated binary.
/// - `Static`: Place in base crate examples (for remote/containerized deploys)
/// - `Dynamic`: Place in dylib crate examples (for sim and localhost deploys)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkingMode {
    // `Static` is only constructed by the deploy backends; Maelstrom-only builds
    // always use `Dynamic`.
    #[cfg_attr(
        not(feature = "deploy"),
        expect(
            dead_code,
            reason = "only constructed by the deploy backends; Maelstrom-only builds use Dynamic"
        )
    )]
    Static,
    #[cfg(any(feature = "deploy", feature = "maelstrom"))]
    Dynamic,
}

#[cfg(any(feature = "deploy", feature = "maelstrom"))]
/// The deployment mode for code generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeployMode {
    #[cfg(feature = "deploy")]
    /// Standard HydroDeploy
    HydroDeploy,
    #[cfg(any(feature = "docker_deploy", feature = "ecs_deploy"))]
    /// Containerized deployment (Docker/ECS)
    Containerized,
    #[cfg(feature = "maelstrom")]
    /// Maelstrom deployment with stdin/stdout JSON protocol
    Maelstrom,
}

pub(crate) static IS_TEST: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

pub(crate) static CONCURRENT_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Enables "test mode" for Hydro, which makes it possible to compile Hydro programs written
/// inside a `#[cfg(test)]` module. This should be enabled in a global [`ctor`] hook.
///
/// # Example
/// ```ignore
/// #[cfg(test)]
/// mod test_init {
///    #[ctor::ctor]
///    fn init() {
///        hydro_lang::compile::init_test();
///    }
/// }
/// ```
pub fn init_test() {
    IS_TEST.store(true, std::sync::atomic::Ordering::Relaxed);
}

#[cfg(any(feature = "deploy", feature = "maelstrom"))]
fn clean_bin_name_prefix(bin_name_prefix: &str) -> String {
    bin_name_prefix
        .replace("::", "__")
        .replace(" ", "_")
        .replace(",", "_")
        .replace("<", "_")
        .replace(">", "")
        .replace("(", "")
        .replace(")", "")
        .replace("{", "_")
        .replace("}", "_")
}

#[derive(Debug, Clone)]
pub struct TrybuildConfig {
    pub project_dir: PathBuf,
    pub target_dir: PathBuf,
    pub features: Option<Vec<String>>,
    #[cfg(any(feature = "deploy", feature = "maelstrom"))]
    // Only the deploy backends read this field; Maelstrom-only builds derive the
    // linking behavior directly.
    #[cfg_attr(
        not(feature = "deploy"),
        expect(dead_code, reason = "only read by the deploy backends")
    )]
    /// Which crate within the workspace to use for examples.
    /// - `Static`: base crate (for remote/containerized deploys)
    /// - `Dynamic`: dylib-examples crate (for sim and localhost deploys)
    pub linking_mode: LinkingMode,
}

#[cfg(any(feature = "deploy", feature = "maelstrom"))]
pub fn create_graph_trybuild(
    graph: DfirGraph,
    extra_stmts: &[syn::Stmt],
    sidecars: &[syn::Expr],
    bin_name_prefix: Option<&str>,
    deploy_mode: DeployMode,
    linking_mode: LinkingMode,
) -> (String, TrybuildConfig) {
    let source_dir = cargo::manifest_dir().unwrap();
    let source_manifest = dependencies::get_manifest(&source_dir).unwrap();
    let crate_name = source_manifest.package.name.replace("-", "_");

    let is_test = IS_TEST.load(std::sync::atomic::Ordering::Relaxed);

    let generated_code =
        compile_graph_trybuild(graph, extra_stmts, sidecars, &crate_name, deploy_mode);

    let inlined_staged = if is_test {
        let raw_toml_manifest = toml::from_str::<toml::Value>(
            &fs::read_to_string(path!(source_dir / "Cargo.toml")).unwrap(),
        )
        .unwrap();

        let maybe_custom_lib_path = raw_toml_manifest
            .get("lib")
            .and_then(|lib| lib.get("path"))
            .and_then(|path| path.as_str());

        let mut gen_staged = stageleft_tool::gen_staged_trybuild(
            &maybe_custom_lib_path
                .map(|s| path!(source_dir / s))
                .unwrap_or_else(|| path!(source_dir / "src" / "lib.rs")),
            &path!(source_dir / "Cargo.toml"),
            &crate_name,
            Some("hydro___test".to_owned()),
        );

        gen_staged.attrs.insert(
            0,
            syn::parse_quote! {
                #![allow(
                    unused,
                    ambiguous_glob_reexports,
                    clippy::suspicious_else_formatting,
                    unexpected_cfgs,
                    reason = "generated code"
                )]
            },
        );

        Some(prettyplease::unparse(&gen_staged))
    } else {
        None
    };

    let source = prettyplease::unparse(&generated_code);

    let hash = format!("{:X}", Sha256::digest(&source))
        .chars()
        .take(8)
        .collect::<String>();

    let bin_name = if let Some(bin_name_prefix) = &bin_name_prefix {
        format!("{}_{}", clean_bin_name_prefix(bin_name_prefix), &hash)
    } else {
        hash
    };

    let (project_dir, target_dir, mut cur_bin_enabled_features) = create_trybuild().unwrap();

    // Determine which crate's examples folder to use based on linking mode
    let examples_dir = match linking_mode {
        LinkingMode::Static => path!(project_dir / "examples"),
        #[cfg(any(feature = "deploy", feature = "maelstrom"))]
        LinkingMode::Dynamic => path!(project_dir / "dylib-examples" / "examples"),
    };

    // TODO(shadaj): garbage collect this directory occasionally
    fs::create_dir_all(&examples_dir).unwrap();

    let out_path = path!(examples_dir / format!("{bin_name}.rs"));
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
            .push("hydro___test".to_owned());
    }

    (
        bin_name,
        TrybuildConfig {
            project_dir,
            target_dir,
            features: cur_bin_enabled_features,
            #[cfg(any(feature = "deploy", feature = "maelstrom"))]
            linking_mode,
        },
    )
}

#[cfg(any(feature = "deploy", feature = "maelstrom"))]
pub fn compile_graph_trybuild(
    partitioned_graph: DfirGraph,
    extra_stmts: &[syn::Stmt],
    sidecars: &[syn::Expr],
    crate_name: &str,
    deploy_mode: DeployMode,
) -> syn::File {
    use crate::staging_util::get_this_crate;

    let mut diagnostics = Diagnostics::new();
    let dfir_expr: syn::Expr = syn::parse2(
        partitioned_graph
            .as_code(&quote! { __root_dfir_rs }, true, quote!(), &mut diagnostics)
            .expect("DFIR code generation failed with diagnostics."),
    )
    .unwrap();

    let orig_crate_name = quote::format_ident!("{}", crate_name);
    let trybuild_crate_name_ident = quote::format_ident!("{}_hydro_trybuild", crate_name);
    let root = get_this_crate();
    let tokio_main_ident = format!("{}::runtime_support::tokio", root);
    let dfir_ident = quote::format_ident!("{}", crate::compile::DFIR_IDENT);

    let source_ast: syn::File = match deploy_mode {
        #[cfg(any(feature = "docker_deploy", feature = "ecs_deploy"))]
        DeployMode::Containerized => {
            syn::parse_quote! {
                #![allow(unused_imports, unused_crate_dependencies, missing_docs, non_snake_case, unexpected_cfgs, unfulfilled_lint_expectations)]
                use #trybuild_crate_name_ident::__root as #orig_crate_name;
                use #orig_crate_name::*;
                use #orig_crate_name::__staged::__deps::*;
                use #root::prelude::*;
                use #root::runtime_support::dfir_rs as __root_dfir_rs;
                pub use #orig_crate_name::__staged;

                #[#root::runtime_support::tokio::main(crate = #tokio_main_ident, flavor = "current_thread")]
                async fn main() {
                    #root::telemetry::initialize_tracing();

                    #( #extra_stmts )*

                    let mut #dfir_ident = #dfir_expr;

                    let local_set = #root::runtime_support::tokio::task::LocalSet::new();
                    #(
                        let _ = local_set.spawn_local( #sidecars ); // Uses #dfir_ident
                    )*

                    let _ = local_set.run_until(#dfir_ident.run()).await;
                }
            }
        }
        #[cfg(feature = "deploy")]
        DeployMode::HydroDeploy => {
            syn::parse_quote! {
                #![allow(unused_imports, unused_crate_dependencies, missing_docs, non_snake_case, unexpected_cfgs, unfulfilled_lint_expectations)]
                use #trybuild_crate_name_ident::__root as #orig_crate_name;
                use #orig_crate_name::*;
                use #orig_crate_name::__staged::__deps::*;
                use #root::prelude::*;
                use #root::runtime_support::dfir_rs as __root_dfir_rs;
                pub use #orig_crate_name::__staged;

                #[#root::runtime_support::tokio::main(crate = #tokio_main_ident, flavor = "current_thread")]
                async fn main() {
                    let __hydro_lang_trybuild_cli_owned: #root::runtime_support::hydro_deploy_integration::DeployPorts<#root::__staged::deploy::deploy_runtime::HydroMeta> = #root::runtime_support::launch::init_no_ack_start().await;
                    let __hydro_lang_trybuild_cli = &__hydro_lang_trybuild_cli_owned;

                    #( #extra_stmts )*

                    let mut #dfir_ident = #dfir_expr;
                    println!("ack start");

                    // TODO(mingwei): initialize `tracing` at this point in execution.
                    // After "ack start" is when we can print whatever we want.

                    let local_set = #root::runtime_support::tokio::task::LocalSet::new();
                    #(
                        let _ = local_set.spawn_local( #sidecars ); // Uses #dfir_ident
                    )*

                    let _ = local_set.run_until(#root::runtime_support::launch::run_stdin_commands(
                        async move {
                            #dfir_ident.run().await
                        }
                    )).await;
                }
            }
        }
        #[cfg(feature = "maelstrom")]
        DeployMode::Maelstrom => {
            syn::parse_quote! {
                #![allow(unused_imports, unused_crate_dependencies, missing_docs, non_snake_case, unexpected_cfgs, unfulfilled_lint_expectations)]
                use #trybuild_crate_name_ident::__root as #orig_crate_name;
                use #orig_crate_name::*;
                use #orig_crate_name::__staged::__deps::*;
                use #root::prelude::*;
                use #root::runtime_support::dfir_rs as __root_dfir_rs;
                pub use #orig_crate_name::__staged;

                #[allow(unused)]
                fn __hydro_runtime<'a>(
                    __hydro_lang_maelstrom_meta: &'a #root::__staged::deploy::maelstrom::deploy_runtime_maelstrom::MaelstromMeta
                )
                    -> #root::runtime_support::dfir_rs::scheduled::context::Dfir<impl #root::runtime_support::dfir_rs::scheduled::context::TickClosure + 'a>
                {
                    #( #extra_stmts )*

                    #dfir_expr
                }

                #[#root::runtime_support::tokio::main(crate = #tokio_main_ident, flavor = "current_thread")]
                async fn main() {
                    #root::telemetry::initialize_tracing();

                    // Initialize Maelstrom protocol - read init message and send init_ok
                    let __hydro_lang_maelstrom_meta = #root::__staged::deploy::maelstrom::deploy_runtime_maelstrom::maelstrom_init();

                    let mut #dfir_ident = __hydro_runtime(&__hydro_lang_maelstrom_meta);

                    __hydro_lang_maelstrom_meta.start_receiving(); // start receiving messages after initializing subscribers

                    let local_set = #root::runtime_support::tokio::task::LocalSet::new();
                    #(
                        let _ = local_set.spawn_local( #sidecars ); // Uses #dfir_ident
                    )*

                    let _ = local_set.run_until(#dfir_ident.run()).await;
                }
            }
        }
    };
    source_ast
}

/// Configuration for [`compile_trybuild_example`], the shared concurrent-build
/// entrypoint used by both the simulator and the Maelstrom deployment target.
#[cfg(any(feature = "sim", feature = "maelstrom"))]
pub struct ExampleBuildConfig<'a> {
    /// The trybuild project + target directories and enabled features.
    pub trybuild: TrybuildConfig,
    /// The generated example base name (a content hash). Used as the per-job
    /// directory name and, when [`Self::set_trybuild_lib_name`] is set, as the
    /// value of the `TRYBUILD_LIB_NAME` environment variable.
    pub bin_name: String,
    /// A runtime feature to enable in addition to [`TrybuildConfig::features`]
    /// (e.g. `hydro___feature_sim_runtime` or `hydro___feature_maelstrom_runtime`).
    pub runtime_feature: &'a str,
    /// The cargo `--example` target to build. For the simulator this is the
    /// fixed `sim-dylib` wrapper; for Maelstrom it is the generated `bin_name`.
    pub example_name: String,
    /// If `Some`, override the crate type on the command line (e.g. `cdylib`
    /// for the simulator). `None` builds a normal executable example.
    pub crate_type: Option<&'a str>,
    /// Whether to set `TRYBUILD_LIB_NAME` to `bin_name` (the simulator uses this
    /// for its `include!`-based indirection).
    pub set_trybuild_lib_name: bool,
    /// Whether to honor the `BOLERO_FUZZER` environment variable. Only the
    /// simulator supports fuzzing; other targets should set this to `false`.
    pub allow_fuzz: bool,
}

/// Compiles a generated trybuild example against the prebuilt dylib crate,
/// using the shared parallel-compilation machinery (per-job target dirs with
/// symlinked shared artifacts, plus a prebuild of the dylib dependencies).
///
/// Returns the path to a temporary copy of the built artifact (a `cdylib` for
/// the simulator, or an executable for Maelstrom). The copy allows the caller
/// to hold onto the artifact independently of the shared target directory.
#[cfg(any(feature = "sim", feature = "maelstrom"))]
pub fn compile_trybuild_example(config: ExampleBuildConfig<'_>) -> Result<tempfile::TempPath, ()> {
    use std::process::{Command, Stdio};

    let ExampleBuildConfig {
        trybuild,
        bin_name,
        runtime_feature,
        example_name,
        crate_type,
        set_trybuild_lib_name,
        allow_fuzz,
    } = config;

    let is_fuzz = allow_fuzz && std::env::var("BOLERO_FUZZER").is_ok();
    // When RUSTFLAGS is set, our prebuild fingerprint doesn't account for it, so skip the
    // parallel build machinery entirely and build directly into the shared target dir.
    let has_custom_rustflags = std::env::var("RUSTFLAGS").is_ok();

    // Run from dylib-examples crate which has the dylib as a dev-dependency (only if not fuzzing)
    let crate_to_compile = if is_fuzz {
        trybuild.project_dir.clone()
    } else {
        path!(trybuild.project_dir / "dylib-examples")
    };

    let (final_target_dir, _prebuild_guard, _cargo_lock) = if !has_custom_rustflags {
        let shared_debug = trybuild.target_dir.join("debug");
        let jobs_dir = trybuild.target_dir.join("jobs");
        let per_job = hydro_concurrent_cargo::setup_job_dir(&jobs_dir, &bin_name, &shared_debug);

        let mut features: Vec<String> = trybuild.features.clone().unwrap_or_default();
        features.push(runtime_feature.to_owned());

        let staged_paths = vec![
            path!(trybuild.project_dir / "src" / "__staged.rs"),
            path!(trybuild.project_dir / "Cargo.lock"),
            std::env::current_exe().unwrap(),
        ];

        let project_dir = trybuild.project_dir.clone();
        let features_for_closure = features.clone();
        let is_fuzz_for_closure = is_fuzz;

        let (guard, cargo_lock) = hydro_concurrent_cargo::run_prebuild(
            &trybuild.target_dir,
            trybuild.project_dir.file_name().unwrap().to_str().unwrap(),
            &features,
            &staged_paths,
            |prebuild_target| {
                let features_str = features_for_closure.join(",");

                let dylib_crate = path!(project_dir / "dylib");
                let mut dep_cmd = Command::new("cargo");
                dep_cmd.current_dir(&dylib_crate);
                dep_cmd.args(["build", "--locked"]);
                dep_cmd.args(["--target-dir", prebuild_target.to_str().unwrap()]);
                dep_cmd.args(["--features", &features_str]);
                dep_cmd.args(["--config", "build.incremental = false"]);
                dep_cmd.env("STAGELEFT_TRYBUILD_BUILD_STAGED", "1");
                eprintln!("[hydro-build] starting prebuild child cargo");
                let status = dep_cmd.stdin(Stdio::null()).status().unwrap();
                eprintln!(
                    "[hydro-build] prebuild child cargo finished, success={}",
                    status.success()
                );
                if !status.success() {
                    panic!("dep prebuild failed");
                }

                // Also prebuild the dylib-examples lib so concurrent final builds
                // don't race on compiling it. Skip in fuzz mode since fuzzing
                // compiles from the base trybuild crate directly (no dylib-examples).
                if !is_fuzz_for_closure {
                    eprintln!("[hydro-build] starting prebuild dylib-examples lib");
                    let dylib_examples_crate = path!(project_dir / "dylib-examples");
                    let mut lib_cmd = Command::new("cargo");
                    lib_cmd.current_dir(&dylib_examples_crate);
                    lib_cmd.args(["build", "--locked", "--lib"]);
                    lib_cmd.args(["--target-dir", prebuild_target.to_str().unwrap()]);
                    lib_cmd.args(["--features", &features_str]);
                    lib_cmd.args(["--config", "build.incremental = false"]);
                    lib_cmd.env("STAGELEFT_TRYBUILD_BUILD_STAGED", "1");
                    let lib_status = lib_cmd.stdin(Stdio::null()).status().unwrap();
                    if !lib_status.success() {
                        panic!("dylib-examples lib prebuild failed");
                    }
                }
            },
        );

        (per_job, Some(guard), Some(cargo_lock))
    } else {
        (trybuild.target_dir.clone(), None, None)
    };

    // Populate per-job build/ dir right before final build. Hold guard for entire build.
    let _job_build_guard = if !has_custom_rustflags {
        let shared_debug = trybuild.target_dir.join("debug");
        Some(hydro_concurrent_cargo::populate_job_build_dir(
            &final_target_dir.join("debug"),
            &shared_debug,
        ))
    } else {
        None
    };

    let mut command = Command::new("cargo");
    command.current_dir(&crate_to_compile);
    command.args([
        "rustc",
        if has_custom_rustflags {
            "--locked"
        } else {
            "--frozen"
        },
    ]);
    command.args(["--example", &example_name]);
    command.args(["--target-dir", final_target_dir.to_str().unwrap()]);
    command.args([
        "--features",
        &trybuild
            .features
            .clone()
            .into_iter()
            .flatten()
            .chain([runtime_feature.to_owned()])
            .collect::<Vec<_>>()
            .join(","),
    ]);
    command.args(["--config", "build.incremental = false"]);
    if let Some(crate_type) = crate_type {
        command.args(["--crate-type", crate_type]);
    }
    command.arg("--message-format=json-diagnostic-rendered-ansi");
    command.env("STAGELEFT_TRYBUILD_BUILD_STAGED", "1");
    if set_trybuild_lib_name {
        command.env("TRYBUILD_LIB_NAME", &bin_name);
    }

    command.arg("--");

    if cfg!(target_os = "linux") {
        let debug_path = if let Ok(target) = std::env::var("CARGO_BUILD_TARGET") {
            path!(final_target_dir / target / "debug")
        } else {
            path!(final_target_dir / "debug")
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

    if allow_fuzz && let Ok(fuzzer) = std::env::var("BOLERO_FUZZER") {
        command.env_remove("BOLERO_FUZZER");

        if fuzzer == "libfuzzer" {
            #[cfg(target_os = "macos")]
            {
                command.args(["-Clink-arg=-undefined", "-Clink-arg=dynamic_lookup"]);
            }

            #[cfg(target_os = "linux")]
            {
                command.args(["-Clink-arg=-Wl,--unresolved-symbols=ignore-all"]);
            }
        }
    }

    let mut spawned = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .spawn()
        .unwrap();
    let reader = std::io::BufReader::new(spawned.stdout.take().unwrap());
    let stderr_handle = spawned.stderr.take().unwrap();
    let stderr_thread = std::thread::spawn(move || {
        use std::io::Read;
        let mut buf = String::new();
        std::io::BufReader::new(stderr_handle)
            .read_to_string(&mut buf)
            .unwrap();
        buf
    });

    let mut out = Err(());
    for message in cargo_metadata::Message::parse_stream(reader) {
        match message.unwrap() {
            cargo_metadata::Message::CompilerArtifact(artifact) => {
                // unlike dylib, cdylib only exports the explicitly exported symbols
                let is_output = artifact.target.is_example();

                if is_output {
                    let path = artifact.filenames.first().unwrap();
                    let path_buf: PathBuf = path.clone().into();
                    out = Ok(path_buf);
                }
            }
            cargo_metadata::Message::CompilerMessage(mut msg) => {
                // Update the path displayed to enable clicking in IDE.
                // TODO(mingwei): deduplicate code with hydro_deploy rust_crate/build.rs
                if let Some(rendered) = msg.message.rendered.as_mut() {
                    let file_names = msg
                        .message
                        .spans
                        .iter()
                        .map(|s| &s.file_name)
                        .collect::<std::collections::BTreeSet<_>>();
                    for file_name in file_names {
                        *rendered = rendered.replace(
                            file_name,
                            &format!("(full path) {}/{file_name}", trybuild.project_dir.display()),
                        )
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
    let stderr_output = stderr_thread.join().unwrap();

    // Check for unexpected recompilations — only dylib-examples should be compiled.
    // (Only relevant when prebuild is active, i.e. no custom RUSTFLAGS.)
    if !has_custom_rustflags {
        for line in stderr_output.lines() {
            if line.contains("Compiling") && !line.contains("dylib-examples") {
                panic!(
                    "unexpected recompilation in final build: {line}\nfull stderr:\n{stderr_output}"
                );
            }
        }
    }

    if out.is_err() {
        panic!("final build failed to produce binary.\nstderr:\n{stderr_output}");
    }

    let out_file = tempfile::NamedTempFile::new().unwrap().into_temp_path();
    fs::copy(out.as_ref().unwrap(), &out_file).unwrap();
    Ok(out_file)
}

pub fn create_trybuild()
-> Result<(PathBuf, PathBuf, Option<Vec<String>>), trybuild_internals_api::error::Error> {
    let Metadata {
        target_directory: target_dir,
        workspace_root: workspace,
        packages,
    } = cargo::metadata()?;

    let source_dir = cargo::manifest_dir()?;
    let mut source_manifest = dependencies::get_manifest(&source_dir)?;

    let mut dev_dependency_features = vec![];
    source_manifest.dev_dependencies.retain(|k, v| {
        if source_manifest.dependencies.contains_key(k) {
            // already a non-dev dependency, so drop the dep and put the features under the test flag
            for feat in &v.features {
                dev_dependency_features.push(format!("{}/{}", k, feat));
            }

            false
        } else {
            // only enable this in test mode, so make it optional otherwise
            dev_dependency_features.push(format!("dep:{k}"));

            v.optional = true;
            true
        }
    });

    let mut features = features::find();

    let path_dependencies = source_manifest
        .dependencies
        .iter()
        .filter_map(|(name, dep)| {
            let path = dep.path.as_ref()?;
            if packages.iter().any(|p| &p.name == name) {
                // Skip path dependencies coming from the workspace itself
                None
            } else {
                Some(PathDependency {
                    name: name.clone(),
                    normalized_path: path.canonicalize().ok()?,
                })
            }
        })
        .collect();

    let crate_name = source_manifest.package.name.clone();
    let project_dir = path!(target_dir / "hydro_trybuild" / crate_name /);
    fs::create_dir_all(&project_dir)?;

    let project_name = format!("{}-hydro-trybuild", crate_name);
    let mut manifest = Runner::make_manifest(
        &workspace,
        &project_name,
        &source_dir,
        &packages,
        &[],
        source_manifest,
    )?;

    if let Some(enabled_features) = &mut features {
        enabled_features
            .retain(|feature| manifest.features.contains_key(feature) || feature == "default");
    }

    for runtime_feature in HYDRO_RUNTIME_FEATURES {
        manifest.features.insert(
            format!("hydro___feature_{runtime_feature}"),
            vec![format!("hydro_lang/{runtime_feature}")],
        );
    }

    manifest
        .dependencies
        .get_mut("hydro_lang")
        .unwrap()
        .features
        .push("runtime_support".to_owned());

    manifest
        .features
        .insert("hydro___test".to_owned(), dev_dependency_features);

    if manifest
        .workspace
        .as_ref()
        .is_some_and(|w| w.dependencies.is_empty())
    {
        manifest.workspace = None;
    }

    let project = Project {
        dir: project_dir,
        source_dir,
        target_dir,
        name: project_name.clone(),
        update: Update::env()?,
        has_pass: false,
        has_compile_fail: false,
        features,
        workspace,
        path_dependencies,
        manifest,
        keep_going: false,
    };

    {
        let _concurrent_test_lock = CONCURRENT_TEST_LOCK.lock().unwrap();

        let project_lock = File::create(path!(project.dir / ".hydro-trybuild-lock"))?;
        project_lock.lock()?;

        fs::create_dir_all(path!(project.dir / "src"))?;
        fs::create_dir_all(path!(project.dir / "examples"))?;

        let crate_name_ident = syn::Ident::new(
            &crate_name.replace("-", "_"),
            proc_macro2::Span::call_site(),
        );

        write_atomic(
            prettyplease::unparse(&syn::parse_quote! {
                #![allow(unused_imports, unused_crate_dependencies, missing_docs, non_snake_case, unexpected_cfgs, unfulfilled_lint_expectations)]

                pub mod __root {
                    pub use #crate_name_ident::*;
                    #[cfg(feature = "hydro___test")]
                    pub use super::__staged;
                }

                #[cfg(feature = "hydro___test")]
                pub mod __staged;
            })
            .as_bytes(),
            &path!(project.dir / "src" / "lib.rs"),
        )
        .unwrap();

        let base_manifest = toml::to_string(&project.manifest)?;

        // Collect feature names for forwarding to dylib and dylib-examples crates
        let feature_names: Vec<_> = project.manifest.features.keys().cloned().collect();

        // Create dylib crate directory
        let dylib_dir = path!(project.dir / "dylib");
        fs::create_dir_all(path!(dylib_dir / "src"))?;

        let trybuild_crate_name_ident = syn::Ident::new(
            &project_name.replace("-", "_"),
            proc_macro2::Span::call_site(),
        );
        write_atomic(
            prettyplease::unparse(&syn::parse_quote! {
                #![allow(unused_imports, unused_crate_dependencies, missing_docs, non_snake_case, unexpected_cfgs, unfulfilled_lint_expectations)]
                pub use #trybuild_crate_name_ident::*;
            })
            .as_bytes(),
            &path!(dylib_dir / "src" / "lib.rs"),
        )?;

        let serialized_edition = toml::to_string(
            &vec![("edition", &project.manifest.package.edition)]
                .into_iter()
                .collect::<std::collections::HashMap<_, _>>(),
        )
        .unwrap();

        // Dylib crate Cargo.toml - only dylib crate-type, with feature forwarding to base crate
        // On Windows, we currently disable dylib compilation due to https://github.com/bevyengine/bevy/pull/2016
        let dylib_features_section = feature_names
            .iter()
            .map(|f| format!("{f} = [\"{project_name}/{f}\"]"))
            .collect::<Vec<_>>()
            .join("\n");

        let dylib_manifest = format!(
            r#"[package]
name = "{project_name}-dylib"
version = "0.0.0"
{}

[lib]
crate-type = ["{}"]

[dependencies]
{project_name} = {{ path = "..", default-features = false }}

[features]
{dylib_features_section}
"#,
            serialized_edition,
            if cfg!(target_os = "windows") {
                "rlib"
            } else {
                "dylib"
            }
        );
        write_atomic(dylib_manifest.as_ref(), &path!(dylib_dir / "Cargo.toml"))?;

        let dylib_examples_dir = path!(project.dir / "dylib-examples");
        fs::create_dir_all(path!(dylib_examples_dir / "src"))?;
        fs::create_dir_all(path!(dylib_examples_dir / "examples"))?;

        write_atomic(
            b"#![allow(unused_crate_dependencies)]\n",
            &path!(dylib_examples_dir / "src" / "lib.rs"),
        )?;

        // Build feature forwarding for dylib-examples - forward to both base and dylib crates
        let features_section = feature_names
            .iter()
            .map(|f| format!("{f} = [\"{project_name}/{f}\", \"{project_name}-dylib/{f}\"]"))
            .collect::<Vec<_>>()
            .join("\n");

        // Dylib-examples crate Cargo.toml - has base crate and dylib as dev-dependencies
        let dylib_examples_manifest = format!(
            r#"[package]
name = "{project_name}-dylib-examples"
version = "0.0.0"
{}

[dev-dependencies]
{project_name} = {{ path = "..", default-features = false }}
{project_name}-dylib = {{ path = "../dylib", default-features = false }}

[features]
{features_section}

[[example]]
name = "sim-dylib"
crate-type = ["cdylib"]
"#,
            serialized_edition
        );
        write_atomic(
            dylib_examples_manifest.as_ref(),
            &path!(dylib_examples_dir / "Cargo.toml"),
        )?;

        // sim-dylib.rs for the base crate and dylib-examples crate
        let sim_dylib_contents = prettyplease::unparse(&syn::parse_quote! {
            #![allow(unused_imports, unused_crate_dependencies, missing_docs, non_snake_case, unexpected_cfgs, unfulfilled_lint_expectations)]
            include!(std::concat!(env!("TRYBUILD_LIB_NAME"), ".rs"));
        });
        write_atomic(
            sim_dylib_contents.as_bytes(),
            &path!(project.dir / "examples" / "sim-dylib.rs"),
        )?;
        write_atomic(
            sim_dylib_contents.as_bytes(),
            &path!(dylib_examples_dir / "examples" / "sim-dylib.rs"),
        )?;

        let workspace_manifest = format!(
            r#"{}
[[example]]
name = "sim-dylib"
crate-type = ["cdylib"]

[workspace]
members = ["dylib", "dylib-examples"]
"#,
            base_manifest,
        );

        write_atomic(
            workspace_manifest.as_ref(),
            &path!(project.dir / "Cargo.toml"),
        )?;

        // Compute hash for cache invalidation (dylib and dylib-examples are functions of workspace_manifest)
        let manifest_hash = format!("{:X}", Sha256::digest(&workspace_manifest))
            .chars()
            .take(8)
            .collect::<String>();

        let workspace_cargo_lock = path!(project.workspace / "Cargo.lock");
        let workspace_cargo_lock_contents_and_hash = if workspace_cargo_lock.exists() {
            let cargo_lock_contents = fs::read_to_string(&workspace_cargo_lock)?;

            let hash = format!("{:X}", Sha256::digest(&cargo_lock_contents))
                .chars()
                .take(8)
                .collect::<String>();

            Some((cargo_lock_contents, hash))
        } else {
            None
        };

        let trybuild_hash = format!(
            "{}-{}",
            manifest_hash,
            workspace_cargo_lock_contents_and_hash
                .as_ref()
                .map(|(_contents, hash)| &**hash)
                .unwrap_or_default()
        );

        if !check_contents(
            trybuild_hash.as_bytes(),
            &path!(project.dir / ".hydro-trybuild-manifest"),
        )
        .is_ok_and(|b| b)
        {
            // this is expensive, so we only do it if the manifest changed
            if let Some((cargo_lock_contents, _)) = workspace_cargo_lock_contents_and_hash {
                // only overwrite when the hash changed, because writing Cargo.lock must be
                // immediately followed by a local `cargo update -w`
                write_atomic(
                    cargo_lock_contents.as_ref(),
                    &path!(project.dir / "Cargo.lock"),
                )?;
            } else {
                let _ = cargo::cargo(&project).arg("generate-lockfile").status();
            }

            // not `--offline` because some new runtime features may be enabled
            std::process::Command::new("cargo")
                .current_dir(&project.dir)
                .args(["update", "-w"]) // -w to not actually update any versions
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .unwrap();

            write_atomic(
                trybuild_hash.as_bytes(),
                &path!(project.dir / ".hydro-trybuild-manifest"),
            )?;
        }

        // Create examples folder for base crate (static linking)
        let examples_folder = path!(project.dir / "examples");
        fs::create_dir_all(&examples_folder)?;

        let workspace_dot_cargo_config_toml = path!(project.workspace / ".cargo" / "config.toml");
        if workspace_dot_cargo_config_toml.exists() {
            let dot_cargo_folder = path!(project.dir / ".cargo");
            fs::create_dir_all(&dot_cargo_folder)?;

            write_atomic(
                fs::read_to_string(&workspace_dot_cargo_config_toml)?.as_ref(),
                &path!(dot_cargo_folder / "config.toml"),
            )?;
        }

        let vscode_folder = path!(project.dir / ".vscode");
        fs::create_dir_all(&vscode_folder)?;
        write_atomic(
            include_bytes!("./vscode-trybuild.json"),
            &path!(vscode_folder / "settings.json"),
        )?;
    }

    Ok((
        project.dir.as_ref().into(),
        project.target_dir.as_ref().into(),
        project.features,
    ))
}

fn check_contents(contents: &[u8], path: &Path) -> Result<bool, std::io::Error> {
    let mut file = File::options()
        .read(true)
        .write(false)
        .create(false)
        .truncate(false)
        .open(path)?;
    file.lock()?;

    let mut existing_contents = Vec::new();
    file.read_to_end(&mut existing_contents)?;
    Ok(existing_contents == contents)
}

pub(crate) fn write_atomic(contents: &[u8], path: &Path) -> Result<(), std::io::Error> {
    let mut file = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;

    let mut existing_contents = Vec::new();
    file.read_to_end(&mut existing_contents)?;
    if existing_contents != contents {
        file.lock()?;
        file.seek(SeekFrom::Start(0))?;
        file.set_len(0)?;
        file.write_all(contents)?;
    }

    Ok(())
}
