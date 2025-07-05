// https://github.com/nessex/rdst/

use crate::par::Scheduler;

pub mod comparative_sort;
pub mod radix_key;
pub mod regions_sort;
pub mod ska_sort;
pub mod sort_utils;
pub mod sorter;

pub const DEFAULT_SCHEDULER: Scheduler = Scheduler::Rayon;
