use std::sync::Once;

use bevy_tasks::{TaskPool, TaskPoolBuilder};

use crate::par::cached_available_parallelism;

static mut COMPUTE: Option<TaskPool> = None;
static INIT: Once = Once::new();

pub fn init_bevy() {
    unsafe {
        INIT.call_once(|| {
            let n = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1);
            let pool = TaskPoolBuilder::new().num_threads(n).build();
            COMPUTE = Some(pool);
        });
    }
}

#[inline(always)]
pub fn with_bevy<F, R>(f: F) -> R
where
    F: FnOnce(&TaskPool) -> R,
{
    #[allow(static_mut_refs)]
    f(unsafe { COMPUTE.as_ref().unwrap() })
}

#[inline(always)]
pub fn par_map<T, F>(data: &mut [T], func: &F, chunks: u32)
where
    T: Send + Sync,
    F: Fn(usize, &mut T) + Send + Sync,
{
    if !data.is_empty() {
        let max_chunks = cached_available_parallelism();
        // https://github.com/bevyengine/bevy/blob/20dfae9a2d07038bda2921f82af50ded6151c3de/crates/bevy_ecs/src/batching.rs#L94

        let chunk_count = (chunks as usize).max(1).min(max_chunks);
        let chunk_size = data.len().div_ceil(chunk_count);
        if chunk_count == 1 {
            for (i, output) in data.iter_mut().enumerate() {
                func(i, output);
            }
        } else {
            with_bevy(|worker| {
                worker.scope(|s| {
                    let mut slice = data;
                    for chunk_id in 0..chunk_count {
                        let slice_len = slice.len();
                        let (left, right) = slice.split_at_mut(chunk_size.min(slice_len));
                        slice = right;
                        if chunk_id == chunk_count - 1 {
                            let start = chunk_id * chunk_size;
                            for (i, output) in left.iter_mut().enumerate() {
                                func(start + i, output);
                            }
                        } else {
                            s.spawn(async move {
                                let start = chunk_id * chunk_size;
                                for (i, output) in left.iter_mut().enumerate() {
                                    func(start + i, output);
                                }
                            });
                        }
                    }
                });
            });
        }
    }
}

#[inline(always)]
pub fn par_chunks_mut<T, F>(data: &mut [T], func: &F, chunk_size: usize)
where
    T: Send + Sync,
    F: Fn(usize, &mut [T]) + Send + Sync,
{
    if !data.is_empty() {
        let chunk_size = chunk_size.max(1);
        let chunk_count = data.len().div_ceil(chunk_size);
        if chunk_count == 1 {
            func(0, data)
        } else {
            with_bevy(|worker| {
                worker.scope(|s| {
                    let mut slice = data;
                    for chunk_id in 0..chunk_count {
                        let slice_len = slice.len();
                        let (left, right) = slice.split_at_mut(chunk_size.min(slice_len));
                        slice = right;
                        if chunk_id == chunk_count - 1 {
                            func(chunk_id, left) // Run the last one on this thread
                        } else {
                            s.spawn(async move { func(chunk_id, left) });
                        }
                    }
                });
            });
        }
    }
}

#[inline(always)]
pub fn par_chunks<T, F>(data: &[T], func: &F, chunk_size: usize)
where
    T: Send + Sync,
    F: Fn(usize, &[T]) + Send + Sync,
{
    if !data.is_empty() {
        let chunk_size = chunk_size.max(1);
        let chunk_count = data.len().div_ceil(chunk_size);
        if chunk_count == 1 {
            func(0, data)
        } else {
            with_bevy(|worker| {
                worker.scope(|s| {
                    let mut slice = data;
                    for chunk_id in 0..chunk_count {
                        let slice_len = slice.len();
                        let (left, right) = slice.split_at(chunk_size.min(slice_len));
                        slice = right;
                        if chunk_id == chunk_count - 1 {
                            func(chunk_id, left) // Run the last one on this thread
                        } else {
                            s.spawn(async move { func(chunk_id, left) });
                        }
                    }
                });
            });
        }
    }
}
