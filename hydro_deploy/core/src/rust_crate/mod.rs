use std::path::PathBuf;
use std::sync::Arc;

use nameof::name_of;
use tracing_options::TracingOptions;

use super::Host;
use crate::ServiceBuilder;

pub(crate) mod build;
pub mod ports;

pub mod service;
pub use service::*;

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
#[derive(Clone)]
pub struct RustCrate {
    src: PathBuf,
    target: CrateTarget,
    on: Arc<dyn Host>,
    profile: Option<String>,
    rustflags: Option<String>,
    target_dir: Option<PathBuf>,
    no_default_features: bool,
    features: Option<Vec<String>>,
    config: Option<String>,
    tracing: Option<TracingOptions>,
    args: Vec<String>,
    display_name: Option<String>,
}

impl RustCrate {
    /// Creates a new `RustCrate` that will be deployed on the given host.
    /// The `src` argument is the path to the crate's directory, and the `on`
    /// argument is the host that the crate will be deployed on.
    pub fn new(src: impl Into<PathBuf>, on: Arc<dyn Host>) -> Self {
        Self {
            src: src.into(),
            target: CrateTarget::Default,
            on,
            profile: None,
            rustflags: None,
            target_dir: None,
            no_default_features: false,
            features: None,
            config: None,
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
        if self.config.is_some() {
            panic!("{} already set", name_of!(config in Self));
        }

        self.config = Some(config.into());
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
}

impl ServiceBuilder for RustCrate {
    type Service = RustCrateService;
    fn build(self, id: usize) -> Self::Service {
        let (bin, example) = match self.target {
            CrateTarget::Default => (None, None),
            CrateTarget::Bin(bin) => (Some(bin), None),
            CrateTarget::Example(example) => (None, Some(example)),
        };

        RustCrateService::new(
            id,
            self.src,
            self.on,
            bin,
            example,
            self.profile,
            self.rustflags,
            self.target_dir,
            self.no_default_features,
            self.tracing,
            self.features,
            self.config,
            Some(self.args),
            self.display_name,
            vec![],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deployment;

    #[tokio::test]
    async fn test_crate_panic() {
        let mut deployment = deployment::Deployment::new();

        let service = deployment.add_service(
            RustCrate::new("../hydro_cli_examples", deployment.Localhost())
                .example("panic_program")
                .profile("dev"),
        );

        deployment.deploy().await.unwrap();

        let mut stdout = service.try_read().unwrap().stdout();

        deployment.start().await.unwrap();

        assert_eq!(stdout.recv().await.unwrap(), "hello!");

        assert!(stdout.recv().await.is_none());
    }
}
