//! Cress, the schedule-amplification checker: does the schedule alone make a Hydro program do
//! work its input did not require, and if so, where?
//!
//! [`check`] takes a simulation and a fixed workload, compiles the simulation once, and runs
//! it many times. The first run follows the deterministic prompt schedule and discovers every
//! `use::batch` and `use::snapshot` hook that ever has something buffered. Each later run holds
//! one of those hooks (see the `schedule` module) for a chosen number of rounds, with every
//! other decision still following the prompt schedule, so a delay of any length on one edge is
//! a single decision. A held batch delays the records arriving at it; a held snapshot keeps its
//! tick observing a stale version of a singleton, and on release jumps to the newest version. Every run is measured with the counts the
//! simulator's own hooks report (see [`WorkCounts`]); nothing is read from the program's
//! outputs.
//!
//! # How long a hold lasts
//!
//! The checker does not ask the caller which delays to try, because a caller who knew the
//! program's timescales would not need the checker, and a list of delays that stops short of the
//! program's timeout would read benign with nothing to say so. Instead, for each hook, it holds
//! for 1 round, then 2, then 4, doubling, up to and including a hold that lasts to the end of the
//! run, which is the schedule in which that hook's records are never delivered. Every length in
//! the sequence is run for every hook, so each report carries the complete curve and a reader
//! can see whether the extra work grows with the length of the delay or steps once and stays
//! flat. The only quantity the caller sets is the run length in rounds, the *horizon*, and a
//! benign verdict is a statement about that horizon: no reaction to a delay of up to the horizon
//! was found at any decision point. The horizon is printed with every report.
//!
//! # The verdict rule
//!
//! A hold delays records and cannot create them, so any count that rises under a hold is the
//! program's reaction to delay. The rule reads two counts against the unheld run on the same
//! input: each cluster member's (or process's) outgoing network messages, and the program's
//! total records admitted into ticks by batch hooks. Messages are read per sender because a
//! cluster total can fall while one member's traffic rises (a leader that stops sending
//! heartbeats while followers start sending vote requests). Admitted records are read in total
//! because within one location a reaction can replace one record with another (a completion
//! becomes an abandonment) without adding work. The verdict is [`Verdict::Hazardous`] if some
//! hold raises either count above the unheld run at any hold length tried, [`Verdict::Benign`]
//! if holds were tried and none did, and [`Verdict::NotExplored`] if the unheld run found no
//! decision point to hold or moved nothing, so that a run that explored no schedule is never
//! read as benign. The rule uses only the existence of extra work. The
//! shape of the curve, whether the extra work grows with the hold length or not, is reported for
//! the reader and is not a criterion the checker applies.
//!
//! # The growth label
//!
//! A hazardous report also carries a [`GrowthLabel`], which is a guess and not a verdict: it
//! says how much extra work was found and how the extra work behaves as the delay grows, in a
//! form a reader can hold against a program's own assurance argument. It has three parts, and
//! each is pinned to the decision point whose hold produced it.
//!
//! The *multiplier* is the count under the worst hold found as a multiple of the unheld run's
//! count, for the count and sender where that ratio is largest: "2.98× the unheld run's 250
//! sends from the client" says that under some single delay the client sent almost three times
//! the messages the input required. The *stall reaction* is the extra work under the hold that
//! lasts to the end of the run, the schedule in which the held records never arrive; a retry
//! policy that stops when nothing succeeds shows a small constant here, and one that does not
//! shows work proportional to the run.
//!
//! The *shape* is read only when the run separates the input from the delay. Every hold in the
//! sequence delivers the same input, but when data arrives in every round the longest holds
//! are also the ones under which the most data is waiting, so a rise across them cannot be told
//! from a rise with the input, and a cap proportional to the input is never reached inside the
//! run. [`CheckConfig::with_workload_rounds`] confines the data to the first rounds; the
//! workload closure keeps sending timer elements afterwards, so holds longer than the data
//! outlast the whole input with the input fixed. Over those holds the checker fits each count's
//! extra work as a formula in the hold length `k`, with constants it names and pins: `a` for a
//! plateau (the extra work stops once the input is over, which is what a bounded number of
//! re-sends per request or a budget refilled only by successes produces), or `a + b·(k − k₀)`
//! for a steady rate `b` per round of delay (which is what a budget refilled by a clock
//! produces, with `b` the refill rate), with a degree above one when the marginal rate itself
//! grows across doublings of the hold, and a "shedding" reading when every count sits below the
//! unheld run's. The constants' meanings in the program (an attempt cap, a bucket size, a refill
//! rate) are not the checker's to know; the fit gives their values and the line each was read
//! at, and the reader matches them to the code. A plateau is read when the last interval rises
//! by at most one record, which is the resolution of an integer count against the phase of the
//! program's timers at the release round, and is stated as such. See [`Shape`].
//!
//! When data arrives in every round the label says the input was not separated and gives only
//! the multiplier and the stall reaction. The report of the shape names the holds it read and
//! the marginal rates between them, so a reader can judge the guess.
//!
//! # The location
//!
//! Among the hooks whose hold added work, the one that reacted to the shortest delay is
//! reported as the location: hooks are ranked by the smallest hold length at which extra work
//! first appeared, then, among hooks tied on that length, by the larger extra work at that
//! length, and only then by the largest extra work anywhere on the curve. The point that reacts
//! to the least delay is where the program's response to lateness begins, which is the fact a
//! reader wants; the largest extra work over the whole curve is not a good primary criterion
//! because the last hold in the sequence lasts to the end of the run, and under a permanent hold
//! on any edge of a resend loop the sender exhausts its resends on every request, so several
//! edges of the same loop reach one ceiling and the largest extra work no longer distinguishes
//! them. The report carries the winning hook's first-reaction length, its extra work at that
//! length, its largest extra work over the curve, and the hooks that admitted more records under
//! it.
//!
//! # What the caller supplies
//!
//! The program, wired with `sim_input` streams for its timers and inputs, a closure that sends
//! one round of the fixed workload, and the horizon. The checker calls the closure once per
//! round and awaits [`hydro_lang::sim::quiesce`] after each call, so the closure need not; if it wants to
//! drain outputs each round it may await `quiesce()` itself first. The workload should be the
//! program's ordinary steady state, with no burst: the checker's job is to find schedules under
//! which that same input costs more. When [`CheckConfig::workload_rounds`] is below the run
//! length, the closure is still called for every round and should send only timer elements
//! from that round on. As each hook's escalation completes the checker prints one line to
//! standard error with that hook's curve, so a long run shows its findings as it goes.
//!
//! Most callers need not write the wiring by hand. The attribute [`amplification_check`] on a
//! Hydro function generates it from the function's signature, and `./scripts/cress <crate>` at
//! the repository root runs every annotated function as a normal command and prints one concise
//! verdict per configuration. Full reports are saved under `target/cress/`. The crate under
//! check lists `cress` as an optional dependency and gates the attribute on
//! `cfg(feature = "cress")`; the command turns that feature on.
//!
//! # What this does not see
//!
//! Records are counted where they enter a tick and where they cross the network. Work that a
//! program represents as a number inside a record, or as records flowing between operators
//! within one tick, is invisible. Snapshot hooks are held but their releases are not counted,
//! because a snapshot is one value per tick execution and not a record the program produced.
//! Hooks are held one at a time.

#[cfg(stageleft_runtime)]
hydro_lang::setup!();

#[cfg(stageleft_runtime)]
mod check;
#[cfg(stageleft_runtime)]
pub use check::*;

#[cfg(stageleft_runtime)]
mod schedule;

#[cfg(stageleft_runtime)]
#[doc(hidden)]
pub mod harness;
#[cfg(stageleft_runtime)]
pub use harness::{CheckResult, InputCounter, InputValue, SimOutputs};

/// Paths the generated harness code reaches through this crate, so that a crate under check
/// need not depend on `hydro_lang` itself.
#[cfg(stageleft_runtime)]
#[doc(hidden)]
pub mod __private {
    pub use hydro_lang;
}

/// Registers a Hydro function with the `cress` command. See the
/// [macro's documentation](cress_config_macro::amplification_check).
pub use cress_config_macro::amplification_check;
/// Implements [`SimOutputs`] for a struct whose fields are streams.
pub use cress_config_macro::SimOutputs;

#[cfg(test)]
mod tests;
