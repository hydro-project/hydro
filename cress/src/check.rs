//! The checker proper: [`check`], its configuration, and its report. Re-exported at the crate
//! root.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::panic::RefUnwindSafe;
use std::time::Instant;

use hydro_lang::sim::compiled::CompiledSim;
use hydro_lang::sim::flow::SimFlow;
use hydro_lang::sim::prompt_schedule::PromptScheduleDriver;

use crate::schedule::{Counts, Instrumented, Schedule, instrument};
pub use crate::schedule::NotScriptable;

/// The records one run moved, by kind and by place: what the verdict reads and what the report
/// shows. Collected from the simulator's hooks (records admitted at each `use::batch`, read from
/// the hook's own buffer count just before each release) and from a counting output on every
/// network send (messages per sending location and member). See the [crate docs](crate) for why
/// these two counts, and not others, are fit for the verdict.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkCounts {
    /// Records released into ticks by each `use::batch` hook, keyed by the hook's
    /// `file:line#index [item type]` identity, summed over cluster members.
    pub hook_releases: BTreeMap<String, u64>,
    /// The same, keyed by hook identity and cluster member (`None` for a process).
    pub hook_releases_by_member: BTreeMap<(String, Option<u32>), u64>,
    /// Network messages sent, keyed by the sending location, summed over members.
    pub network_messages: BTreeMap<String, u64>,
    /// The same, keyed by sending location and member.
    pub network_messages_by_member: BTreeMap<(String, Option<u32>), u64>,
}

impl WorkCounts {
    /// Total records released into ticks by `use::batch` hooks: the checker's work measure.
    pub fn admitted(&self) -> u64 {
        self.hook_releases.values().sum()
    }

    /// Total network messages sent.
    pub fn network(&self) -> u64 {
        self.network_messages.values().sum()
    }

    /// Each cluster member's (or process's) outgoing network messages, keyed
    /// `sends from Location @member`. These are the places the checker's verdict reads alongside
    /// [`WorkCounts::admitted`]: a message is work one location imposes on another, and it is
    /// counted per sender because a reaction at one member can be hidden in a total by another
    /// member going quiet.
    pub fn sends_by_member(&self) -> BTreeMap<String, u64> {
        self.network_messages_by_member
            .iter()
            .map(|((loc, member), n)| (format!("sends from {loc}{}", member_suffix(*member)), *n))
            .collect()
    }

    /// Every `use::batch` hook on every member, keyed `hook_id @member`. Reported for
    /// localization; not read by the verdict.
    pub fn hooks_by_member(&self) -> BTreeMap<String, u64> {
        self.hook_releases_by_member
            .iter()
            .map(|((hook, member), n)| (format!("{hook}{}", member_suffix(*member)), *n))
            .collect()
    }

    fn from_schedule(counts: &Counts, schedule: &Schedule<'_>) -> Self {
        let mut out = WorkCounts::default();
        for (instance, n) in &counts.admitted {
            let id = schedule.identity(instance.id);
            *out.hook_releases.entry(id.clone()).or_default() += n;
            *out
                .hook_releases_by_member
                .entry((id, instance.member))
                .or_default() += n;
        }
        for ((loc, member), n) in &counts.sent {
            let loc = format!("{loc:?}");
            *out.network_messages.entry(loc.clone()).or_default() += n;
            *out
                .network_messages_by_member
                .entry((loc, *member))
                .or_default() += n;
        }
        out
    }
}

fn member_suffix(member: Option<u32>) -> String {
    member.map(|m| format!(" @{m}")).unwrap_or_default()
}

/// The horizon [`CheckConfig::default`] uses, in rounds. See [`CheckConfig::rounds`] for how it
/// was chosen.
pub const DEFAULT_HORIZON: usize = 1000;

/// How long to run. The hold lengths are not configurable; see the [crate docs](crate).
#[derive(Debug, Clone)]
pub struct CheckConfig {
    /// Rounds of workload per run: the horizon. A benign verdict means no reaction was found to
    /// a delay of up to `rounds - hold_start` rounds. The default, [`DEFAULT_HORIZON`], is
    /// large because the horizon bounds the longest delay the checker can impose and a program
    /// whose reaction begins beyond it reads benign; the cost is roughly linear in the horizon
    /// and is printed with every report, so a caller who finds the default too slow can lower it
    /// knowingly. On the corpus a 240-round run costs about two seconds, so one hook costs
    /// about `2 s × (log2(rounds) + 1) × rounds / 240`, which at the default is roughly 90
    /// seconds per hook and, for a program with seven hooks, about ten minutes.
    pub rounds: usize,
    /// The round at which every hold begins. This is not a tuning knob: a hold delays whatever
    /// arrives at the held hook after it begins, and the program's reaction depends on how long
    /// the delay lasts, not on when it starts, so the checker starts every hold at round 1, after
    /// one round of ordinary operation has created the hooks and their state. It is kept as a
    /// field so that a report can say exactly which rounds were held.
    pub hold_start: usize,
    /// Whether to print one line to standard error as each hook's escalation completes.
    pub progress: bool,
    /// The rounds in which the workload closure sends the program's data, `0..workload_rounds`.
    /// The default is `rounds`: data arrives in every round and the growth label cannot separate
    /// the input from the delay (see the [crate docs](crate)). When it is smaller, the closure
    /// is still called for every round, and is expected to send only timer elements once the
    /// data is over; the checker then reads the holds that outlast the input for the growth
    /// label. The checker does not enforce what the closure sends.
    pub workload_rounds: usize,
}

impl Default for CheckConfig {
    fn default() -> Self {
        Self::new(DEFAULT_HORIZON)
    }
}

impl CheckConfig {
    /// A run of `rounds` rounds with every hold starting at round 1, data in every round, and
    /// progress printed.
    pub fn new(rounds: usize) -> Self {
        Self {
            rounds,
            hold_start: 1,
            progress: true,
            workload_rounds: rounds,
        }
    }

    /// Turns progress printing off.
    pub fn quiet(mut self) -> Self {
        self.progress = false;
        self
    }

    /// Confines the data to the first `workload_rounds` rounds so that holds can outlast it.
    /// For the growth label to be read, at least two finite hold lengths must outlast the
    /// input, which with the doubling sequence means `workload_rounds` at most about a quarter
    /// of `rounds`; [`CheckConfig::holds_beyond_input`] says which lengths qualify.
    pub fn with_workload_rounds(mut self, workload_rounds: usize) -> Self {
        assert!(
            workload_rounds <= self.rounds,
            "workload_rounds ({workload_rounds}) cannot exceed rounds ({})",
            self.rounds
        );
        self.workload_rounds = workload_rounds;
        self
    }

