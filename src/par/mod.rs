use std::str::FromStr;

pub mod par_chili;
pub mod par_forte;
pub mod par_rayon;
pub mod par_sequential;

// Used for now instead of features just for rust-analyzer
#[derive(PartialEq, Eq, Default, Clone, Copy)]
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

    #[inline(always)]
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

impl Scheduler {
    #[inline(always)]
    pub fn par_map<T, F>(self, data: &mut [T], func: &F, workers_per_thread: u32)
    where
        T: Send + Sync,
        F: Fn(usize, &mut T) + Send + Sync,
    {
        match self {
            Scheduler::SequentialOptimized => par_sequential::par_map(data, func),
            Scheduler::Sequential => par_sequential::par_map(data, func),
            Scheduler::Forte => par_forte::par_map(data, func, workers_per_thread),
            Scheduler::Chili => par_chili::par_map(data, func, workers_per_thread),
            Scheduler::Rayon => par_rayon::par_map(data, func),
        }
    }

    #[inline(always)]
    pub fn par_chunks<T, F>(self, data: &mut [T], func: &F, workers_per_thread: u32)
    where
        T: Send + Sync,
        F: Fn(usize, &mut [T]) + Send + Sync,
    {
        match self {
            Scheduler::SequentialOptimized => par_sequential::par_chunks(data, func),
            Scheduler::Sequential => par_sequential::par_chunks(data, func),
            Scheduler::Forte => par_forte::par_chunks(data, func, workers_per_thread),
            Scheduler::Chili => par_chili::par_chunks(data, func, workers_per_thread),
            Scheduler::Rayon => par_rayon::par_chunks(data, func, workers_per_thread),
        }
    }

    #[inline(always)]
    pub fn init(self) {
        match self {
            Scheduler::Forte => {
                par_forte::COMPUTE.resize_to_available();
            }
            Scheduler::Chili => {
                par_chili::init_chili();
            }
            _ => (),
        }
    }
}
