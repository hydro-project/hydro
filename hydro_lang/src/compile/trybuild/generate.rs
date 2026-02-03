use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

#[cfg(feature = "deploy")]
use dfir_lang::graph::DfirGraph;
use sha2::{Digest, Sha256};
#[cfg(feature = "deploy")]
use stageleft::internal::quote;
#[cfg(feature = "deploy")]
use syn::visit_mut::VisitMut;
use trybuild_internals_api::cargo::{self, Metadata};
use trybuild_internals_api::env::Update;
use trybuild_internals_api::run::{PathDependency, Project};
use trybuild_internals_api::{Runner, dependencies, features, path};

#[cfg(feature = "deploy")]
use super::rewriters::UseTestModeStaged;

pub const HYDRO_RUNTIME_FEATURES: &[&str] = &[
    "deploy_integration",
    "runtime_measure",
    "docker_runtime",
    "ecs_runtime",
];

#[cfg(feature = "deploy")]
/// Whether to use dynamic linking for the generated binary.
/// - `Static`: Place in base crate examples (for remote/containerized deploys)
/// - `Dynamic`: Place in dylib crate examples (for sim and localhost deploys)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkingMode {
    Static,
    Dynamic,
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

#[cfg(feature = "deploy")]
fn clean_bin_name_prefix(bin_name_prefix: &str) -> String {
    bin_name_prefix
        .replace("::", "__")
        .replace(" ", "_")
        .replace(",", "_")
        .replace("<", "_")
        .replace(">", "")
        .replace("(", "")
        .replace(")", "")
}

#[derive(Debug, Clone)]
pub struct TrybuildConfig {
    pub project_dir: PathBuf,
    pub target_dir: PathBuf,
    pub features: Option<Vec<String>>,
    #[cfg(feature = "deploy")]
    /// Which crate within the workspace to use for examples.
    /// - `Static`: base crate (for remote/containerized deploys)
    /// - `Dynamic`: dylib-examples crate (for sim and localhost deploys)
    pub linking_mode: LinkingMode,
}

#[cfg(feature = "deploy")]
pub fn create_graph_trybuild(
    graph: DfirGraph,
    extra_stmts: &[syn::Stmt],
    sidecars: &[syn::Expr],
    bin_name_prefix: Option<&str>,
    is_containerized: bool,
    linking_mode: LinkingMode,
) -> (String, TrybuildConfig) {
    let source_dir = cargo::manifest_dir().unwrap();
    let source_manifest = dependencies::get_manifest(&source_dir).unwrap();
    let crate_name = source_manifest.package.name.replace("-", "_");

    let is_test = IS_TEST.load(std::sync::atomic::Ordering::Relaxed);

    let generated_code = compile_graph_trybuild(
        graph,
        extra_stmts,
        sidecars,
        &crate_name,
        is_test,
        is_containerized,
    );

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
            linking_mode,
        },
    )
}

