use std::str::FromStr;

pub mod par_chili;
pub mod par_forte;
pub mod par_rayon;
pub mod par_sequential;

// Used for now instead of features just for rust-analyzer
#[derive(PartialEq, Eq, Default, Clone, Copy)]
#[repr(u32)]
pub enum Scheduler {
    SequentialOptimized = 0,
    Sequential = 1,
    #[default]
    Forte = 2,
    Chili = 3,
    Rayon = 4,
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
    pub fn from(value: u32) -> Self {
        match value {
            0 => Scheduler::SequentialOptimized,
            1 => Scheduler::Sequential,
            2 => Scheduler::Forte,
            3 => Scheduler::Chili,
            4 => Scheduler::Rayon,
            _ => panic!("invalid scheduler enum value: {value}"),
        }
    }

    #[inline(always)]
    pub fn par_map<T, F>(self, data: &mut [T], func: &F, chunks: u32)
    where
        T: Send + Sync,
        F: Fn(usize, &mut T) + Send + Sync,
    {
        match self {
            Scheduler::SequentialOptimized => par_sequential::par_map(data, func),
            Scheduler::Sequential => par_sequential::par_map(data, func),
            Scheduler::Forte => par_forte::par_map(data, func, chunks),
            Scheduler::Chili => par_chili::par_map(data, func, chunks),
            Scheduler::Rayon => par_rayon::par_map(data, func),
        }
    }

    #[inline(always)]
    pub fn par_chunks_mut<T, F>(self, data: &mut [T], func: &F, chunks: u32)
    where
        T: Send + Sync,
        F: Fn(usize, &mut [T]) + Send + Sync,
    {
        match self {
            Scheduler::SequentialOptimized => par_sequential::par_chunks_mut(data, func, chunks),
            Scheduler::Sequential => par_sequential::par_chunks_mut(data, func, chunks),
            Scheduler::Forte => par_forte::par_chunks_mut(data, func, chunks),
            Scheduler::Chili => par_chili::par_chunks_mut(data, func, chunks),
            Scheduler::Rayon => par_rayon::par_chunks_mut(data, func, chunks),
        }
    }

    #[inline(always)]
    pub fn par_chunks<T, F>(self, data: &[T], func: &F, chunks: u32)
    where
        T: Send + Sync,
        F: Fn(usize, &[T]) + Send + Sync,
    {
        match self {
            Scheduler::SequentialOptimized => par_sequential::par_chunks(data, func, chunks),
            Scheduler::Sequential => par_sequential::par_chunks(data, func, chunks),
            Scheduler::Forte => par_forte::par_chunks(data, func, chunks),
            Scheduler::Chili => par_chili::par_chunks(data, func, chunks),
            Scheduler::Rayon => par_rayon::par_chunks(data, func, chunks),
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

    pub fn current_num_threads(self) -> usize {
        // TODO replicate rayon::current_num_threads() for forte and chili
        match self {
            Scheduler::SequentialOptimized => 1,
            Scheduler::Sequential => 1,
            Scheduler::Forte => std::thread::available_parallelism().unwrap().get(),
            Scheduler::Chili => std::thread::available_parallelism().unwrap().get(),
            Scheduler::Rayon => rayon::current_num_threads(),
        }
    }
}
