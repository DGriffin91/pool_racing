use std::sync::Once;

static mut COMPUTE: Option<chili::Scope> = None;
static INIT: Once = Once::new();

pub fn init_chili() {
    unsafe {
        INIT.call_once(|| {
            COMPUTE = Some(chili::Scope::global());
        });
    }
}

#[inline(always)]
pub fn with_chili<F, R>(f: F) -> R
where
    F: FnOnce(&mut chili::Scope) -> R,
{
    #[allow(static_mut_refs)]
    f(unsafe { COMPUTE.as_mut().unwrap() }) // chat, is this ub?
}

#[inline(always)]
pub fn par_map<T, F>(data: &mut [T], func: &F, chunks: u32)
where
    T: Send + Sync,
    F: Fn(usize, &mut T) + Send + Sync,
{
    #[inline(always)]
    fn recursive_split<T, F>(
        worker: &mut chili::Scope,
        data: &mut [T],
        func: &F,
        base_id: usize,
        splits_left: u32,
    ) where
        T: Send + Sync,
        F: Fn(usize, &mut T) + Send + Sync,
    {
        if splits_left == 0 {
            for (index, output) in data.iter_mut().enumerate() {
                func(base_id + index, output);
            }
        } else {
            let split_id = data.len() / 2;
            let (left, right) = data.split_at_mut(split_id);
            worker.join(
                |worker| recursive_split(worker, left, func, base_id, splits_left - 1),
                |worker| recursive_split(worker, right, func, base_id + split_id, splits_left - 1),
            );
        }
    }
    let splits = 31 - chunks.leading_zeros().max(1);
    with_chili(|worker| {
        recursive_split(worker, data, &func, 0, splits);
    });
}

#[inline(always)]
pub fn par_chunks_mut<T, F>(data: &mut [T], func: &F, chunk_size: usize)
where
    T: Send + Sync,
    F: Fn(usize, &mut [T]) + Send + Sync,
{
    fn recursive_split<T, F>(
        worker: &mut chili::Scope,
        start_chunk: usize,
        slice: &mut [T],
        func: &F,
        chunk_size: usize,
    ) where
        T: Send + Sync,
        F: Fn(usize, &mut [T]) + Send + Sync,
    {
        let len = slice.len();
        if len <= chunk_size {
            func(start_chunk, slice);
        } else {
            let n_chunks = len.div_ceil(chunk_size);
            let left_chunks = n_chunks / 2;
            let left_len = left_chunks * chunk_size;
            let left_len = left_len.min(len);
            let (left, right) = slice.split_at_mut(left_len);

            worker.join(
                |worker| recursive_split(worker, start_chunk, left, func, chunk_size),
                |worker| {
                    recursive_split(worker, start_chunk + left_chunks, right, func, chunk_size)
                },
            );
        }
    }
    if !data.is_empty() {
        with_chili(|worker| {
            recursive_split(worker, 0, data, func, chunk_size.max(1));
        });
    }
}

#[inline(always)]
pub fn par_chunks<T, F>(data: &[T], func: &F, chunk_size: usize)
where
    T: Send + Sync,
    F: Fn(usize, &[T]) + Send + Sync,
{
    fn recursive_split<T, F>(
        worker: &mut chili::Scope,
        start_chunk: usize,
        slice: &[T],
        func: &F,
        chunk_size: usize,
    ) where
        T: Send + Sync,
        F: Fn(usize, &[T]) + Send + Sync,
    {
        let len = slice.len();
        if len <= chunk_size {
            func(start_chunk, slice);
        } else {
            let n_chunks = len.div_ceil(chunk_size);
            let left_chunks = n_chunks / 2;
            let left_len = left_chunks * chunk_size;
            let left_len = left_len.min(len);
            let (left, right) = slice.split_at(left_len);

            worker.join(
                |worker| recursive_split(worker, start_chunk, left, func, chunk_size),
                |worker| {
                    recursive_split(worker, start_chunk + left_chunks, right, func, chunk_size)
                },
            );
        }
    }
    if !data.is_empty() {
        with_chili(|worker| {
            recursive_split(worker, 0, data, func, chunk_size.max(1));
        });
    }
}
