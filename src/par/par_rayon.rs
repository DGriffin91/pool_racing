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
pub fn par_chunks_mut<T, F>(data: &mut [T], func: &F, chunk_size: usize)
where
    T: Send + Sync,
    F: Fn(usize, &mut [T]) + Send + Sync,
{
    if !data.is_empty() {
        data.par_chunks_mut(chunk_size.max(1))
            .enumerate()
            .for_each(|(chunk_index, chunk)| func(chunk_index, chunk));
    }
}

#[inline(always)]
pub fn par_chunks<T, F>(data: &[T], func: &F, chunk_size: usize)
where
    T: Send + Sync,
    F: Fn(usize, &[T]) + Send + Sync,
{
    if !data.is_empty() {
        data.par_chunks(chunk_size.max(1))
            .enumerate()
            .for_each(|(chunk_index, chunk)| func(chunk_index, chunk));
    }
}
