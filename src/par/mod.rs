use std::{str::FromStr, sync::Once};

pub mod par_chili;
pub mod par_forte;
pub mod par_raw;
pub mod par_rayon;
pub mod par_sequential;

static INIT: Once = Once::new();
static mut AVAILABLE_PARALLELISM: usize = 1;

fn init_available_parallelism() {
    INIT.call_once(|| {
        let n = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        unsafe {
            // SAFETY: This is in a call_once
            AVAILABLE_PARALLELISM = n;
        }
    });
}

#[inline(always)]
pub fn cached_available_parallelism() -> usize {
    // SAFETY: We don't mutate
    unsafe { AVAILABLE_PARALLELISM }
}

// Used for now instead of features just for rust-analyzer
#[derive(PartialEq, Eq, Default, Clone, Copy, Debug)]
#[repr(u32)]
pub enum Scheduler {
    SequentialOptimized = 0,
    Sequential = 1,
    #[default]
    Forte = 2,
    Chili = 3,
    Rayon = 4,
    Raw = 5,
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
            "raw" => Ok(Self::Raw),
            _ => Err(format!(
                "Unknown mode: '{s}', valid modes: 'seq_opt', 'seq', 'forte', 'chili', 'rayon', 'raw'"
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
            5 => Scheduler::Raw,
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
            Scheduler::Raw => par_raw::par_map(data, func, chunks),
        }
    }

    #[inline(always)]
    pub fn par_chunks_mut<T, F>(self, data: &mut [T], func: &F, chunk_size: usize)
    where
        T: Send + Sync,
        F: Fn(usize, &mut [T]) + Send + Sync,
    {
        match self {
            Scheduler::SequentialOptimized => {
                par_sequential::par_chunks_mut(data, func, chunk_size)
            }
            Scheduler::Sequential => par_sequential::par_chunks_mut(data, func, chunk_size),
            Scheduler::Forte => par_forte::par_chunks_mut(data, func, chunk_size),
            Scheduler::Chili => par_chili::par_chunks_mut(data, func, chunk_size),
            Scheduler::Rayon => par_rayon::par_chunks_mut(data, func, chunk_size),
            Scheduler::Raw => par_raw::par_chunks_mut(data, func, chunk_size),
        }
    }

    #[inline(always)]
    pub fn par_chunks<T, F>(self, data: &[T], func: &F, chunk_size: usize)
    where
        T: Send + Sync,
        F: Fn(usize, &[T]) + Send + Sync,
    {
        match self {
            Scheduler::SequentialOptimized => par_sequential::par_chunks(data, func, chunk_size),
            Scheduler::Sequential => par_sequential::par_chunks(data, func, chunk_size),
            Scheduler::Forte => par_forte::par_chunks(data, func, chunk_size),
            Scheduler::Chili => par_chili::par_chunks(data, func, chunk_size),
            Scheduler::Rayon => par_rayon::par_chunks(data, func, chunk_size),
            Scheduler::Raw => par_raw::par_chunks(data, func, chunk_size),
        }
    }

    #[inline(always)]
    pub fn init(self) {
        init_available_parallelism();
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
            Scheduler::Forte => cached_available_parallelism(),
            Scheduler::Chili => cached_available_parallelism(),
            Scheduler::Rayon => cached_available_parallelism(),
            Scheduler::Raw => cached_available_parallelism(),
        }
    }
}
