use std::path::PathBuf;
use std::sync::Arc;

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

/// Specifies a crate that uses `hydroflow_deploy_integration` to be
/// deployed as a service.
#[derive(Clone)]
pub struct HydroflowCrate {
    src: PathBuf,
    target: CrateTarget,
    on: Arc<dyn Host>,
    profile: Option<String>,
    rustflags: Option<String>,
    target_dir: Option<PathBuf>,
    no_default_features: bool,
    features: Option<Vec<String>>,
    tracing: Option<TracingOptions>,
    args: Vec<String>,
    display_name: Option<String>,
}

impl HydroflowCrate {
    /// Creates a new `HydroflowCrate` that will be deployed on the given host.
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
            tracing: None,
            args: vec![],
            display_name: None,
        }
    }

    /// Sets the target to be a binary with the given name,
    /// equivalent to `cargo run --bin <name>`.
    pub fn bin(mut self, bin: impl Into<String>) -> Self {
        if self.target != CrateTarget::Default {
            panic!("target already set");
        }

        self.target = CrateTarget::Bin(bin.into());
        self
    }

    /// Sets the target to be an example with the given name,
    /// equivalent to `cargo run --example <name>`.
    pub fn example(mut self, example: impl Into<String>) -> Self {
        if self.target != CrateTarget::Default {
            panic!("target already set");
        }

        self.target = CrateTarget::Example(example.into());
        self
    }

    /// Sets the profile to be used when building the crate.
    /// Equivalent to `cargo run --profile <profile>`.
    pub fn profile(mut self, profile: impl Into<String>) -> Self {
        if self.profile.is_some() {
            panic!("profile already set");
        }

        self.profile = Some(profile.into());
        self
    }

    pub fn rustflags(mut self, rustflags: impl Into<String>) -> Self {
        if self.rustflags.is_some() {
            panic!("rustflags already set");
        }

        self.rustflags = Some(rustflags.into());
        self
    }

    pub fn target_dir(mut self, target_dir: impl Into<PathBuf>) -> Self {
        if self.target_dir.is_some() {
            panic!("target_dir already set");
        }

        self.target_dir = Some(target_dir.into());
        self
    }

    pub fn no_default_features(mut self) -> Self {
        self.no_default_features = true;
        self
    }

    pub fn features(mut self, features: impl IntoIterator<Item = impl Into<String>>) -> Self {
        if self.features.is_some() {
            panic!("features already set");
        }

        self.features = Some(features.into_iter().map(|s| s.into()).collect());
        self
    }

    pub fn tracing(mut self, perf: impl Into<TracingOptions>) -> Self {
        if self.tracing.is_some() {
            panic!("tracing options are already set");
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
            panic!("display_name already set");
        }

        self.display_name = Some(display_name.into());
        self
    }
}

impl ServiceBuilder for HydroflowCrate {
    type Service = HydroflowCrateService;
    fn build(self, id: usize) -> Self::Service {
        let (bin, example) = match self.target {
            CrateTarget::Default => (None, None),
            CrateTarget::Bin(bin) => (Some(bin), None),
            CrateTarget::Example(example) => (None, Some(example)),
        };

        HydroflowCrateService::new(
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
            HydroflowCrate::new("../hydro_cli_examples", deployment.Localhost())
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
