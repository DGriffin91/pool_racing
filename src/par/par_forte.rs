pub static COMPUTE: forte::ThreadPool = forte::ThreadPool::new();

#[inline(always)]
pub fn par_map<T, F>(data: &mut [T], func: &F, workers_per_thread: u32)
where
    T: Send + Sync,
    F: Fn(usize, &mut T) + Send + Sync,
{
    #[inline(always)]
    fn recursive_split<T, F>(
        worker: &forte::Worker,
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
    let num_threads =
        std::thread::available_parallelism().unwrap().get() as u32 * workers_per_thread.max(1);
    let splits = 31 - num_threads.leading_zeros().max(1);
    COMPUTE.with_worker(|worker| {
        recursive_split(worker, data, &func, 0, splits);
    });
}

#[inline(always)]
pub fn par_chunks<T, F>(data: &mut [T], func: &F, workers_per_thread: u32)
where
    T: Send + Sync,
    F: Fn(usize, &mut [T]) + Send + Sync,
{
    #[inline(always)]
    fn recursive_split<T, F>(
        worker: &forte::Worker,
        data: &mut [T],
        func: &F,
        base_id: usize,
        splits_left: u32,
    ) where
        T: Send + Sync,
        F: Fn(usize, &mut [T]) + Send + Sync,
    {
        if splits_left == 0 {
            func(base_id, data);
        } else {
            let split_id = data.len() / 2;
            let (left, right) = data.split_at_mut(split_id);
            worker.join(
                |worker| recursive_split(worker, left, func, base_id, splits_left - 1),
                |worker| recursive_split(worker, right, func, base_id + split_id, splits_left - 1),
            );
        }
    }

    let num_threads =
        std::thread::available_parallelism().unwrap().get() as u32 * workers_per_thread.max(1);
    let splits = 31 - num_threads.leading_zeros().max(1);
    COMPUTE.with_worker(|worker| {
        recursive_split(worker, data, &func, 0, splits);
    });
}
