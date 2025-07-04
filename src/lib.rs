use std::time::Duration;

use glam::*;

use crate::{bvh::Bvh2Node, ray::Ray};
pub mod aabb;
pub mod bvh;
pub mod morton;
pub mod par_forte;
pub mod par_rayon;
pub mod ploc;
pub mod ray;
pub mod test_util;
pub mod triangle;

// Used for now instead of features just for rust-analyzer
#[allow(dead_code)]
pub enum Scheduler {
    Sequential,
    Forte,
    Rayon,
}

pub const SCHEDULER: Scheduler = Scheduler::Forte;

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
