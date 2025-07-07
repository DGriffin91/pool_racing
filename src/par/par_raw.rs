use std::thread;

use crate::par::cached_available_parallelism;

pub static COMPUTE: forte::ThreadPool = forte::ThreadPool::new();

#[inline(always)]
pub fn par_map<T, F>(data: &mut [T], func: &F, chunks: u32)
where
    T: Send + Sync,
    F: Fn(usize, &mut T) + Send + Sync,
{
    if !data.is_empty() {
        // Limit the max number of chunks in this case since they are actual threads
        let max_chunks = cached_available_parallelism() * 6;

        let chunk_count = (chunks as usize).max(1).min(max_chunks);
        let chunk_size = data.len().div_ceil(chunk_count);
        if chunk_count == 1 {
            for (i, output) in data.iter_mut().enumerate() {
                func(i, output);
            }
        } else {
            thread::scope(|s| {
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
                        s.spawn(move || {
                            let start = chunk_id * chunk_size;
                            for (i, output) in left.iter_mut().enumerate() {
                                func(start + i, output);
                            }
                        });
                    }
                }
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
            thread::scope(|s| {
                let mut slice = data;
                for chunk_id in 0..chunk_count {
                    let slice_len = slice.len();
                    let (left, right) = slice.split_at_mut(chunk_size.min(slice_len));
                    slice = right;
                    if chunk_id == chunk_count - 1 {
                        func(chunk_id, left) // Run the last one on this thread
                    } else {
                        s.spawn(move || func(chunk_id, left));
                    }
                }
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
            thread::scope(|s| {
                let mut slice = data;
                for chunk_id in 0..chunk_count {
                    let slice_len = slice.len();
                    let (left, right) = slice.split_at(chunk_size.min(slice_len));
                    slice = right;
                    if chunk_id == chunk_count - 1 {
                        func(chunk_id, left) // Run the last one on this thread
                    } else {
                        s.spawn(move || func(chunk_id, left));
                    }
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU32;

    #[test]
    fn test_par_chunks_mut_basic_increment() {
        for chunk_size in 1..24 {
            for data_len in 1..24 {
                //dbg!(chunk_size, data_len);
                let mut data = vec![0; data_len];
                data.iter_mut().enumerate().for_each(|(i, d)| *d = i as u32);
                let list: Vec<AtomicU32> = (0..data_len).map(|_| AtomicU32::new(0)).collect();
                let func = |chunk_id: usize, chunk: &mut [u32]| {
                    let offset = chunk_id * chunk_size;
                    for (i, item) in chunk.iter().enumerate() {
                        list[offset + i].store(
                            offset as u32 + i as u32,
                            std::sync::atomic::Ordering::Relaxed,
                        );
                        assert_eq!(*item as usize, offset + i);
                    }
                    //dbg!(&chunk_id, &offset, &chunk);
                    assert_eq!(offset as u32, chunk[0]);
                };
                par_chunks_mut(&mut data, &func, chunk_size);
                assert_eq!(
                    unsafe { std::mem::transmute::<Vec<AtomicU32>, Vec<u32>>(list) },
                    data
                );
            }
        }
    }
}
