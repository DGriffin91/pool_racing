use std::time::Instant;

use argh::FromArgs;

use crate::par::Scheduler;

pub mod bvh;
pub mod par;
pub mod ploc;
pub mod radix;

#[derive(FromArgs)]
/// `demoscene` example
pub struct Args {
    /// threading scheduler backend for ploc. Modes: 'seq_opt', 'seq', 'forte', 'chili', 'rayon'
    #[argh(option, default = "Scheduler::Forte")]
    pub ploc_sch: Scheduler,

    /// threading scheduler backend for radix. Modes: 'seq_opt', 'seq', 'forte', 'chili', 'rayon'
    #[argh(option, default = "Scheduler::Forte")]
    pub radix_sch: Scheduler,
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
