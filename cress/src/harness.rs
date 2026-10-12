//! Runtime support for the harness that [`amplification_check`](crate::amplification_check)
//! generates: how data inputs are given values, how a program's outputs are sunk so the dataflow
//! is not pruned, and where a run's summary is recorded so `cargo-check-amplification` can
//! tabulate it. Nothing here is specific to any program.
//!
//! # Values for data inputs
//!
//! A stream parameter that is not given a `round = ...` closure receives values from
//! [`InputValue::nth`], with `n` counting from zero over the whole run. The implementations for
//! the standard types return the counter itself (integers), a value derived from it (`bool`,
//! `char`, `String`), or the unit value, so request ids come out sequential and distinct, which
//! is what the hand-written harnesses in the corpus did. A program whose input type is its own
//! (an enum of operations, say) implements the trait in a few lines and may use `n` to fix the
//! mix. bolero's `TypeGenerator` was considered and not used: its derive is only reachable from
//! a crate that depends on bolero directly, which a user's crate does not, and its values are
//! arbitrary rather than sequential, which makes a report harder to read against the input.
//!
//! # Sinking outputs
//!
//! A Hydro dataflow whose outputs are not consumed is pruned at compile time, so every stream a
//! program returns must reach a `sim_output`. [`SimOutputs::sink_all`] does that for a single
//! stream, a keyed stream, a tuple of them, and, through `#[derive(SimOutputs)]`, a struct whose
//! fields are streams. The receivers are dropped at once; a `SimReceiver` is only a port id, so
//! dropping it does not close anything.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::Report;
use hydro_lang::live_collections::boundedness::Boundedness;
use hydro_lang::live_collections::keyed_stream::KeyedStream;
use hydro_lang::live_collections::stream::{Ordering, Retries, Stream};
use hydro_lang::location::cluster::Consistency;
use hydro_lang::location::{Cluster, Process};

/// A value the generated harness can feed to a data input. See the [module docs](self).
pub trait InputValue: Sized {
    /// The `n`th value, `n` counting from zero across the whole run. Implementations must be
    /// deterministic; where the type has enough values they should be distinct for distinct `n`.
    fn nth(n: u64) -> Self;
}

impl InputValue for () {
    fn nth(_: u64) -> Self {}
}

impl InputValue for bool {
    fn nth(n: u64) -> Self {
        n % 2 == 1
    }
}

impl InputValue for char {
    fn nth(n: u64) -> Self {
        char::from_u32(('a' as u32) + (n % 26) as u32).unwrap_or('a')
    }
}

impl InputValue for String {
    fn nth(n: u64) -> Self {
        n.to_string()
    }
}

macro_rules! input_value_int {
    ($($t:ty),*) => {
        $(impl InputValue for $t {
            fn nth(n: u64) -> Self {
                n as $t
            }
        })*
    };
}

input_value_int!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);

impl<T: InputValue> InputValue for Option<T> {
    fn nth(n: u64) -> Self {
        Some(T::nth(n))
    }
}

impl<T: InputValue> InputValue for Vec<T> {
    fn nth(n: u64) -> Self {
        vec![T::nth(n)]
    }
}

impl<A: InputValue, B: InputValue> InputValue for (A, B) {
    fn nth(n: u64) -> Self {
        (A::nth(n), B::nth(n))
    }
}

impl<A: InputValue, B: InputValue, C: InputValue> InputValue for (A, B, C) {
    fn nth(n: u64) -> Self {
        (A::nth(n), B::nth(n), C::nth(n))
    }
}

/// Hands out successive [`InputValue`]s. One counter is shared by every data input of a run, so
/// no two inputs receive the same id.
#[derive(Debug, Default)]
pub struct InputCounter {
    next: u64,
}

impl InputCounter {
    /// A counter starting at zero.
    pub fn new() -> Self {
        Self::default()
    }

    /// The next value.
    pub fn next_value<T: InputValue>(&mut self) -> T {
        let v = T::nth(self.next);
        self.next += 1;
        v
    }

    /// Back to zero, so that every run of a check sees the same input.
    pub fn reset(&mut self) {
        self.next = 0;
    }
}

/// Everything a Hydro function may return that the generated harness knows how to sink. See the
/// [module docs](self).
pub trait SimOutputs {
    /// Attach every stream in `self` to a simulation output so the compiler keeps it.
    fn sink_all(self);
}

impl SimOutputs for () {
    fn sink_all(self) {}
}

impl<'a, T, L, B, O, R> SimOutputs for Stream<T, Process<'a, L>, B, O, R>
where
    T: Serialize + DeserializeOwned,
    B: Boundedness,
    O: Ordering,
    R: Retries,
{
    fn sink_all(self) {
        let _ = self.sim_output();
    }
}

impl<'a, T, L, B, C, O, R> SimOutputs for Stream<T, Cluster<'a, L, C>, B, O, R>
where
    T: Serialize + DeserializeOwned,
    B: Boundedness,
    C: Consistency,
    O: Ordering,
    R: Retries,
{
    fn sink_all(self) {
        let _ = self.sim_cluster_output();
    }
}