    /// The finite hold lengths that outlast the input: a hold from `hold_start` for `k` rounds
    /// covers every data round after `hold_start` when `hold_start + k >= workload_rounds`. The
    /// hold to the end of the run is excluded, because its records are never delivered and its
    /// reaction is read separately as the stall reaction.
    pub fn holds_beyond_input(&self) -> Vec<usize> {
        let horizon = self.horizon();
        self.hold_lengths()
            .into_iter()
            .filter(|&k| k < horizon && self.hold_start + k >= self.workload_rounds)
            .collect()
    }

    /// Whether the run separates the input from the delay: at least two finite holds outlast
    /// the input, so a rate of extra work per round of delay can be read with the input fixed.
    pub fn separates_input(&self) -> bool {
        self.holds_beyond_input().len() >= 2
    }

    /// The longest delay the checker will impose: a hold from `hold_start` to the end of the run.
    pub fn horizon(&self) -> usize {
        self.rounds.saturating_sub(self.hold_start)
    }

    /// The hold lengths tried for every hook: 1, 2, 4, ... while below the horizon, then the
    /// horizon itself, which is a hold that lasts to the end of the run.
    pub fn hold_lengths(&self) -> Vec<usize> {
        let max = self.horizon();
        let mut ks = Vec::new();
        let mut k = 1usize;
        while k < max {
            ks.push(k);
            k *= 2;
        }
        if max > 0 {
            ks.push(max);
        }
        ks
    }
}

/// The checker's answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Some hold raised a member's outgoing messages or the program's admitted records.
    Hazardous,
    /// Every hold of every decision point was tried and none added any work.
    Benign,
    /// Nothing could be held, so no schedule was tried and nothing is known. This is not benign:
    /// the unheld run found no decision point with buffered input (the program has no
    /// `use::batch` or `use::snapshot` the simulator can script, or the workload sent nothing
    /// into it), or it moved no records and sent no messages at all.
    NotExplored {
        /// Why: what the unheld run found.
        reason: String,
    },
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Verdict::Hazardous => f.write_str("hazardous"),
            Verdict::Benign => f.write_str("benign"),
            Verdict::NotExplored { .. } => f.write_str("not explored"),
        }
    }
}

/// A guess at the shape of one count's extra work as a function of the hold length, read only
/// on the holds that outlast the input, where the input is fixed and only the delay grows. The
/// verdict does not depend on it. Each shape carries the constants of a formula in the hold
/// length `k`, and the report pins every constant to the decision point whose hold produced it.
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    /// The count is at or below the unheld run's at every hold that outlasts the input: the
    /// program sheds work under a long delay rather than adding it. Its rise toward the
    /// unheld run, if any, is not amplification.
    Shed {
        /// The smallest extra work over those holds.
        low: i64,
        /// The largest extra work over those holds, at most zero.
        high: i64,
    },
    /// `extra ≈ a`: over the last interval between holds that outlast the input, the extra work
    /// rises by at most one record, which is the resolution of an integer count against the
    /// phase of the program's timers at the release round. `a` is the value at the last hold.
    Flat {
        /// The extra work at the last hold.
        plateau: i64,
        /// The first hold from which every value is within one record of the plateau. When it
        /// is later than the first hold that outlasts the input, the reaction to the last data
        /// was still completing (or draining) at the earlier holds.
        reached_at: usize,
    },
    /// `extra ≈ a + b·(k − k₀)`: the extra work rises at a steady rate per round of delay,
    /// where `k₀` is the first hold that outlasts the input. The marginal rates between
    /// consecutive holds neither double nor halve across a doubling of the hold.
    Linear {
        /// The extra work at `k₀`.
        a: i64,
        /// The last marginal rate, per round of delay.
        b: f64,
    },
    /// `extra ≈ a + b·(k − k₀)^d` with `d ≥ 2`: the marginal rate itself at least doubles
    /// across a doubling of the hold. `d` is `1 + log2(r_last / r_previous)` rounded down, so
    /// a rate that is rising but has not doubled reads as linear.
    Polynomial {
        /// The extra work at `k₀`.
        a: i64,
        /// The last marginal rate, per round of delay.
        b: f64,
        /// The guessed degree.
        degree: u32,
    },
    /// The extra work rises but its marginal rate falls across doublings of the hold: the
    /// rise is slowing.
    Sublinear {
        /// The extra work at `k₀`.
        a: i64,
        /// The last marginal rate, per round of delay.
        b: f64,
    },
}

impl Shape {
    /// The shape's class and last rate, for ranking: polynomial above linear above sublinear
    /// above flat and shedding. See [`shape_order`] for how the report breaks ties.
    fn rank(&self) -> (u32, f64) {
        match self {
            Shape::Shed { .. } => (0, 0.0),
            Shape::Flat { .. } => (0, 0.0),
            Shape::Sublinear { b, .. } => (1, *b),
            Shape::Linear { b, .. } => (2, *b),
            Shape::Polynomial { degree, b, .. } => (2 + *degree, *b),
        }
    }

    /// One word for the table.
    pub fn word(&self) -> String {
        match self {
            Shape::Shed { .. } => "shedding".to_owned(),
            Shape::Flat { .. } => "stops".to_owned(),
            Shape::Sublinear { .. } => "grows, slowing".to_owned(),
            Shape::Linear { .. } => "grows".to_owned(),
            Shape::Polynomial { degree, .. } => format!("grows, degree {degree}"),
        }
    }
}

/// The extra work of one count beyond the input, and the shape guessed from it.
#[derive(Debug, Clone, PartialEq)]
pub struct Growth {
    /// The hold lengths read: every finite hold that outlasts the input, in order.
    pub holds: Vec<usize>,
    /// The extra work at each of those holds.
    pub extra: Vec<i64>,
    /// The marginal rate between each pair of consecutive holds, per round of delay.
    pub rates: Vec<f64>,
    /// The guessed shape and its constants.
    pub shape: Shape,
}

