use std::{str::FromStr, time::Instant};

use argh::FromArgs;
use glam::*;

use obvhs::ray::Ray;

pub mod bvh;
pub mod par;
pub mod ploc;

// Used for now instead of features just for rust-analyzer
#[derive(PartialEq, Eq, Default)]
pub enum Scheduler {
    SequentialOptimized,
    Sequential,
    #[default]
    Forte,
    Chili,
    Rayon,
}

impl FromStr for Scheduler {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "seq_opt" => Ok(Self::SequentialOptimized),
            "seq" => Ok(Self::Sequential),
            "forte" => Ok(Self::Forte),
            "chili" => Ok(Self::Chili),
            "rayon" => Ok(Self::Rayon),
            _ => Err(format!(
                "Unknown mode: '{s}', valid modes: 'seq_opt', 'seq', 'forte', 'chili', 'rayon'"
            )),
        }
    }
}

#[derive(FromArgs)]
/// `demoscene` example
pub struct Args {
    /// threading scheduler backend. Modes: 'seq_opt', 'seq', 'forte', 'chili', 'rayon'
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
            format!("{}", obvhs::PrettyDuration(elapsed)),
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
