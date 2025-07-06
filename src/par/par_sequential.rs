#[inline(always)]
pub fn par_map<T, F>(data: &mut [T], func: &F)
where
    T: Send + Sync,
    F: Fn(usize, &mut T) + Send + Sync,
{
    for (index, output) in data.iter_mut().enumerate() {
        func(index, output);
    }
}

#[inline(always)]
pub fn par_chunks_mut<T, F>(data: &mut [T], func: &F, chunk_size: usize)
where
    T: Send + Sync,
    F: Fn(usize, &mut [T]) + Send + Sync,
{
    fn recursive_split<T, F>(start_chunk: usize, slice: &mut [T], func: &F, chunk_size: usize)
    where
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

            recursive_split(start_chunk, left, func, chunk_size);
            recursive_split(start_chunk + left_chunks, right, func, chunk_size);
        }
    }
    if !data.is_empty() {
        recursive_split(0, data, func, chunk_size.max(1));
    }
}

#[inline(always)]
pub fn par_chunks<T, F>(data: &[T], func: &F, chunk_size: usize)
where
    T: Send + Sync,
    F: Fn(usize, &[T]) + Send + Sync,
{
    fn recursive_split<T, F>(start_chunk: usize, slice: &[T], func: &F, chunk_size: usize)
    where
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

            recursive_split(start_chunk, left, func, chunk_size);
            recursive_split(start_chunk + left_chunks, right, func, chunk_size);
        }
    }
    if !data.is_empty() {
        recursive_split(0, data, func, chunk_size.max(1));
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