impl Growth {
    /// Guesses the shape from extra work at consecutive holds. `holds` and `extra` have the
    /// same length, at least two.
    fn fit(holds: Vec<usize>, extra: Vec<i64>) -> Self {
        let rates: Vec<f64> = holds
            .windows(2)
            .zip(extra.windows(2))
            .map(|(k, e)| (e[1] - e[0]) as f64 / (k[1] - k[0]) as f64)
            .collect();
        let a = extra[0];
        let low = *extra.iter().min().unwrap();
        let high = *extra.iter().max().unwrap();
        let n = extra.len();
        let last = extra[n - 1];
        let last_rise = last - extra[n - 2];
        let shape = if high <= 0 {
            Shape::Shed { low, high }
        } else if last_rise <= 1 {
            // The first hold from which every value stays within one record of the last.
            let first_stable = (0..n)
                .find(|&i| extra[i..].iter().all(|e| (e - last).abs() <= 1))
                .unwrap_or(n - 1);
            Shape::Flat {
                plateau: last,
                reached_at: holds[first_stable],
            }
        } else {
            let b = *rates.last().unwrap();
            match rates.len() {
                0 | 1 => Shape::Linear { a, b },
                m => {
                    let prev = rates[m - 2];
                    if prev <= 0.0 {
                        Shape::Linear { a, b }
                    } else {
                        // A marginal rate that at least doubles across a doubling of the hold
                        // reads as degree two; anything less is linear with a rate still
                        // settling, which the printed marginal rates show.
                        let degree = 1.0 + (b / prev).log2();
                        let d = degree.floor();
                        if d >= 2.0 {
                            Shape::Polynomial {
                                a,
                                b,
                                degree: d as u32,
                            }
                        } else if d <= 0.0 {
                            Shape::Sublinear { a, b }
                        } else {
                            Shape::Linear { a, b }
                        }
                    }
                }
            }
        };
        Growth {
            holds,
            extra,
            rates,
            shape,
        }
    }

    /// The reading for a reader: whether the extra work stops once the hold outlasts the input,
    /// and the ceiling if it does or the rate if it does not. The fitted constants stay in
    /// [`Shape`]; the printed form uses the words a reader needs. The marginal rates between
    /// the holds read are always printed so the guess can be checked.
    fn describe(&self, count: &str) -> String {
        let rates = self
            .rates
            .iter()
            .map(|r| format!("{r:.3}"))
            .collect::<Vec<_>>()
            .join(", ");
        match &self.shape {
            Shape::Shed { low, high } => format!(
                "extra {count} stops and stays at or below the unheld run ({low}..{high}): shedding, not amplifying"
            ),
            Shape::Flat {
                plateau,
                reached_at,
            } if *reached_at == self.holds[0] => {
                let range = self.extra.iter().max().unwrap() - self.extra.iter().min().unwrap();
                format!(
                    "extra {count} stops: ceiling {plateau}{} (rates per round of delay {rates})",
                    if range == 0 { "" } else { ", within one record" }
                )
            }
            Shape::Flat {
                plateau,
                reached_at,
            } => {
                let before: Vec<String> = self
                    .holds
                    .iter()
                    .zip(&self.extra)
                    .filter(|(k, _)| **k < *reached_at)
                    .map(|(k, e)| format!("{e} at {k}"))
                    .collect();
                format!(
                    "extra {count} stops: ceiling {plateau} from a hold of {reached_at} rounds, still rising before that ({}) (rates per round of delay {rates})",
                    before.join(", ")
                )
            }
            Shape::Linear { b, .. } => format!(
                "extra {count} keeps growing: rate {b:.2} per round of delay, no ceiling within the run (rates {rates})"
            ),
            Shape::Polynomial { b, degree, .. } => format!(
                "extra {count} keeps growing and the rate itself grows: last rate {b:.2} per round of delay, degree {degree} (rates {rates})"
            ),
            Shape::Sublinear { b, .. } => format!(
                "extra {count} keeps growing but more slowly: last rate {b:.2} per round of delay and falling (rates {rates})"
            ),
        }
    }
}

/// One count's reaction under one held hook, across every hold length: how far it rose, what
/// it did under a hold that never ends, and, when the run separates the input from the delay,
/// how it grows once the hold outlasts the input.
#[derive(Debug, Clone, PartialEq)]
pub struct SeriesGrowth {
    /// `"admitted records"` or a `sends from ...` place.
    pub count: String,
    /// The count in the unheld run.
    pub reference: u64,
    /// The largest extra work at any hold length, including the hold to the end of the run.
    pub worst: i64,
    /// The hold length at which `worst` was reached.
    pub worst_at: usize,
    /// The extra work under the hold that lasts to the end of the run: the reaction to
    /// records that never arrive.
    pub stall: i64,
    /// Present when at least two finite holds outlast the input.
    pub beyond: Option<Growth>,
}

impl SeriesGrowth {
    /// `(reference + worst) / reference`, the count under the worst hold as a multiple of the
    /// unheld run's; `None` when the unheld run had none of this count.
    pub fn multiplier(&self) -> Option<f64> {
        (self.reference > 0)
            .then(|| (self.reference as i64 + self.worst) as f64 / self.reference as f64)
    }

    fn multiplier_text(&self) -> String {
        match self.multiplier() {
            Some(m) => format!(
                "{m:.2}× the unheld run's {} {} (+{} at a hold of {} rounds)",
                self.reference, self.count, self.worst, self.worst_at
            ),
            None => format!(
                "+{} {} where the unheld run had none (at a hold of {} rounds)",
                self.worst, self.count, self.worst_at
            ),
        }
    }
}

/// The growth label of a hazardous report: a guess, separate from the verdict, at how the
/// extra work behaves. See the [crate docs](crate).
#[derive(Debug, Clone, PartialEq)]
pub struct GrowthLabel {
    /// The finite hold lengths that outlast the input; empty or one long when the run does not
    /// separate the input from the delay.
    pub holds_beyond_input: Vec<usize>,
    /// The worst shape over every reacting decision point, with the hook it was read at and
    /// the series it was read from. `None` when the input is not separated.
    pub shape: Option<(String, SeriesGrowth)>,
    /// The largest multiplier over every reacting decision point and count, with its hook.
    pub multiplier: (String, SeriesGrowth),
    /// The largest stall reaction over every reacting decision point and count, with its hook.
    pub stall: (String, SeriesGrowth),
    /// The length of the hold that never ends, for the stall reaction's per-round figure.
    pub horizon: usize,
}

/// What one held hook did to the counts, across the hold lengths tried.
#[derive(Debug, Clone)]
pub struct HookCurve {
    /// The hook's identity, `file:line#index [item type]` (see
    /// `Schedule::identity` in the `schedule` module).
    pub hook: String,
    /// `(k, admitted records minus the unheld run's)` for each hold length, including `k = 0`.
    pub extra_admitted: Vec<(usize, i64)>,
    /// `(k, per-sender messages minus the unheld run's)` for each hold length.
    pub extra_sends_by_member: Vec<(usize, BTreeMap<String, i64>)>,
    /// The full counts at each hold length.
    pub counts: Vec<(usize, WorkCounts)>,
    /// `(k, how many times a tick containing the held hook ran for another hook's sake while the
    /// held hook had something waiting)` for each hold length with `k > 0`. Zero at some `k`
    /// does not mean the hold had no effect: a tick whose only loaded hook is held is not
    /// runnable, so it parks and the held hook is never consulted. That is how a hold on a
    /// single-hook tick (a server whose only input is its arrivals) takes effect.
    pub holds_applied: Vec<(usize, u64)>,
    /// The extra work this hold caused: the largest rise, over the unheld run, of admitted
    /// records or of any one sender's messages, at any hold length.
    pub extra_work: i64,
    /// What rose to give `extra_work`: `"admitted records"` or a `sends from ...` place.
    pub rose: Option<String>,
    /// The smallest hold length at which any extra work appeared.
    pub first_reaction_at: Option<usize>,
    /// The extra work at `first_reaction_at` (admitted records or one sender's messages), used
    /// to rank hooks that first reacted at the same length.
    pub extra_work_at_first_reaction: i64,
    /// Every count that rose under this hook at some hold length, with its worst rise, its
    /// stall reaction, and its growth beyond the input when the run separates the input from
    /// the delay. Counts that never rose are omitted.
    pub series: Vec<SeriesGrowth>,
}

