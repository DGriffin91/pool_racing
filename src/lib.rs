use std::{
    str::FromStr,
    time::{Duration, Instant},
};

use argh::FromArgs;
use glam::*;

use crate::{bvh::Bvh2Node, ray::Ray};
pub mod aabb;
pub mod bvh;
pub mod morton;
pub mod par_forte;
pub mod par_rayon;
pub mod par_sequential;
pub mod ploc;
pub mod ray;
pub mod test_util;
pub mod triangle;

// Used for now instead of features just for rust-analyzer
#[derive(PartialEq, Eq, Default)]
pub enum Scheduler {
    SequentialOptimized,
    Sequential,
    #[default]
    Forte,
    Rayon,
}

impl FromStr for Scheduler {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "seq_opt" => Ok(Self::SequentialOptimized),
            "seq" => Ok(Self::Sequential),
            "forte" => Ok(Self::Forte),
            "rayon" => Ok(Self::Rayon),
            _ => Err(format!(
                "Unknown mode: '{s}', valid modes: 'seq_opt', 'seq', 'forte', 'rayon'"
            )),
        }
    }
}

#[derive(FromArgs)]
/// `demoscene` example
pub struct Args {
    /// threading scheduler backend. Modes: 'seq_opt', 'seq', 'forte', 'rayon'
    #[argh(option, default = "Scheduler::Forte")]
    pub backend: Scheduler,
}

pub struct Traversal {
    pub stack: Vec<u32>,
    pub ray: Ray,
}

impl Traversal {
    #[inline(always)]
    /// Reinitialize traversal state with new ray.
    pub fn reinit(&mut self, ray: Ray) {
        self.stack.clear();
        self.stack.push(0);
        self.ray = ray;
    }
}

pub struct Timer {
    start: Instant,
    label: String,
}

impl Timer {
    pub fn new(label: &str) -> Self {
        Self {
            start: Instant::now(),
            label: label.to_string(),
        }
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        println!(
            "{:>8} {}",
            format!("{}", PrettyDuration(elapsed)),
            self.label
        )
    }
}

/// Add profile scope. Nesting the macro allows us to make the profiling crate optional.
/// Use profile feature to enable profiling.
#[doc(hidden)]
#[macro_export]
macro_rules! scope {
    [$label:expr] => {
        #[cfg(feature = "profile")]
        profiling::scope!($label);
    };
}

/// Add profile scope and timer.
/// Use scope_print feature to print times to console.
/// Use profile feature to enable profiling.
#[doc(hidden)]
#[macro_export]
macro_rules! scope_print {
    [$label:expr] => {
        #[cfg(feature = "profile")]
        profiling::scope!($label);
        #[cfg(feature = "scope_print")]
        let _t = $crate::Timer::new($label);
    };
}

/// Add profile scope and timer.
/// Use scope_print_major feature to print times to console.
/// Use profile feature to enable profiling.
#[doc(hidden)]
#[macro_export]
macro_rules! scope_print_major {
    [$label:expr] => {
        #[cfg(feature = "profile")]
        profiling::scope!($label);
        #[cfg(feature = "scope_print_major")]
        let _t = $crate::Timer::new($label);
    };
}

/// A wrapper struct for `std::time::Duration` to provide pretty-printing of durations.
#[doc(hidden)]
pub struct PrettyDuration(pub Duration);

impl std::fmt::Display for PrettyDuration {
    /// Durations are formatted as follows:
    /// - If the duration is greater than or equal to 1 second, it is formatted in seconds (s).
    /// - If the duration is greater than or equal to 1 millisecond but less than 1 second, it is formatted in milliseconds (ms).
    /// - If the duration is less than 1 millisecond, it is formatted in microseconds (µs).
    ///   In the case of seconds & milliseconds, the duration is always printed with a precision of two decimal places.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let duration = self.0;
        if duration.as_secs() > 0 {
            let seconds =
                duration.as_secs() as f64 + f64::from(duration.subsec_nanos()) / 1_000_000_000.0;
            write!(f, "{seconds:.2}s ")
        } else if duration.subsec_millis() > 0 {
            let milliseconds =
                duration.as_millis() as f64 + f64::from(duration.subsec_micros() % 1_000) / 1_000.0;
            write!(f, "{milliseconds:.2}ms")
        } else {
            let microseconds = duration.as_micros();
            write!(f, "{microseconds}µs")
        }
    }
}