impl<'a, K, V, L, B, O, R> SimOutputs for KeyedStream<K, V, Process<'a, L>, B, O, R>
where
    K: Serialize + DeserializeOwned,
    V: Serialize + DeserializeOwned,
    B: Boundedness,
    O: Ordering,
    R: Retries,
{
    fn sink_all(self) {
        let _ = self.entries().sim_output();
    }
}

impl<'a, K, V, L, B, C, O, R> SimOutputs for KeyedStream<K, V, Cluster<'a, L, C>, B, O, R>
where
    K: Serialize + DeserializeOwned,
    V: Serialize + DeserializeOwned,
    B: Boundedness,
    C: Consistency,
    O: Ordering,
    R: Retries,
{
    fn sink_all(self) {
        let _ = self.entries().sim_cluster_output();
    }
}

macro_rules! sim_outputs_tuple {
    ($($name:ident),+) => {
        impl<$($name: SimOutputs),+> SimOutputs for ($($name,)+) {
            #[expect(non_snake_case, reason = "the type parameters double as binding names")]
            fn sink_all(self) {
                let ($($name,)+) = self;
                $($name.sink_all();)+
            }
        }
    };
}

sim_outputs_tuple!(A);
sim_outputs_tuple!(A, B);
sim_outputs_tuple!(A, B, C);
sim_outputs_tuple!(A, B, C, D);
sim_outputs_tuple!(A, B, C, D, E);
sim_outputs_tuple!(A, B, C, D, E, F);
sim_outputs_tuple!(A, B, C, D, E, F, G);
sim_outputs_tuple!(A, B, C, D, E, F, G, H);

/// A generated amplification check that can be discovered by the checker command.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct RegisteredCheck {
    pub function: &'static str,
    pub configuration: &'static str,
    pub ignore: Option<&'static str>,
    pub run: fn() -> Vec<CheckResult>,
}

static REGISTERED_CHECKS: OnceLock<Mutex<Vec<RegisteredCheck>>> = OnceLock::new();

/// Adds a generated check to the command's process-local registry.
#[doc(hidden)]
pub fn register_check(check: RegisteredCheck) {
    REGISTERED_CHECKS
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .expect("amplification check registry lock poisoned")
        .push(check);
}

/// The concise result of one workload checked by a registered function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckResult {
    /// The annotated Hydro function.
    pub function: &'static str,
    /// The attribute configuration name.
    pub configuration: &'static str,
    /// The input rates used for this run.
    pub workload: String,
    /// The checker verdict, which is the inferred amplification label.
    pub verdict: crate::Verdict,
    /// How extra work changes as the imposed delay grows.
    pub growth: String,
    /// The largest observed work multiplier, formatted for display.
    pub multiplier: String,
    /// The source location whose delayed input first exposed amplification.
    pub location: Option<String>,
}

/// Runs registered checks whose function name contains `function_filter` and whose configuration
/// exactly matches `configuration_filter` when one is supplied.
///
/// Checks carrying `ignore = "..."` are omitted unless `include_ignored` is true. A hazardous
/// verdict is returned as data, not as an error. An error means that no registered check matched.
pub fn run_registered_checks(
    function_filter: Option<&str>,
    configuration_filter: Option<&str>,
    include_ignored: bool,
) -> Result<Vec<CheckResult>, String> {
    let filter = function_filter.unwrap_or_default();
    let configuration = configuration_filter.unwrap_or_default();
    let mut checks = REGISTERED_CHECKS
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .map_err(|_| "amplification check registry lock poisoned".to_owned())?
        .clone();
    checks.retain(|check| check.function.contains(filter));
    checks.retain(|check| configuration.is_empty() || check.configuration == configuration);
    checks.retain(|check| include_ignored || check.ignore.is_none());
    checks.sort_by_key(|check| (check.function, check.configuration));
    if checks.is_empty() {
        let function_qualifier = if filter.is_empty() {
            String::new()
        } else {
            format!(" matching function `{filter}`")
        };
        let configuration_qualifier = if configuration.is_empty() {
            String::new()
        } else {
            format!(" with configuration `{configuration}`")
        };
        return Err(format!(
            "no amplification checks found{function_qualifier}{configuration_qualifier}"
        ));
    }

    let mut results = Vec::new();
    for check in checks {
        eprintln!(
            "Checking {} ({})…",
            check.function, check.configuration
        );
        results.extend((check.run)());
    }
    Ok(results)
}

/// Prints one polished verdict line per result. Colors are used only on a terminal.
pub fn print_verdicts(results: &[CheckResult]) {
    let color = io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();
    for result in results {
        // Green only for benign; a hazardous verdict and a run that explored nothing are both
        // red, so that "not explored" is never mistaken for a pass.
        let (icon, verdict, code) = match &result.verdict {
            crate::Verdict::Benign => ("✓", "BENIGN", 32),
            crate::Verdict::Hazardous => ("!", "HAZARDOUS", 31),
            crate::Verdict::NotExplored { .. } => ("✗", "NOT EXPLORED", 31),
        };
        let label = if color {
            format!("\x1b[1;{code}m{icon} {verdict}\x1b[0m")
        } else {
            format!("{icon} {verdict}")
        };
        let configuration = if result.configuration == "default" {
            String::new()
        } else {
            format!(" ({})", result.configuration)
        };
        let location = match &result.verdict {
            crate::Verdict::NotExplored { reason } => reason.as_str(),
            _ => result.location.as_deref().unwrap_or("-"),
        };
        println!(
            "{label}  {}{configuration}  growth: {}  multiplier: {}  location: {location}",
            result.function, result.growth, result.multiplier
        );
    }
}

