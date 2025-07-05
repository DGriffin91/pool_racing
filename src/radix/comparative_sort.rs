//! `comparative_sort` is a radix-aware comparison sort. It operates on radixes rather than
//! whole numbers to support all the same use-cases as the original radix sort including
//! sorting across multiple keys or partial keys etc.
//!
//! The purpose of this sort is to ensure that the library can provide a simpler interface. Without
//! this sort, users would have to implement both `RadixKey` for the radix sort, _and_ `Ord` for
//! the comparison sort. With this, only `RadixKey` is required.
//!
//! While the performance generally sucks, it is still faster than setting up for a full radix sort
//! in situations where there are very few items.
//!
//! ## Characteristics
//!
//!  * in-place
//!  * unstable
//!  * single-threaded
//!
//! ## Performance
//!
//! This is even slower than a typical comparison sort and so is only used as a fallback for very
//! small inputs. However for those very small inputs it provides a significant speed-up due to
//! having essentially no overhead (from count arrays, buffers etc.) compared to a radix sort.

use std::cmp::Ordering;

use crate::radix::radix_key::RadixKey;

pub(crate) fn comparative_sort<T>(bucket: &mut [T], start_level: usize)
where
    T: RadixKey + Sized + Send + Copy + Sync,
{
    if bucket.len() < 2 {
        return;
    }

    bucket.sort_unstable_by(|a, b| -> Ordering {
        let mut level = start_level;
        loop {
            let cmp = a.get_level(level).cmp(&b.get_level(level));

            if level != 0 && cmp == Ordering::Equal {
                level -= 1;
                continue;
            }

            return cmp;
        }
    });
}
