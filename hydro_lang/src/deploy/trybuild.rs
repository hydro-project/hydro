use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use dfir_lang::graph::DfirGraph;
use sha2::{Digest, Sha256};
use stageleft::internal::quote;
use syn::visit_mut::VisitMut;
use trybuild_internals_api::cargo::{self, Metadata};
use trybuild_internals_api::env::Update;
use trybuild_internals_api::run::{PathDependency, Project};
use trybuild_internals_api::{Runner, dependencies, features, path};

use super::trybuild_rewriters::UseTestModeStaged;

pub const HYDRO_RUNTIME_FEATURES: [&str; 1] = ["runtime_measure"];

static IS_TEST: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

static CONCURRENT_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn init_test() {
    IS_TEST.store(true, std::sync::atomic::Ordering::Relaxed);
}

fn clean_name_hint(name_hint: &str) -> String {
    name_hint
        .replace("::", "__")
        .replace(" ", "_")
        .replace(",", "_")
        .replace("<", "_")
        .replace(">", "")
        .replace("(", "")
        .replace(")", "")
}

pub struct TrybuildConfig {
    pub project_dir: PathBuf,
    pub target_dir: PathBuf,
    pub features: Option<Vec<String>>,
}

pub fn create_graph_trybuild(
    graph: DfirGraph,
    extra_stmts: Vec<syn::Stmt>,
    name_hint: &Option<String>,
) -> (String, TrybuildConfig) {
    let source_dir = cargo::manifest_dir().unwrap();
    let source_manifest = dependencies::get_manifest(&source_dir).unwrap();
    let crate_name = &source_manifest.package.name.to_string().replace("-", "_");

    let is_test = IS_TEST.load(std::sync::atomic::Ordering::Relaxed);

    let generated_code = compile_graph_trybuild(graph, extra_stmts, crate_name.clone(), is_test);

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

    let bin_name = if let Some(name_hint) = &name_hint {
        format!("{}_{}", clean_name_hint(name_hint), &hash)
    } else {
        hash
    };

    let (project_dir, target_dir, mut cur_bin_enabled_features) = create_trybuild().unwrap();

    // TODO(shadaj): garbage collect this directory occasionally
    fs::create_dir_all(path!(project_dir / "src" / "bin")).unwrap();

    let out_path = path!(project_dir / "src" / "bin" / format!("{bin_name}.rs"));
    {
        let _concurrent_test_lock = CONCURRENT_TEST_LOCK.lock().unwrap();
        write_atomic(source.as_ref(), &out_path).unwrap();
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

pub fn compile_graph_trybuild(
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
        #![allow(unused_imports, unused_crate_dependencies, missing_docs, non_snake_case)]
        use hydro_lang::*;
        use hydro_lang::runtime_support::dfir_rs as __root_dfir_rs;

        #[allow(unused)]
        fn __hydro_runtime<'a>(__hydro_lang_trybuild_cli: &'a hydro_lang::runtime_support::dfir_rs::util::deploy::DeployPorts<hydro_lang::deploy_runtime::HydroMeta>) -> hydro_lang::runtime_support::dfir_rs::scheduled::graph::Dfir<'a> {
            #(#extra_stmts)*
            #dfir_expr
        }

        #[hydro_lang::runtime_support::tokio::main(crate = "hydro_lang::runtime_support::tokio", flavor = "current_thread")]
        async fn main() {
            let ports = hydro_lang::runtime_support::dfir_rs::util::deploy::init_no_ack_start().await;
            let flow = __hydro_runtime(&ports);
            println!("ack start");

            hydro_lang::runtime_support::resource_measurement::run(flow).await;
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
        .push("runtime_support".to_string());

    manifest
        .features
        .insert("hydro___test".to_string(), dev_dependency_features);

    let project = Project {
        dir: project_dir,
        source_dir,
        target_dir,
        name: project_name,
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
        // TODO(mingwei): remove cfg once file locking is stable: https://github.com/rust-lang/rust/issues/130994
        #[cfg(nightly)]
        project_lock.lock()?;
        #[cfg(not(nightly))]
        fs2::FileExt::lock_exclusive(&project_lock)?;

        let manifest_toml = toml::to_string(&project.manifest)?;
        write_atomic(manifest_toml.as_ref(), &path!(project.dir / "Cargo.toml"))?;

        let workspace_cargo_lock = path!(project.workspace / "Cargo.lock");
        if workspace_cargo_lock.exists() {
            write_atomic(
                fs::read_to_string(&workspace_cargo_lock)?.as_ref(),
                &path!(project.dir / "Cargo.lock"),
            )?;
        } else {
            let _ = cargo::cargo(&project).arg("generate-lockfile").status();
        }

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
        path!(project.target_dir / "hydro_trybuild"),
        project.features,
    ))
}

fn write_atomic(contents: &[u8], path: &Path) -> Result<(), std::io::Error> {
    let mut file = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;

    // TODO(mingwei): remove cfg once file locking is stable: https://github.com/rust-lang/rust/issues/130994
    #[cfg(nightly)]
    file.lock()?;
    #[cfg(not(nightly))]
    fs2::FileExt::lock_exclusive(&file)?;

    let mut existing_contents = Vec::new();
    file.read_to_end(&mut existing_contents)?;
    if existing_contents != contents {
        file.seek(SeekFrom::Start(0))?;
        file.set_len(0)?;
        file.write_all(contents)?;
    }

    Ok(())
}
