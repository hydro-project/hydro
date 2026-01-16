use std::path::PathBuf;
use std::sync::Arc;

use nameof::name_of;
use tracing_options::TracingOptions;

use super::Host;
use crate::rust_crate::build::BuildParams;
use crate::{HostTargetType, ServiceBuilder};

pub mod build;
pub mod ports;

pub mod service;
pub use service::*;

#[cfg(feature = "tracing")]
pub(crate) mod flamegraph;
pub mod tracing_options;

#[derive(PartialEq, Clone)]
pub enum CrateTarget {
    Default,
    Bin(String),
    Example(String),
}

/// Specifies a crate that uses `hydro_deploy_integration` to be
/// deployed as a service.
///
/// A [crate](https://doc.rust-lang.org/cargo/appendix/glossary.html#crate) is a particular
/// [target](https://doc.rust-lang.org/cargo/appendix/glossary.html#target) within a
/// [package](https://doc.rust-lang.org/cargo/appendix/glossary.html#package).
#[derive(Clone)]
pub struct RustCrate {
    src: PathBuf,
    target: CrateTarget,
    profile: Option<String>,
    rustflags: Option<String>,
    target_dir: Option<PathBuf>,
    build_env: Vec<(String, String)>,
    no_default_features: bool,
    features: Option<Vec<String>>,
    config: Vec<String>,
    tracing: Option<TracingOptions>,
    args: Vec<String>,
    display_name: Option<String>,
}

impl RustCrate {
    /// Creates a new `RustCrate`.
    ///
    /// The `src` argument is the path to the package's directory.
    pub fn new(src: impl Into<PathBuf>) -> Self {
        Self {
            src: src.into(),
            target: CrateTarget::Default,
            profile: None,
            rustflags: None,
            target_dir: None,
            build_env: vec![],
            no_default_features: false,
            features: None,
            config: vec![],
            tracing: None,
            args: vec![],
            display_name: None,
        }
    }

    /// Sets the target to be a binary with the given name,
    /// equivalent to `cargo run --bin <name>`.
    pub fn bin(mut self, bin: impl Into<String>) -> Self {
        if self.target != CrateTarget::Default {
            panic!("{} already set", name_of!(target in Self));
        }

        self.target = CrateTarget::Bin(bin.into());
        self
    }

    /// Sets the target to be an example with the given name,
    /// equivalent to `cargo run --example <name>`.
    pub fn example(mut self, example: impl Into<String>) -> Self {
        if self.target != CrateTarget::Default {
            panic!("{} already set", name_of!(target in Self));
        }

        self.target = CrateTarget::Example(example.into());
        self
    }

    /// Sets the profile to be used when building the crate.
    /// Equivalent to `cargo run --profile <profile>`.
    pub fn profile(mut self, profile: impl Into<String>) -> Self {
        if self.profile.is_some() {
            panic!("{} already set", name_of!(profile in Self));
        }

        self.profile = Some(profile.into());
        self
    }

    pub fn rustflags(mut self, rustflags: impl Into<String>) -> Self {
        if self.rustflags.is_some() {
            panic!("{} already set", name_of!(rustflags in Self));
        }

        self.rustflags = Some(rustflags.into());
        self
    }

    pub fn target_dir(mut self, target_dir: impl Into<PathBuf>) -> Self {
        if self.target_dir.is_some() {
            panic!("{} already set", name_of!(target_dir in Self));
        }

        self.target_dir = Some(target_dir.into());
        self
    }

    pub fn build_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.build_env.push((key.into(), value.into()));
        self
    }

    pub fn no_default_features(mut self) -> Self {
        self.no_default_features = true;
        self
    }

    pub fn features(mut self, features: impl IntoIterator<Item = impl Into<String>>) -> Self {
        if self.features.is_none() {
            self.features = Some(vec![]);
        }

        self.features
            .as_mut()
            .unwrap()
            .extend(features.into_iter().map(|s| s.into()));

        self
    }

    pub fn config(mut self, config: impl Into<String>) -> Self {
        self.config.push(config.into());
        self
    }

    pub fn tracing(mut self, perf: impl Into<TracingOptions>) -> Self {
        if self.tracing.is_some() {
            panic!("{} already set", name_of!(tracing in Self));
        }

        self.tracing = Some(perf.into());
        self
    }

    /// Sets the arguments to be passed to the binary when it is launched.
    pub fn args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.args.extend(args.into_iter().map(|s| s.into()));
        self
    }

    /// Sets the display name for this service, which will be used in logging.
    pub fn display_name(mut self, display_name: impl Into<String>) -> Self {
        if self.display_name.is_some() {
            panic!("{} already set", name_of!(display_name in Self));
        }

        self.display_name = Some(display_name.into());
        self
    }

    pub fn get_build_params(&self, target: HostTargetType) -> BuildParams {
        let (bin, example) = match &self.target {
            CrateTarget::Default => (None, None),
            CrateTarget::Bin(bin) => (Some(bin.clone()), None),
            CrateTarget::Example(example) => (None, Some(example.clone())),
        };

        BuildParams::new(
            self.src.clone(),
            bin,
            example,
            self.profile.clone(),
            self.rustflags.clone(),
            self.target_dir.clone(),
            self.build_env.clone(),
            self.no_default_features,
            target,
            self.features.clone(),
            self.config.clone(),
        )
    }
}

impl ServiceBuilder for RustCrate {
    type Service = RustCrateService;
    fn build(self, id: usize, on: Arc<dyn Host>) -> Self::Service {
        let build_params = self.get_build_params(on.target_type());

        RustCrateService::new(
            id,
            on,
            build_params,
            self.tracing,
            Some(self.args),
            self.display_name,
            vec![],
        )
    }
}
