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
pub fn par_chunks<T, F>(data: &mut [T], func: &F)
where
    T: Send + Sync,
    F: Fn(usize, &mut [T]) + Send + Sync,
{
    func(0, data)
}
