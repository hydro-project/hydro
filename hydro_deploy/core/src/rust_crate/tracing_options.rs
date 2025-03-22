#![allow(clippy::too_many_arguments, reason = "buildstructor")]
#![allow(
    unexpected_cfgs,
    reason = "https://github.com/BrynCooke/buildstructor/issues/192"
)]

use std::borrow::Cow;
use std::path::PathBuf;

use inferno::collapse::dtrace::Options as DtraceOptions;
use inferno::collapse::perf::Options as PerfOptions;

type FlamegraphOptions = inferno::flamegraph::Options<'static>;

/// `Cow<'static, str>`.
///
/// `buildstructor` doesn't support `Into<_>` for types with parameters (like `Cow<'static, str>`),
/// so we trick it by defining a type alias.
pub type CowStr = Cow<'static, str>;

#[derive(Clone, buildstructor::Builder)]
#[non_exhaustive] // Prevent direct construction.
pub struct TracingOptions {
    /// Samples per second.
    pub frequency: u32,

    /// Output filename for `dtrace`. Example: `my_worker.stacks`.
    pub dtrace_outfile: Option<PathBuf>,

    /// Output filename for the raw data emitted by `perf record`. Example: `my_worker.perf.data`.
    pub perf_raw_outfile: Option<PathBuf>,

    // /// Output filename for `perf script -i <`[`Self::perf_raw_outfile`]`>`. Example: `my_worker.perf`.
    // pub perf_script_outfile: Option<PathBuf>,
    /// If set, what the write the folded output to.
    pub fold_outfile: Option<PathBuf>,
    pub fold_dtrace_options: Option<DtraceOptions>,
    pub fold_perf_options: Option<PerfOptions>,
    /// If set, what to write the output flamegraph SVG file to.
    pub flamegraph_outfile: Option<PathBuf>,
    // This type is super annoying and isn't `clone` and has a lifetime... so wrap in fn pointer for now.
    pub flamegraph_options: Option<fn() -> FlamegraphOptions>,

    /// Command to setup tracing before running the command, i.e. to install `perf` or set kernel flags.
    ///
    /// NOTE: Currently is only run for remote/cloud ssh hosts, not local hosts.
    ///
    /// Example: see [`DEBIAN_PERF_SETUP_COMMAND`].
    pub setup_command: Option<CowStr>,
}

pub const DEBIAN_PERF_SETUP_COMMAND: &str = "sudo sh -c 'apt update && apt install -y linux-perf binutils && echo -1 > /proc/sys/kernel/perf_event_paranoid && echo 0 > /proc/sys/kernel/kptr_restrict'";
