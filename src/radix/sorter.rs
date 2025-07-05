use arbitrary_chunks::ArbitraryChunks;
use std::cmp::max;

use crate::radix::{
    comparative_sort::comparative_sort,
    radix_key::RadixKey,
    regions_sort::regions_sort_adapter,
    ska_sort::ska_sort_adapter,
    sort_utils::{aggregate_tile_counts, get_counts, get_tile_counts, is_homogenous_bucket},
    DEFAULT_SCHEDULER,
};

#[inline]
fn handle_chunk<T>(chunk: &mut [T], level: usize, threads: usize)
where
    T: RadixKey + Sized + Send + Copy + Sync,
{
    if chunk.len() <= 1 {
        return;
    } else if chunk.len() <= 128 {
        comparative_sort(chunk, level);
        return;
    }

    let use_tiles = chunk.len() >= 260_000;
    let tile_size = if use_tiles {
        max(30_000, chunk.len().div_ceil(threads))
    } else {
        chunk.len()
    };

    let mut tile_counts: Option<Vec<[usize; 256]>> = None;
    let mut already_sorted = false;

    if use_tiles {
        let (tc, s) = get_tile_counts(chunk, tile_size, level);
        tile_counts = Some(tc);
        already_sorted = s;
    }

    let counts = if let Some(tile_counts) = &tile_counts {
        aggregate_tile_counts(tile_counts)
    } else {
        let (counts, s) = get_counts(chunk, level);
        already_sorted = s;

        counts
    };

    if already_sorted || (chunk.len() >= 30_000 && is_homogenous_bucket(&counts)) {
        if level != 0 {
            director(chunk, &counts, level - 1);
        }

        return;
    }

    // Ensure tile_counts is always set when it is required
    if tile_counts.is_none() {
        tile_counts = Some(vec![counts]);
    }

    if let Some(tile_counts) = tile_counts {
        regions_sort_adapter(chunk, &counts, &tile_counts, tile_size, level)
    } else {
        ska_sort_adapter(chunk, &counts, level)
    }
}

#[inline]
pub fn director<T>(bucket: &mut [T], counts: &[usize; 256], level: usize)
where
    T: RadixKey + Send + Sync + Copy,
{
    // Original rayon version:
    // bucket.arbitrary_chunks_mut(counts).par_bridge()
    //       .for_each(|chunk| handle_chunk(chunk, level, current_num_threads()));

    // TODO perf was using par_bridge with rayon, don't allocate
    let mut chunks = bucket.arbitrary_chunks_mut(counts).collect::<Vec<_>>();
    DEFAULT_SCHEDULER.par_map(
        &mut chunks,
        &|_, chunk| handle_chunk(chunk, level, DEFAULT_SCHEDULER.current_num_threads()),
        1,
    )
}

#[inline]
pub fn sort<T>(data: &mut [T])
where
    T: RadixKey + Copy + Send + Sync,
{
    // By definition, this is already sorted
    if data.len() <= 1 {
        return;
    }

    let threads = DEFAULT_SCHEDULER.current_num_threads();
    let level = T::LEVELS - 1;
    handle_chunk(data, level, threads);
}
