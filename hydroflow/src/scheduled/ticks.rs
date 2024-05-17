//! This module contains types to work with ticks.
//!
//! Each iteration of a Hydroflow transducer loop is called a tick. Associated with the transducer
//! is a clock value, which tells you how many ticks were executed by this transducer prior to the
//! current tick. Each transducer produces totally ordered, sequentially increasing clock values,
//! which you can think of as the "local logical time" at the transducer.

use std::fmt::{Display, Formatter};
use std::ops::{Add, AddAssign, Neg, Sub, SubAssign};

use serde::{Deserialize, Serialize};

/// A point in time during execution on transducer.
///
/// `TickInstant` instances can be subtracted to calculate the `TickDuration` between them.
///
/// ```
/// # use hydroflow::scheduled::ticks::{TickDuration, TickInstant};
///
/// assert_eq!(TickInstant(1) - TickInstant(0), TickDuration::SINGLE_TICK);
/// assert_eq!(TickInstant(0) - TickInstant(1), -TickDuration::SINGLE_TICK);
/// ```
#[derive(
    Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Hash, Default, Debug, Serialize, Deserialize,
)]
pub struct TickInstant(pub u64);

/// The duration between two ticks.
///
/// `TickDuration` instances can be negative to allow for calculation of `TickInstant` instances in the past.
///
/// ```
/// # use hydroflow::scheduled::ticks::{TickDuration, TickInstant};
/// assert_eq!(TickInstant(1) + TickDuration::new(-1), TickInstant(0))
/// ```
/// `TickDuration` instances can be added/subtracted to/from other `TickDuration` instances
///
/// ```
/// # use hydroflow::scheduled::ticks::TickDuration;
/// assert_eq!(TickDuration::ZERO + TickDuration::ZERO, TickDuration::ZERO);
/// assert_eq!(
///     TickDuration::ZERO + TickDuration::SINGLE_TICK,
///     TickDuration::SINGLE_TICK
/// );
/// assert_eq!(
///     TickDuration::SINGLE_TICK - TickDuration::ZERO,
///     TickDuration::SINGLE_TICK
/// );
/// assert_eq!(
///     TickDuration::SINGLE_TICK - TickDuration::SINGLE_TICK,
///     TickDuration::ZERO
/// );
/// assert_eq!(
///     TickDuration::ZERO - TickDuration::SINGLE_TICK,
///     -TickDuration::SINGLE_TICK
/// );
/// ```
#[derive(
    Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Hash, Default, Debug, Serialize, Deserialize,
)]
pub struct TickDuration {
    /// The length of the duration, measured in ticks.
    pub ticks: i64,
}

impl TickInstant {
    /// Create a new TickInstant
    ///
    /// The specified parameter indicates the number of ticks that have elapsed on the transducer,
    /// prior to this one.
    pub fn new(ticks: u64) -> Self {
        TickInstant(ticks)
    }
}

impl TickDuration {
    /// A zero duration
    ///
    /// It is the identity element for addition for both `TickDuration` and
    /// `TickInstant` (i.e. adding zero duration to a `TickInstant` or `TickDuration` results in
    /// the same `TickInstant` or `TickDuration`.
    ///
    /// ```
    /// # use hydroflow::scheduled::ticks::{TickDuration, TickInstant};
    /// # use hydroflow_lang::graph::ops::DelayType::Tick;
    /// let ticks = TickInstant::new(100);
    /// assert_eq!(ticks + TickDuration::ZERO, ticks);
    /// assert_eq!(ticks - TickDuration::ZERO, ticks);
    ///
    /// let duration = TickDuration::new(100);
    /// assert_eq!(duration + TickDuration::ZERO, duration);
    /// assert_eq!(duration - TickDuration::ZERO, duration);
    /// ```
    pub const ZERO: Self = TickDuration { ticks: 0 };

    /// A single tick duration.
    ///
    /// It is the duration between two consecutive `TickInstant` instances.
    ///
    /// ```
    /// # use hydroflow::scheduled::ticks::{TickDuration, TickInstant};
    /// assert_eq!(TickInstant(0) + TickDuration::SINGLE_TICK, TickInstant(1))
    /// ```
    pub const SINGLE_TICK: Self = TickDuration { ticks: 1 };

    /// Create a new `TickDuration` for the specified tick interval.
    ///
    /// A negative duration allows for calculating `TickInstants` in the past and represents a
    /// backward movement in time.
    pub fn new(ticks: i64) -> TickDuration {
        TickDuration { ticks }
    }
}

impl Add<TickDuration> for TickInstant {
    type Output = TickInstant;

    fn add(self, rhs: TickDuration) -> Self::Output {
        let mut result = self;
        result += rhs;
        result
    }
}

impl AddAssign<TickDuration> for TickInstant {
    fn add_assign(&mut self, rhs: TickDuration) {
        self.0 = self
            .0
            .checked_add_signed(rhs.ticks)
            .expect("overflow while adding tick duration to tick instant.");
    }
}

impl Sub<TickDuration> for TickInstant {
    type Output = TickInstant;

    fn sub(self, rhs: TickDuration) -> Self::Output {
        let mut result = self;
        result -= rhs;
        result
    }
}

impl SubAssign<TickDuration> for TickInstant {
    fn sub_assign(&mut self, rhs: TickDuration) {
        if rhs.ticks.is_positive() {
            self.0 = self
                .0
                .checked_sub(rhs.ticks.unsigned_abs())
                .expect("overflow while subtracting duration from instant.");
        } else if rhs.ticks.is_negative() {
            self.0 = self
                .0
                .checked_add(rhs.ticks.unsigned_abs())
                .expect("overflow while subtracting duration from instant.")
        }
    }
}

impl Sub for TickInstant {
    type Output = TickDuration;

    fn sub(self, rhs: TickInstant) -> Self::Output {
        let minuend = (self.0 as i64).wrapping_add(i64::MIN);
        let subtrahend = (rhs.0 as i64).wrapping_add(i64::MIN);
        let (difference, overflowed) = minuend.overflowing_sub(subtrahend);
        if overflowed {
            panic!("overflow while subtracting two TickInstants.")
        }
        TickDuration { ticks: difference }
    }
}

impl Add for TickDuration {
    type Output = TickDuration;

    fn add(self, rhs: Self) -> Self::Output {
        let mut result = self;
        result += rhs;
        result
    }
}

impl AddAssign for TickDuration {
    fn add_assign(&mut self, rhs: Self) {
        self.ticks = self
            .ticks
            .checked_add(rhs.ticks)
            .expect("Overflow occurred while adding TickDuration instances.")
    }
}

impl Sub for TickDuration {
    type Output = TickDuration;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut result = self;
        result -= rhs;
        result
    }
}

impl SubAssign for TickDuration {
    fn sub_assign(&mut self, rhs: Self) {
        self.ticks = self
            .ticks
            .checked_sub(rhs.ticks)
            .expect("Overflow occurred while subtracting TickDuration instances.");
    }
}

impl Neg for TickDuration {
    type Output = TickDuration;

    fn neg(self) -> Self::Output {
        TickDuration {
            ticks: self
                .ticks
                .checked_neg()
                .expect("Overflow while negating duration."),
        }
    }
}

impl Display for TickInstant {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}]", self.0)
    }
}

impl Display for TickDuration {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "<{}>", self.ticks)
    }
}

impl From<TickInstant> for u64 {
    fn from(value: TickInstant) -> Self {
        value.0
    }
}

impl From<TickDuration> for i64 {
    fn from(value: TickDuration) -> Self {
        value.ticks
    }
}