impl HookCurve {
    /// The series whose shape ranks worst (see [`Shape`]); a tie goes to a `sends from` series
    /// over the admitted total, because messages are the physical unit and name a location,
    /// and then to the larger worst rise. `None` when no count rose or the input is not
    /// separated.
    pub fn worst_shape(&self) -> Option<&SeriesGrowth> {
        self.series
            .iter()
            .filter(|s| s.beyond.is_some())
            .max_by(|x, y| shape_order(x, y))
    }

    /// The `sends from` series whose shape ranks worst, and the admitted-records series, when
    /// each exists and has a growth reading; these are the two units a report shows per hook.
    fn shape_series(&self) -> Vec<&SeriesGrowth> {
        let mut out = Vec::new();
        if let Some(s) = self
            .series
            .iter()
            .filter(|s| s.beyond.is_some() && s.count != "admitted records")
            .max_by(|x, y| shape_order(x, y))
        {
            out.push(s);
        }
        if let Some(s) = self
            .series
            .iter()
            .find(|s| s.beyond.is_some() && s.count == "admitted records")
        {
            out.push(s);
        }
        out
    }

    /// The series with the largest multiplier; a count the unheld run lacked entirely ranks
    /// above every finite multiplier. `None` when no count rose.
    pub fn worst_multiplier(&self) -> Option<&SeriesGrowth> {
        self.series.iter().max_by(|x, y| {
            let kx = x.multiplier().unwrap_or(f64::INFINITY);
            let ky = y.multiplier().unwrap_or(f64::INFINITY);
            kx.partial_cmp(&ky)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(x.worst.cmp(&y.worst))
        })
    }

    /// The series with the largest stall reaction. `None` when no count rose.
    pub fn worst_stall(&self) -> Option<&SeriesGrowth> {
        self.series.iter().max_by_key(|s| s.stall)
    }

    /// The extra admitted records at each hold length, excluding the unheld run.
    fn admitted_row(&self) -> Vec<i64> {
        self.extra_admitted
            .iter()
            .filter(|(k, _)| *k > 0)
            .map(|(_, e)| *e)
            .collect()
    }

    /// The extra messages from the busiest sender at each hold length, excluding the unheld run.
    fn sends_row(&self) -> Vec<i64> {
        self.extra_sends_by_member
            .iter()
            .filter(|(k, _)| *k > 0)
            .map(|(_, m)| m.values().copied().max().unwrap_or(0))
            .collect()
    }

    /// The extra messages summed over every sender at each hold length, excluding the unheld
    /// run. Read beside [`Self::sends_row`]: when the busiest sender rose and the sum did not,
    /// the work moved from one sender to another (a leader changed, say) rather than being
    /// added. The verdict reads the busiest sender, so that a quiet sender cannot hide a busy
    /// one; this row lets the reader tell the two cases apart.
    fn sends_total_row(&self) -> Vec<i64> {
        self.extra_sends_by_member
            .iter()
            .filter(|(k, _)| *k > 0)
            .map(|(_, m)| m.values().sum())
            .collect()
    }

    /// Whether the program sent any network message in any run, so that the messages rows mean
    /// something.
    fn has_sends(&self) -> bool {
        self.counts.iter().any(|(_, c)| c.network() > 0)
    }

    /// The hook's identity rendered for a reader: `file:line, batch of T` or
    /// `file:line, snapshot of T`. The index is shown only when `show_index` is set, which a
    /// report does for decision points that share a line.
    pub fn readable_name(&self, show_index: bool) -> String {
        readable_hook_name(&self.hook, show_index)
    }

    /// The lines the checker prints for this hook. A hook whose hold caused no extra work at
    /// any length prints one line; otherwise the name and the curves (records admitted, messages
    /// from the busiest sender, and, when the program sends anything, messages from all
    /// senders), aligned under the hold-length header the report prints once.
    fn lines(&self, width: usize, show_index: bool) -> Vec<String> {
        let mut out = Vec::new();
        let admitted = self.admitted_row();
        let sends = self.sends_row();
        let name = self.readable_name(show_index);
        let quiet = admitted.iter().all(|e| *e <= 0) && sends.iter().all(|e| *e <= 0);
        let mut notes = Vec::new();
        let unasked = self.unasked_while_held_at();
        if !unasked.is_empty() {
            let at = if unasked.len() == self.holds_applied.len() {
                "at every hold length".to_owned()
            } else if unasked.len() == 1 {
                format!("at hold length {}", unasked[0])
            } else {
                format!("at hold lengths {}", list(&unasked))
            };
            notes.push(format!(
                "held by parking the tick, which has no other input, {at}"
            ));
        }
        if quiet {
            let mut line = format!("{name}: no extra work at any hold length");
            if !notes.is_empty() {
                line.push_str(&format!(" ({})", notes.join("; ")));
            }
            out.push(line);
            return out;
        }
        out.push(name);
        out.push(format!(
            "{:<width$}{}",
            "    extra records admitted",
            row(&admitted),
            width = width
        ));
        out.push(format!(
            "{:<width$}{}",
            "    extra messages, busiest sender",
            row(&sends),
            width = width
        ));
        if self.has_sends() {
            out.push(format!(
                "{:<width$}{}",
                "    extra messages, all senders",
                row(&self.sends_total_row()),
                width = width
            ));
        }
        let mut summary = format!(
            "    first reacted at a hold of {} rounds with {} extra; largest extra work {} in {}",
            self.first_reaction_at.unwrap_or(0),
            self.extra_work_at_first_reaction,
            self.extra_work,
            self.rose.as_deref().unwrap_or("nothing")
        );
        if !notes.is_empty() {
            summary.push_str(&format!(" ({})", notes.join("; ")));
        }
        out.push(summary);
        for line in self.growth_lines() {
            out.push(format!("    {line}"));
        }
        out
    }

