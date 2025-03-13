use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;

use dfir_lang::graph::DfirGraph;
use sha2::{Digest, Sha256};
use stageleft::internal::quote;
use syn::visit_mut::VisitMut;
use trybuild_internals_api::cargo::{self, Metadata};
use trybuild_internals_api::env::Update;
use trybuild_internals_api::run::{PathDependency, Project};
use trybuild_internals_api::{Runner, dependencies, features, path};

use super::trybuild_rewriters::ReplaceCrateNameWithStaged;

pub const HYDRO_RUNTIME_FEATURES: [&str; 1] = ["runtime_measure"];

static IS_TEST: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

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

pub fn create_graph_trybuild(
    graph: DfirGraph,
    extra_stmts: Vec<syn::Stmt>,
    name_hint: &Option<String>,
) -> (String, (PathBuf, PathBuf, Option<Vec<String>>)) {
    let source_dir = cargo::manifest_dir().unwrap();
    let source_manifest = dependencies::get_manifest(&source_dir).unwrap();
    let crate_name = &source_manifest.package.name.to_string().replace("-", "_");

    let is_test = IS_TEST.load(std::sync::atomic::Ordering::Relaxed);

    let mut generated_code = compile_graph_trybuild(graph, extra_stmts);

    ReplaceCrateNameWithStaged {
        crate_name: crate_name.clone(),
        is_test,
    }
    .visit_file_mut(&mut generated_code);

    let inlined_staged = if is_test {
        stageleft_tool::gen_staged_trybuild(
            &path!(source_dir / "src" / "lib.rs"),
            crate_name.clone(),
            is_test,
        )
    } else {
        syn::parse_quote!()
    };

    let source = prettyplease::unparse(&syn::parse_quote! {
        #generated_code

        #[allow(
            unused,
            ambiguous_glob_reexports,
            clippy::suspicious_else_formatting,
            unexpected_cfgs,
            reason = "generated code"
        )]
        pub mod __staged {
            #inlined_staged
        }
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
        let mut out_file = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&out_path)
            .unwrap();
        #[cfg(nightly)]
        out_file.lock().unwrap();

        let mut existing_contents = String::new();
        out_file.read_to_string(&mut existing_contents).unwrap();
        if existing_contents != source {
            out_file.write_all(source.as_ref()).unwrap()
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
        (project_dir, target_dir, cur_bin_enabled_features),
    )
}

pub fn compile_graph_trybuild(
    partitioned_graph: DfirGraph,
    extra_stmts: Vec<syn::Stmt>,
) -> syn::File {
    let mut diagnostics = Vec::new();
    let tokens = partitioned_graph.as_code(
        &quote! { hydro_lang::dfir_rs },
        true,
        quote!(),
        &mut diagnostics,
    );

    let source_ast: syn::File = syn::parse_quote! {
        #![allow(unused_imports, unused_crate_dependencies, missing_docs, non_snake_case)]
        use hydro_lang::*;

        #[allow(unused)]
        fn __hydro_runtime<'a>(__hydro_lang_trybuild_cli: &'a hydro_lang::dfir_rs::util::deploy::DeployPorts<hydro_lang::deploy_runtime::HydroMeta>) -> hydro_lang::dfir_rs::scheduled::graph::Dfir<'a> {
            #(#extra_stmts)*
            #tokens
        }

        #[tokio::main]
        async fn main() {
            let ports = hydro_lang::dfir_rs::util::deploy::init_no_ack_start().await;
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

    manifest.features.remove("stageleft_devel");

    if let Some(enabled_features) = &mut features {
        enabled_features
            .retain(|feature| manifest.features.contains_key(feature) || feature == "default");

        manifest
            .features
            .get_mut("default")
            .iter_mut()
            .for_each(|v| {
                v.retain(|f| f != "stageleft_devel");
            });
    }

    for runtime_feature in HYDRO_RUNTIME_FEATURES {
        manifest.features.insert(
            format!("hydro___feature_{runtime_feature}"),
            vec![format!("hydro_lang/{runtime_feature}")],
        );
    }

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
        let project_lock = File::create(path!(project.dir / ".hydro-trybuild-lock"))?;
        #[cfg(nightly)]
        project_lock.lock()?;

        let manifest_toml = toml::to_string(&project.manifest)?;
        fs::write(path!(project.dir / "Cargo.toml"), manifest_toml)?;

        let workspace_cargo_lock = path!(project.workspace / "Cargo.lock");
        if workspace_cargo_lock.exists() {
            let _ = fs::copy(workspace_cargo_lock, path!(project.dir / "Cargo.lock"));
        } else {
            let _ = cargo::cargo(&project).arg("generate-lockfile").status();
        }

        let workspace_dot_cargo_config_toml = path!(project.workspace / ".cargo" / "config.toml");
        if workspace_dot_cargo_config_toml.exists() {
            let dot_cargo_folder = path!(project.dir / ".cargo");
            fs::create_dir_all(&dot_cargo_folder)?;

            let _ = fs::copy(
                workspace_dot_cargo_config_toml,
                path!(dot_cargo_folder / "config.toml"),
            );
        }
    }

    Ok((
        project.dir.as_ref().into(),
        path!(project.target_dir / "hydro_trybuild"),
        project.features,
    ))
}