#[cfg(feature = "deploy")]
pub fn compile_graph_trybuild(
    partitioned_graph: DfirGraph,
    extra_stmts: &[syn::Stmt],
    sidecars: &[syn::Expr],
    crate_name: &str,
    is_test: bool,
    is_containerized: bool,
) -> syn::File {
    use crate::staging_util::get_this_crate;

    let mut diagnostics = Vec::new();
    let mut dfir_expr: syn::Expr = syn::parse2(partitioned_graph.as_code(
        &quote! { __root_dfir_rs },
        true,
        quote!(),
        &mut diagnostics,
    ))
    .unwrap();

    if is_test {
        UseTestModeStaged { crate_name }.visit_expr_mut(&mut dfir_expr);
    }

    let orig_crate_name = quote::format_ident!("{}", crate_name);
    let trybuild_crate_name_ident = quote::format_ident!("{}_hydro_trybuild", crate_name);
    let root = get_this_crate();
    let tokio_main_ident = format!("{}::runtime_support::tokio", root);
    let dfir_ident = quote::format_ident!("{}", crate::compile::DFIR_IDENT);

    let source_ast: syn::File = if is_containerized {
        syn::parse_quote! {
            #![allow(unused_imports, unused_crate_dependencies, missing_docs, non_snake_case)]
            use #trybuild_crate_name_ident::__root as #orig_crate_name;
            use #trybuild_crate_name_ident::__staged::__deps::*;
            use #root::prelude::*;
            use #root::runtime_support::dfir_rs as __root_dfir_rs;
            pub use #trybuild_crate_name_ident::__staged;

            #[allow(unused)]
            async fn __hydro_runtime<'a>() -> #root::runtime_support::dfir_rs::scheduled::graph::Dfir<'a> {
                /// extra_stmts
                #( #extra_stmts )*

                /// dfir_expr
                #dfir_expr
            }

            #[#root::runtime_support::tokio::main(crate = #tokio_main_ident, flavor = "current_thread")]
            async fn main() {
                #root::telemetry::initialize_tracing();

                let mut #dfir_ident = __hydro_runtime().await;

                let local_set = #root::runtime_support::tokio::task::LocalSet::new();
                #(
                    let _ = local_set.spawn_local( #sidecars ); // Uses #dfir_ident
                )*

                let _ = local_set.run_until(#dfir_ident.run()).await;
            }
        }
    } else {
        syn::parse_quote! {
            #![allow(unused_imports, unused_crate_dependencies, missing_docs, non_snake_case)]
            use #trybuild_crate_name_ident::__root as #orig_crate_name;
            use #trybuild_crate_name_ident::__staged::__deps::*;
            use #root::prelude::*;
            use #root::runtime_support::dfir_rs as __root_dfir_rs;
            pub use #trybuild_crate_name_ident::__staged;

            #[allow(unused)]
            fn __hydro_runtime<'a>(
                __hydro_lang_trybuild_cli: &'a #root::runtime_support::hydro_deploy_integration::DeployPorts<#root::__staged::deploy::deploy_runtime::HydroMeta>
            )
                -> #root::runtime_support::dfir_rs::scheduled::graph::Dfir<'a>
            {
                #( #extra_stmts )*

                #dfir_expr
            }

            #[#root::runtime_support::tokio::main(crate = #tokio_main_ident, flavor = "current_thread")]
            async fn main() {
                let ports = #root::runtime_support::launch::init_no_ack_start().await;
                let #dfir_ident = __hydro_runtime(&ports);
                println!("ack start");

                // TODO(mingwei): initialize `tracing` at this point in execution.
                // After "ack start" is when we can print whatever we want.

                let local_set = #root::runtime_support::tokio::task::LocalSet::new();
                #(
                    let _ = local_set.spawn_local( #sidecars ); // Uses #dfir_ident
                )*

                let _ = local_set.run_until(#root::runtime_support::launch::run_stdin_commands(#dfir_ident)).await;
            }
        }
    };
    source_ast
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
                #![allow(unused_imports, unused_crate_dependencies, missing_docs, non_snake_case)]

                pub use #crate_name_ident as __root;

                #[cfg(feature = "hydro___test")]
                pub mod __staged;

                #[cfg(not(feature = "hydro___test"))]
                pub use #crate_name_ident::__staged;
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
                #![allow(unused_imports, unused_crate_dependencies, missing_docs, non_snake_case)]
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

        // Dylib crate Cargo.toml - only dylib crate-type, no features needed
        // Features are enabled on the base crate directly from dylib-examples
        // On Windows, we currently disable dylib compilation due to https://github.com/bevyengine/bevy/pull/2016
        let dylib_manifest = format!(
            r#"[package]
name = "{project_name}-dylib"
version = "0.0.0"
{}

[lib]
crate-type = ["{}"]

[dependencies]
{project_name} = {{ path = "..", default-features = false }}
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

        // Build feature forwarding for dylib-examples - forward directly to base crate
        let features_section = feature_names
            .iter()
            .map(|f| format!("{f} = [\"{project_name}/{f}\"]"))
            .collect::<Vec<_>>()
            .join("\n");

        // Dylib-examples crate Cargo.toml - has dylib as dev-dependency, features go to base crate
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
            #![allow(unused_imports, unused_crate_dependencies, missing_docs, non_snake_case)]
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
