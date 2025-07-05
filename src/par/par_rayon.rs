use rayon::iter::IntoParallelRefMutIterator;
use rayon::{
    iter::{IndexedParallelIterator, ParallelIterator},
    slice::ParallelSliceMut,
};

#[inline(always)]
pub fn par_map<T, F>(data: &mut [T], func: &F)
where
    T: Send + Sync,
    F: Fn(usize, &mut T) + Send + Sync,
{
    data.par_iter_mut()
        .enumerate()
        .for_each(|(index, item)| func(index, item));
}

#[inline(always)]
pub fn par_chunks<T, F>(data: &mut [T], func: &F, workers_per_thread: u32)
where
    T: Send + Sync,
    F: Fn(usize, &mut [T]) + Send + Sync,
{
    let available_parallelism =
        std::thread::available_parallelism().unwrap().get() * workers_per_thread.max(1) as usize;
    data.par_chunks_mut(available_parallelism)
        .enumerate()
        .for_each(|(chunk_index, chunk)| {
            let start = chunk_index * available_parallelism;
            func(start, chunk)
        });
}
