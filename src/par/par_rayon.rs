use rayon::iter::IntoParallelRefMutIterator;
use rayon::slice::ParallelSlice;
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
pub fn par_chunks_mut<T, F>(data: &mut [T], func: &F, chunks: u32)
where
    T: Send + Sync,
    F: Fn(usize, &mut [T]) + Send + Sync,
{
    let chunks = (data.len() / chunks as usize).max(1);
    data.par_chunks_mut(chunks)
        .enumerate()
        .for_each(|(chunk_index, chunk)| {
            let start = chunk_index * chunks;
            func(start, chunk)
        });
}

#[inline(always)]
pub fn par_chunks<T, F>(data: &[T], func: &F, chunks: u32)
where
    T: Send + Sync,
    F: Fn(usize, &[T]) + Send + Sync,
{
    let chunks = (data.len() / chunks as usize).max(1);
    data.par_chunks(chunks)
        .enumerate()
        .for_each(|(chunk_index, chunk)| {
            let start = chunk_index * chunks;
            func(start, chunk)
        });
}