    /// The growth lines for this hook: one for the count with the worst shape beyond the
    /// input (or a note that the input was not separated), one for the multiplier, one for
    /// the stall reaction. Empty when no count rose.
    fn growth_lines(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.series.is_empty() {
            return out;
        }
        match self.worst_shape() {
            Some(_) => {
                for s in self.shape_series() {
                    let g = s.beyond.as_ref().unwrap();
                    out.push(format!(
                        "beyond the input (holds {}): {}",
                        list(&g.holds),
                        g.describe(&s.count)
                    ));
                }
            }
            None => out.push(
                "beyond the input: not read, because fewer than two finite holds outlast the input"
                    .to_owned(),
            ),
        }
        if let Some(m) = self.worst_multiplier() {
            out.push(format!("multiplier: {}", m.multiplier_text()));
        }
        if let Some(s) = self.worst_stall() {
            let rising = self
                .worst_shape()
                .is_some_and(|w| w.beyond.as_ref().unwrap().shape.rank().0 > 0);
            out.push(format!(
                "under a hold that never ends: {}",
                stall_text(s, self.counts.last().map(|(k, _)| *k).unwrap_or(0), rising)
            ));
        }
        out
    }

    /// The `use::batch` or `use::snapshot` source position as `file:line`, without the index
    /// and item type.
    pub fn source_location(&self) -> &str {
        hook_line(&self.hook)
    }

    /// Whether the held hook was a `use::snapshot` (its identity carries the word `snapshot`).
    pub fn is_snapshot(&self) -> bool {
        self.hook.contains("[snapshot ")
    }

    /// Hold lengths at which no tick containing the held hook ran while it had something
    /// waiting. Its tick parked for the whole hold (see [`HookCurve::holds_applied`]).
    pub fn unasked_while_held_at(&self) -> Vec<usize> {
        self.holds_applied
            .iter()
            .filter(|(_, n)| *n == 0)
            .map(|(k, _)| *k)
            .collect()
    }
}

/// Where the hazard was found.
#[derive(Debug, Clone)]
pub struct Location {
    /// The held hook that reacted to the shortest delay (see the [crate docs](crate)).
    pub hook: String,
    /// That hook's `use::batch` or `use::snapshot` source location.
    pub source_location: String,
    /// What rose to give `extra_work`: `"admitted records"` or a `sends from ...` place.
    pub rose: String,
    /// The largest extra work over the unheld run at any hold length.
    pub extra_work: i64,
    /// The smallest hold length at which extra work appeared.
    pub first_reaction_at: usize,
    /// The extra work at `first_reaction_at`.
    pub extra_work_at_first_reaction: i64,
    /// Every hook that admitted more records under the winning hold than in the unheld run,
    /// at the hold length where the winning hook's admitted total peaked, with the rise. These
    /// are the edges the extra records crossed.
    pub hooks_that_rose: BTreeMap<String, i64>,
}

impl Location {
    /// The hook's identity rendered for a reader: `file:line, batch of T` or
    /// `file:line, snapshot of T`. The index is shown only when `show_index` is set, which a
    /// report does for decision points that share a line.
    pub fn readable_name(&self, show_index: bool) -> String {
        readable_hook_name(&self.hook, show_index)
    }
}

/// Everything [`check`] measured.
#[derive(Debug, Clone)]
pub struct Report {
    /// Hazardous if some hold raised a count; benign if every hold was tried and none did; not
    /// explored if there was nothing to hold.
    pub verdict: Verdict,
    /// Rounds of workload in every run.
    pub rounds: usize,
    /// The round at which every hold began.
    pub hold_start: usize,
    /// The longest delay imposed, in rounds: a hold from `hold_start` to the end of the run. A
    /// benign verdict is a statement about delays up to this length.
    pub horizon: usize,
    /// The hold lengths tried for every hook, in order, not including the unheld run.
    pub hold_lengths: Vec<usize>,
    /// The rounds in which data arrived, `0..workload_rounds`.
    pub workload_rounds: usize,
    /// Counts from the unheld run.
    pub baseline: WorkCounts,
    /// One curve per hook the discovery run found with buffered input.
    pub curves: Vec<HookCurve>,
    /// Decision points the simulator cannot script, and so the checker cannot hold: their
    /// `file:line` and the reason. Their decisions follow the prompt schedule in every run.
    pub not_holdable: Vec<NotScriptable>,
    /// Present when the verdict is hazardous.
    pub location: Option<Location>,
    /// Present when the verdict is hazardous: the growth label, a guess separate from the
    /// verdict.
    pub growth: Option<GrowthLabel>,
    /// Wall clock for the whole check, including compilation.
    pub seconds: f64,
}

/// The width of the label column in the printed report, so that every curve's numbers line up
/// under the hold-length header.
const LABEL_WIDTH: usize = 36;

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "cress amplification check: {} rounds of workload, every hold begins at round {}",
            self.rounds, self.hold_start
        )?;
        writeln!(
            f,
            "horizon: {} rounds (the longest delay imposed; a hold of that length lasts to the end of the run)",
            self.horizon
        )?;
        writeln!(f, "run time: {:.1} s including compilation", self.seconds)?;
        if self.workload_rounds < self.rounds {
            writeln!(
                f,
                "input: data in rounds 0..{}, timers only afterwards; finite holds that outlast the input: {}",
                self.workload_rounds,
                if self.growth_holds().is_empty() {
                    "none".to_owned()
                } else {
                    list(&self.growth_holds())
                }
            )?;
        } else {
            writeln!(
                f,
                "input: data in every round, so no hold outlasts the input and the growth label gives only the multiplier"
            )?;
        }
        writeln!(
            f,
            "unheld run: {} records admitted at decision points, {} network messages sent, {} decision points had buffered input",
            self.baseline.admitted(),
            self.baseline.network(),
            self.curves.len()
        )?;
        writeln!(f)?;
        writeln!(
            f,
            "A decision point is named by the file and line of its own use::batch or use::snapshot expression, then its kind: a batch admits the records waiting at an edge, a snapshot reads a version of a piece of state. When two decision points share a line, #n gives each one's position within its step. Each curve gives extra work over the unheld run, at each hold length: records admitted, messages from the busiest sender (what the verdict reads), and messages from all senders together (which stays flat when work only moved from one sender to another)."
        )?;
        let shared = shared_lines(self.curves.iter().map(|c| c.hook.as_str()));
        let show_index = |hook: &str| shared.contains(hook_line(hook));
        writeln!(f)?;
        writeln!(
            f,
            "{:<width$}{}",
            "hold length (rounds)",
            row_usize(&self.hold_lengths),
            width = LABEL_WIDTH
        )?;
        for c in &self.curves {
            for line in c.lines(LABEL_WIDTH, show_index(&c.hook)) {
                writeln!(f, "{line}")?;
            }
        }
        for n in &self.not_holdable {
            writeln!(
                f,
                "{}: not holdable ({}; the simulator cannot script it, so it follows the prompt schedule in every run)",
                n.location, n.reason
            )?;
        }
        writeln!(f)?;
        if let Verdict::NotExplored { reason } = &self.verdict {
            writeln!(f, "verdict: not explored")?;
            writeln!(f, "  {reason}")?;
            writeln!(
                f,
                "  no schedule was tried, so this says nothing about the program; check that the workload reaches a use::batch or use::snapshot the simulator can script"
            )?;
            return Ok(());
        }
        if let Some(l) = &self.location {
            writeln!(f, "verdict: hazardous")?;
            writeln!(
                f,
                "  decision point: {}",
                l.readable_name(show_index(&l.hook))
            )?;
            writeln!(
                f,
                "  first reacted at a hold of {} rounds, with {} extra {} there",
                l.first_reaction_at,
                l.extra_work_at_first_reaction,
                if l.rose == "admitted records" {
                    "admitted records".to_owned()
                } else {
                    format!("messages ({})", l.rose)
                }
            )?;
            writeln!(
                f,
                "  largest extra work over the curve: {} in {}",
                l.extra_work, l.rose
            )?;
            if !l.hooks_that_rose.is_empty() {
                writeln!(
                    f,
                    "  decision points that admitted more records under that hold, at its peak:"
                )?;
                for (h, d) in &l.hooks_that_rose {
                    writeln!(f, "    {} +{d}", readable_hook_name(h, show_index(h)))?;
                }
            }
            if let Some(g) = &self.growth {
                writeln!(f, "growth: {}", g.headline())?;
                for line in g.lines(&show_index) {
                    writeln!(f, "  {line}")?;
                }
            }
        } else {
            writeln!(f, "verdict: benign")?;
            writeln!(
                f,
                "  no reaction to a delay of up to {} rounds was found at any decision point",
                self.horizon
            )?;
        }
        Ok(())
    }
}

