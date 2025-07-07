//! `ska_sort` is a single-threaded, in-place algorithm described by Malte Skarupke.
//!
//! <https://probablydance.com/2016/12/27/i-wrote-a-faster-sorting-algorithm/>
//!
//! This implementation isn't entirely faithful to the original, however it follows the general
//! principle of skipping over the largest output bucket and simply swapping the remaining buckets
//! until the entire thing is sorted.
//!
//! The in-place nature of this algorithm makes it very efficient memory-wise.
//!
//! ## Characteristics
//!
//!  * in-place
//!  * memory efficient
//!  * unstable
//!  * single-threaded
//!
//! ## Performance
//!
//! This is generally slower than `lsb_sort` for smaller types T or smaller input arrays. For larger
//! types or inputs, the memory efficiency of this algorithm can make it faster than `lsb_sort`.

use partition::partition_index;

use crate::radix::{
    radix_key::RadixKey,
    sort_utils::{get_end_offsets, get_prefix_sums},
    sorter::director,
};

pub fn ska_sort<T>(
    bucket: &mut [T],
    prefix_sums: &mut [usize; 256],
    end_offsets: &[usize; 256],
    level: usize,
) where
    T: RadixKey + Sized + Send + Copy + Sync,
{
    crate::scope!("ska_sort");
    let mut finished = 0;
    let mut finished_map = [false; 256];
    let mut largest = 0;
    let mut largest_index = 0;

    for i in 0..256 {
        let rem = end_offsets[i] - prefix_sums[i];
        if rem == 0 {
            finished_map[i] = true;
            finished += 1;
        } else if rem > largest {
            largest = rem;
            largest_index = i;
        }
    }

    if largest == bucket.len() {
        // Already sorted
        return;
    } else if largest > (bucket.len() / 2) {
        // Partition in-place the largest chunk so we don't spend all our time
        // swapping things in and out that are already in the correct place.

        let li = largest_index as u8;
        let offs = partition_index(
            &mut bucket[prefix_sums[largest_index]..end_offsets[largest_index]],
            |v| v.get_level(level) == li,
        );

        prefix_sums[largest_index] += offs;
    }

    if !finished_map[largest_index] {
        finished_map[largest_index] = true;
        finished += 1;
    }

    while finished != 256 {
        for b in 0..256 {
            if finished_map[b] {
                continue;
            } else if prefix_sums[b] >= end_offsets[b] {
                finished_map[b] = true;
                finished += 1;
            }

            for i in prefix_sums[b]..end_offsets[b] {
                let new_b = bucket[i].get_level(level) as usize;
                bucket.swap(prefix_sums[new_b], i);
                prefix_sums[new_b] += 1;
            }
        }
    }
}

pub(crate) fn ska_sort_adapter<T>(bucket: &mut [T], counts: &[usize; 256], level: usize)
where
    T: RadixKey + Sized + Send + Copy + Sync,
{
    if bucket.len() < 2 {
        return;
    }

    let mut prefix_sums = get_prefix_sums(counts);
    let end_offsets = get_end_offsets(counts, &prefix_sums);

    ska_sort(bucket, &mut prefix_sums, &end_offsets, level);

    if level == 0 {
        return;
    }

    director(bucket, counts, level - 1);
}
