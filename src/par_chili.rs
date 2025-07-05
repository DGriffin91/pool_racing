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
    f(unsafe { &mut COMPUTE.as_mut().unwrap() }) // chat, is this ub?
}

#[inline(always)]
pub fn par_map<T, F>(data: &mut [T], func: &F)
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
    let num_threads = std::thread::available_parallelism().unwrap().get() as u32;
    let splits = 31 - num_threads.leading_zeros().max(1);
    with_chili(|worker| {
        recursive_split(worker, data, &func, 0, splits);
    });
}

#[inline(always)]
pub fn par_chunks<T, F>(data: &mut [T], func: &F)
where
    T: Send + Sync,
    F: Fn(usize, &mut [T]) + Send + Sync,
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

    let num_threads = std::thread::available_parallelism().unwrap().get() as u32;
    let splits = 31 - num_threads.leading_zeros().max(1);
    with_chili(|worker| {
        recursive_split(worker, data, &func, 0, splits);
    });
}