impl Report {
    /// The finite hold lengths that outlast the input (see
    /// [`CheckConfig::holds_beyond_input`]).
    pub fn growth_holds(&self) -> Vec<usize> {
        let horizon = self.horizon;
        self.hold_lengths
            .iter()
            .copied()
            .filter(|&k| k < horizon && self.hold_start + k >= self.workload_rounds)
            .collect()
    }
}

impl GrowthLabel {
    /// The one-line form for the table: `stops`, `grows`, `grows, slowing`, `grows, degree d`,
    /// `shedding`, or `not read` when the input ran to the end of the run.
    pub fn word(&self) -> String {
        match &self.shape {
            Some((_, s)) => s.beyond.as_ref().unwrap().shape.word(),
            None => "not read".to_owned(),
        }
    }

    /// The largest multiplier, as a number, when the reference count was nonzero.
    pub fn multiplier_value(&self) -> Option<f64> {
        self.multiplier.1.multiplier()
    }

    fn headline(&self) -> String {
        match &self.shape {
            Some((_, s)) => match &s.beyond.as_ref().unwrap().shape {
                Shape::Shed { .. } => {
                    "shedding. Once the hold outlasts the input, the program does less work than the unheld run, not more.".to_owned()
                }
                Shape::Flat { .. } => {
                    "stops. Once the hold outlasts the input, the extra work reaches a ceiling and a longer delay adds nothing.".to_owned()
                }
                Shape::Linear { .. } => {
                    "keeps growing. Once the hold outlasts the input, every further round of delay costs the same amount of extra work.".to_owned()
                }
                Shape::Polynomial { degree, .. } => format!(
                    "keeps growing, faster and faster. Once the hold outlasts the input, each further round of delay costs more than the last (degree {degree})."
                ),
                Shape::Sublinear { .. } => {
                    "keeps growing, more slowly. Once the hold outlasts the input, the extra work still rises but each further round costs less than the last.".to_owned()
                }
            },
            None => "not read. Data arrived until fewer than two finite holds remained, so whether the extra work stops once the input is over could not be told; only the multiplier and the stall reaction are given.".to_owned(),
        }
    }

    fn lines(&self, show_index: &dyn Fn(&str) -> bool) -> Vec<String> {
        let mut out = Vec::new();
        if let Some((hook, s)) = &self.shape {
            let g = s.beyond.as_ref().unwrap();
            out.push(format!(
                "worst shape, at {} over holds {}: {}",
                readable_hook_name(hook, show_index(hook)),
                list(&g.holds),
                g.describe(&s.count)
            ));
        }
        let (hook, m) = &self.multiplier;
        out.push(format!(
            "largest multiplier, at {}: {}",
            readable_hook_name(hook, show_index(hook)),
            m.multiplier_text()
        ));
        let (hook, s) = &self.stall;
        let rising = self
            .shape
            .as_ref()
            .is_some_and(|(_, w)| w.beyond.as_ref().unwrap().shape.rank().0 > 0);
        out.push(format!(
            "largest stall reaction, at {}: {} under a hold that never ends",
            readable_hook_name(hook, show_index(hook)),
            stall_text(s, self.horizon, rising)
        ));
        out
    }
}

/// `+N count`, and, when the shape says the extra work keeps rising with the delay, its
/// per-round figure over the stall, which for a mechanism that keeps working at a steady rate
/// while its records never arrive is an estimate of that rate independent of the fitted shape.
fn stall_text(s: &SeriesGrowth, horizon: usize, rising: bool) -> String {
    if rising && s.stall > 0 && horizon > 0 {
        format!(
            "+{} {} ({:.1} per round of the run)",
            s.stall,
            s.count,
            s.stall as f64 / horizon as f64
        )
    } else {
        format!("{}{} {}", if s.stall >= 0 { "+" } else { "" }, s.stall, s.count)
    }
}

/// Renders a hook identity `file:line#index [item]` or `file:line#index [snapshot item]` as
/// `file:line, batch of item` or `file:line, snapshot of item`. When `show_index` is set, the
/// index follows the position as ` #index`, which a report does only for decision points that
/// share a line with another.
pub fn readable_hook_name(hook: &str, show_index: bool) -> String {
    let Some((loc, rest)) = hook.split_once('#') else {
        return hook.to_owned();
    };
    let (index, item) = match rest.split_once(" [") {
        Some((i, item)) => (i, item.trim_end_matches(']')),
        None => (rest, ""),
    };
    let position = if show_index {
        format!("{loc} #{index}")
    } else {
        loc.to_owned()
    };
    if let Some(t) = item.strip_prefix("snapshot ") {
        format!("{position}, snapshot of {t}")
    } else if item.is_empty() {
        position
    } else {
        format!("{position}, batch of {item}")
    }
}

/// The `file:line` part of a hook identity.
fn hook_line(hook: &str) -> &str {
    hook.split('#').next().unwrap_or(hook)
}

