// https://github.com/nessex/rdst/

use std::sync::atomic::{AtomicU32, Ordering};

use crate::{par::Scheduler, Args};

pub mod comparative_sort;
pub mod radix_key;
pub mod regions_sort;
pub mod ska_sort;
pub mod sort_utils;
pub mod sorter;

static RADIX_SCHEDULER: AtomicU32 = AtomicU32::new(0);

pub fn radix_scheduler() -> Scheduler {
    Scheduler::from(RADIX_SCHEDULER.load(Ordering::Relaxed))
}

pub fn init_radix_scheduler() {
    let config: Args = argh::from_env();
    config.radix_sch.init();
    RADIX_SCHEDULER.store(config.radix_sch as u32, Ordering::Relaxed);
}