/// What one generated check records for the summary table.
#[derive(Debug)]
pub struct Summary<'r> {
    /// The crate the annotated function lives in (`CARGO_PKG_NAME`).
    pub crate_name: &'static str,
    /// The manifest directory of that crate (`CARGO_MANIFEST_DIR`), used to find `target/`.
    pub manifest_dir: &'static str,
    /// The module path of the annotated function.
    pub module: &'static str,
    /// The annotated function's name.
    pub function: &'static str,
    /// The configuration's name, or `"default"`.
    pub configuration: &'static str,
    /// The per-round rate of every stream input, `name=rate` joined by `, `.
    pub workload: String,
    /// The checker's report.
    pub report: &'r Report,
}

/// The directory results are written to: `$CRESS_RESULTS_DIR` if set, otherwise
/// `target/cress/` under the nearest ancestor of `manifest_dir` that holds a
/// `Cargo.lock`, which is the workspace root.
pub fn results_dir(manifest_dir: &str) -> PathBuf {
    if let Ok(dir) = std::env::var("CRESS_RESULTS_DIR") {
        return PathBuf::from(dir);
    }
    let mut cur: Option<&Path> = Some(Path::new(manifest_dir));
    while let Some(dir) = cur {
        if dir.join("Cargo.lock").exists() {
            return dir.join("target").join("cress");
        }
        cur = dir.parent();
    }
    Path::new(manifest_dir).join("target").join("cress")
}

/// The one-line form of a report for the table: verdict, location, first reaction, extra work,
/// growth word, multiplier.
fn summary_fields(report: &Report) -> BTreeMap<&'static str, String> {
    let mut f = BTreeMap::new();
    f.insert("verdict", report.verdict.to_string());
    f.insert("horizon", report.horizon.to_string());
    f.insert(
        "location",
        report
            .location
            .as_ref()
            .map(|l| crate::readable_hook_name(&l.hook, false))
            .unwrap_or_else(|| "-".to_owned()),
    );
    f.insert(
        "first_reaction",
        report
            .location
            .as_ref()
            .map(|l| l.first_reaction_at.to_string())
            .unwrap_or_else(|| "-".to_owned()),
    );
    f.insert(
        "extra_work",
        report
            .location
            .as_ref()
            .map(|l| format!("{} in {}", l.extra_work, l.rose))
            .unwrap_or_else(|| "-".to_owned()),
    );
    f.insert(
        "growth",
        report
            .growth
            .as_ref()
            .map(|g| g.word())
            .unwrap_or_else(|| "-".to_owned()),
    );
    f.insert(
        "multiplier",
        report
            .growth
            .as_ref()
            .map(|g| match g.multiplier_value() {
                Some(m) => format!("{m:.2}x"),
                None => "from 0".to_owned(),
            })
            .unwrap_or_else(|| "-".to_owned()),
    );
    f.insert("seconds", format!("{:.1}", report.seconds));
    f
}

/// Append one line to `results.tsv` and write the full report to `reports/`. Failures to write
/// are reported on standard error and otherwise ignored, so a missing directory never fails a
/// test.
pub fn record(summary: &Summary<'_>) {
    let dir = results_dir(summary.manifest_dir);
    let fields = summary_fields(summary.report);
    let line = [
        summary.crate_name.to_owned(),
        summary.module.to_owned(),
        summary.function.to_owned(),
        summary.configuration.to_owned(),
        summary.workload.clone(),
        fields["horizon"].clone(),
        fields["verdict"].clone(),
        fields["location"].clone(),
        fields["first_reaction"].clone(),
        fields["extra_work"].clone(),
        fields["growth"].clone(),
        fields["multiplier"].clone(),
        fields["seconds"].clone(),
    ]
    .join("\t");
    let write = || -> io::Result<()> {
        fs::create_dir_all(dir.join("reports"))?;
        let mut tsv = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("results.tsv"))?;
        writeln!(tsv, "{line}")?;
        let file = format!(
            "{}.{}.{}.{}.txt",
            summary.crate_name,
            summary.function,
            summary.configuration,
            summary.workload.replace([' ', ','], "").replace('=', "-")
        );
        fs::write(
            dir.join("reports").join(file),
            format!(
                "{} {} ({}) workload {}\n\n{}",
                summary.module,
                summary.function,
                summary.configuration,
                summary.workload,
                summary.report
            ),
        )?;
        Ok(())
    };
    if let Err(e) = write() {
        eprintln!("cress: could not record results under {}: {e}", dir.display());
    }
}