/// The lines shared by more than one of the given hook identities. A decision point on such a
/// line is shown with its index so the reader can tell the points apart.
fn shared_lines<'a>(hooks: impl IntoIterator<Item = &'a str>) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut shared = BTreeSet::new();
    for h in hooks {
        let line = hook_line(h).to_owned();
        if !seen.insert(line.clone()) {
            shared.insert(line);
        }
    }
    shared
}

fn row(values: &[i64]) -> String {
    use std::fmt::Write;
    values.iter().fold(String::new(), |mut s, v| {
        write!(s, "{v:>7}").unwrap();
        s
    })
}

fn row_usize(values: &[usize]) -> String {
    use std::fmt::Write;
    values.iter().fold(String::new(), |mut s, v| {
        write!(s, "{v:>7}").unwrap();
        s
    })
}

fn list(values: &[usize]) -> String {
    values
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Runs the checker. `round(i)` sends round `i` of the fixed workload; the checker awaits
/// `quiesce()` after it. See the [crate docs](crate).
pub fn check(
    flow: SimFlow<'_>,
    config: &CheckConfig,
    mut round: impl AsyncFnMut(usize) + RefUnwindSafe,
) -> Report {
    let started = Instant::now();
    let mut flow = flow;
    let instrumented = instrument(&mut flow);
    let compiled = flow.compiled();
    let hold_lengths = config.hold_lengths();
    let (baseline, seen, _) = run_once(&compiled, &instrumented, config, &mut round, None, 0);
    // Hooks that had something waiting in the unheld run, in identity order.
    let hooks: Vec<(usize, String)> = {
        let mut v: Vec<(usize, String)> = seen.into_iter().collect();
        v.sort_by(|a, b| a.1.cmp(&b.1));
        v
    };
    let base_sends = baseline.sends_by_member();
    let base_admitted = baseline.admitted() as i64;

    let shared = shared_lines(hooks.iter().map(|(_, h)| h.as_str()));
    let mut curves = Vec::with_capacity(hooks.len());
    for (hook_id, hook) in &hooks {
        let mut extra_admitted = vec![(0usize, 0i64)];
        let mut extra_sends = vec![(0usize, BTreeMap::new())];
        let mut counts = vec![(0usize, baseline.clone())];
        let mut holds_applied = Vec::new();
        let mut extra_work = 0i64;
        let mut rose = None;
        let mut first_reaction_at = None;
        let mut extra_work_at_first_reaction = 0i64;
        for &k in &hold_lengths {
            let (c, _, applied) =
                run_once(&compiled, &instrumented, config, &mut round, Some(*hook_id), k);
            holds_applied.push((k, applied));
            let d_admitted = c.admitted() as i64 - base_admitted;
            let mut d_sends = BTreeMap::new();
            let mut rise_here = d_admitted.max(0);
            if d_admitted > extra_work {
                extra_work = d_admitted;
                rose = Some("admitted records".to_owned());
            }
            for (place, n) in c.sends_by_member() {
                let d = n as i64 - base_sends.get(&place).copied().unwrap_or(0) as i64;
                rise_here = rise_here.max(d);
                if d > extra_work {
                    extra_work = d;
                    rose = Some(place.clone());
                }
                d_sends.insert(place, d);
            }
            if rise_here > 0 && first_reaction_at.is_none() {
                first_reaction_at = Some(k);
                extra_work_at_first_reaction = rise_here;
            }
            extra_admitted.push((k, d_admitted));
            extra_sends.push((k, d_sends));
            counts.push((k, c));
        }
        let series = series_growth(
            &baseline,
            &counts,
            &hold_lengths,
            &config.holds_beyond_input(),
            config.separates_input(),
        );
        let curve = HookCurve {
            hook: hook.clone(),
            extra_admitted,
            extra_sends_by_member: extra_sends,
            counts,
            holds_applied,
            extra_work,
            rose,
            first_reaction_at,
            extra_work_at_first_reaction,
            series,
        };
        if config.progress {
            eprintln!(
                "[amplification {}/{} at {:.0} s] hold lengths {}",
                curves.len() + 1,
                hooks.len(),
                started.elapsed().as_secs_f64(),
                row_usize(&hold_lengths).trim_start()
            );
            for line in curve.lines(LABEL_WIDTH, shared.contains(hook_line(hook))) {
                eprintln!("  {line}");
            }
        }
        curves.push(curve);
    }

    // Rank: shortest first-reaction length, then more extra work at that length, then more
    // extra work over the curve. A remaining tie goes to the first hook in identity order, and
    // the report's curves show it.
    let mut best: Option<&HookCurve> = None;
    for c in curves.iter().filter(|c| c.extra_work > 0) {
        let key = |c: &HookCurve| {
            (
                std::cmp::Reverse(c.first_reaction_at.unwrap_or(usize::MAX)),
                c.extra_work_at_first_reaction,
                c.extra_work,
            )
        };
        if best.is_none_or(|b| key(c) > key(b)) {
            best = Some(c);
        }
    }
    let location = best.map(|c| Location {
        hook: c.hook.clone(),
        source_location: c.source_location().to_owned(),
        rose: c.rose.clone().unwrap_or_default(),
        extra_work: c.extra_work,
        first_reaction_at: c.first_reaction_at.unwrap_or(0),
        extra_work_at_first_reaction: c.extra_work_at_first_reaction,
        hooks_that_rose: hooks_that_rose(&baseline, c),
    });
    let growth = location
        .as_ref()
        .and_then(|_| growth_label(&curves, &config.holds_beyond_input(), config.horizon()));
    Report {
        verdict: verdict_of(&baseline, &curves, location.is_some()),
        rounds: config.rounds,
        hold_start: config.hold_start,
        horizon: config.horizon(),
        hold_lengths,
        workload_rounds: config.workload_rounds,
        baseline,
        curves,
        not_holdable: instrumented.not_scriptable.clone(),
        location,
        growth,
        seconds: started.elapsed().as_secs_f64(),
    }
}

/// The verdict: hazardous when some hold added work; benign when holds were tried and none did;
/// not explored when there was nothing to hold or the unheld run moved nothing, which is the one
/// case in which a run that found no extra work must not be read as benign.
fn verdict_of(baseline: &WorkCounts, curves: &[HookCurve], hazardous: bool) -> Verdict {
    if hazardous {
        return Verdict::Hazardous;
    }
    if curves.is_empty() {
        return Verdict::NotExplored {
            reason: format!(
                "no decision point had buffered input in the unheld run ({} records admitted, {} messages sent), so there was nothing to hold",
                baseline.admitted(),
                baseline.network()
            ),
        };
    }
    if baseline.admitted() == 0 && baseline.network() == 0 {
        return Verdict::NotExplored {
            reason: "the unheld run admitted no records and sent no messages, so the counts the verdict reads never moved".to_owned(),
        };
    }
    Verdict::Benign
}

/// For every count that rose under one hook at some hold length: its worst rise, its stall
/// reaction, and, when `separated`, its growth over the holds that outlast the input.
fn series_growth(
    baseline: &WorkCounts,
    counts: &[(usize, WorkCounts)],
    hold_lengths: &[usize],
    beyond: &[usize],
    separated: bool,
) -> Vec<SeriesGrowth> {
    let end = *hold_lengths.last().unwrap_or(&0);
    let base_sends = baseline.sends_by_member();
    // Each count's reference and its extra at every hold length k > 0.
    let mut names: Vec<(String, u64)> = vec![("admitted records".to_owned(), baseline.admitted())];
    let mut places: BTreeSet<String> = base_sends.keys().cloned().collect();
    for (_, c) in counts {
        places.extend(c.sends_by_member().into_keys());
    }
    for p in places {
        let r = base_sends.get(&p).copied().unwrap_or(0);
        names.push((p, r));
    }
    let extra_of = |name: &str, c: &WorkCounts| -> i64 {
        if name == "admitted records" {
            c.admitted() as i64 - baseline.admitted() as i64
        } else {
            c.sends_by_member().get(name).copied().unwrap_or(0) as i64
                - base_sends.get(name).copied().unwrap_or(0) as i64
        }
    };
    let mut out = Vec::new();
    for (name, reference) in names {
        let extras: Vec<(usize, i64)> = counts
            .iter()
            .filter(|(k, _)| *k > 0)
            .map(|(k, c)| (*k, extra_of(&name, c)))
            .collect();
        let Some(&(worst_at, worst)) = extras.iter().max_by_key(|(_, e)| *e) else {
            continue;
        };
        if worst <= 0 {
            continue;
        }
        let stall = extras
            .iter()
            .find(|(k, _)| *k == end)
            .map(|(_, e)| *e)
            .unwrap_or(0);
        let beyond_growth = separated.then(|| {
            let e: Vec<i64> = beyond
                .iter()
                .map(|k| {
                    extras
                        .iter()
                        .find(|(kk, _)| kk == k)
                        .map(|(_, e)| *e)
                        .unwrap_or(0)
                })
                .collect();
            Growth::fit(beyond.to_vec(), e)
        });
        out.push(SeriesGrowth {
            count: name,
            reference,
            worst,
            worst_at,
            stall,
            beyond: beyond_growth,
        });
    }
    out
}

/// Orders two series with growth readings: by shape class, then a `sends from` series above the
/// admitted total, then the larger last rate, then the larger worst rise.
fn shape_order(x: &SeriesGrowth, y: &SeriesGrowth) -> std::cmp::Ordering {
    let (cx, rx) = x.beyond.as_ref().unwrap().shape.rank();
    let (cy, ry) = y.beyond.as_ref().unwrap().shape.rank();
    cx.cmp(&cy)
        .then((x.count != "admitted records").cmp(&(y.count != "admitted records")))
        .then(rx.partial_cmp(&ry).unwrap_or(std::cmp::Ordering::Equal))
        .then(x.worst.cmp(&y.worst))
}

/// The growth label over every hook that reacted: the worst shape (when the input is
/// separated), the largest multiplier, and the largest stall reaction, each pinned to its hook.
fn growth_label(curves: &[HookCurve], beyond: &[usize], horizon: usize) -> Option<GrowthLabel> {
    let reacting: Vec<&HookCurve> = curves
        .iter()
        .filter(|c| c.extra_work > 0 && !c.series.is_empty())
        .collect();
    let pin = |s: Option<(&HookCurve, &SeriesGrowth)>| s.map(|(c, s)| (c.hook.clone(), s.clone()));
    let shape = pin(reacting
        .iter()
        .filter_map(|c| c.worst_shape().map(|s| (*c, s)))
        .max_by(|(_, x), (_, y)| shape_order(x, y)));
    let multiplier = pin(reacting
        .iter()
        .filter_map(|c| c.worst_multiplier().map(|s| (*c, s)))
        .max_by(|(_, x), (_, y)| {
            let kx = x.multiplier().unwrap_or(f64::INFINITY);
            let ky = y.multiplier().unwrap_or(f64::INFINITY);
            kx.partial_cmp(&ky)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(x.worst.cmp(&y.worst))
        }))?;
    let stall = pin(reacting
        .iter()
        .filter_map(|c| c.worst_stall().map(|s| (*c, s)))
        .max_by_key(|(_, s)| s.stall))?;
    Some(GrowthLabel {
        holds_beyond_input: beyond.to_vec(),
        shape,
        multiplier,
        stall,
        horizon,
    })
}

/// One execution: `config.rounds` rounds of workload, holding hook `target` (every member's
/// instance of it) from `config.hold_start` for `k` rounds (a `k` reaching the end of the run is
/// a hold that never ends). Returns the counts, the hooks that had something waiting at some
/// point (by id and identity), and how many times a tick containing the held hook ran for
/// another hook's sake while the held hook had something waiting.
fn run_once(
    compiled: &CompiledSim,
    instrumented: &Instrumented,
    config: &CheckConfig,
    round: &mut (impl AsyncFnMut(usize) + RefUnwindSafe),
    target: Option<usize>,
    k: usize,
) -> (WorkCounts, BTreeMap<usize, String>, u64) {
    let result = std::sync::Mutex::new(None);
    let result_ref = &result;
    compiled.run_with_driver(PromptScheduleDriver::default(), async |instance| {
        instance
            .run_with_scheduler(async {
                let mut schedule = Schedule::new(instrumented);
                for r in 0..config.rounds {
                    if let Some(id) = target {
                        if r == config.hold_start && k > 0 {
                            schedule.hold(id);
                        }
                        if r == config.hold_start + k {
                            schedule.release();
                        }
                    }
                    round(r).await;
                    schedule.settle().await;
                }
                let counts = WorkCounts::from_schedule(&schedule.counts, &schedule);
                let seen = schedule
                    .seen
                    .iter()
                    .map(|id| (*id, schedule.identity(*id)))
                    .collect();
                *result_ref.lock().unwrap() = Some((counts, seen, schedule.holds_applied));
            })
            .await
    });
    result.into_inner().unwrap().unwrap()
}

/// Hooks that admitted more records than in the unheld run, at the hold length where the
/// curve's admitted total peaked.
fn hooks_that_rose(baseline: &WorkCounts, curve: &HookCurve) -> BTreeMap<String, i64> {
    let Some((_, peak)) = curve.counts.iter().max_by_key(|(_, c)| c.admitted()) else {
        return BTreeMap::new();
    };
    let mut rises = BTreeMap::new();
    for (hook, n) in &peak.hook_releases {
        let d = *n as i64 - baseline.hook_releases.get(hook).copied().unwrap_or(0) as i64;
        if d > 0 {
            rises.insert(hook.clone(), d);
        }
    }
    rises
}
