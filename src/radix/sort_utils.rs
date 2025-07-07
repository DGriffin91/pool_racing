use std::sync::mpsc::channel;

use crate::radix::{radix_key::RadixKey, radix_scheduler};

#[inline]
pub fn get_prefix_sums(counts: &[usize; 256]) -> [usize; 256] {
    crate::scope!("get_prefix_sums");
    let mut sums = [0usize; 256];

    let mut running_total = 0;
    for (i, c) in counts.iter().enumerate() {
        sums[i] = running_total;
        running_total += c;
    }

    sums
}

#[inline]
pub fn get_end_offsets(counts: &[usize; 256], prefix_sums: &[usize; 256]) -> [usize; 256] {
    let mut end_offsets = [0usize; 256];

    end_offsets[0..255].copy_from_slice(&prefix_sums[1..256]);
    end_offsets[255] = counts[255] + prefix_sums[255];

    end_offsets
}

#[inline]
pub fn par_get_counts_with_ends<T>(bucket: &[T], level: usize) -> ([usize; 256], bool, u8, u8)
where
    T: RadixKey + Sized + Send + Sync,
{
    crate::scope!("par_get_counts_with_ends");
    if bucket.len() < 400_000 {
        return get_counts_with_ends(bucket, level);
    }

    let threads = radix_scheduler().current_num_threads();
    let chunk_divisor = 8;
    let chunk_size = (bucket.len() / threads / chunk_divisor) + 1;
    let len = bucket.len().div_ceil(chunk_size);
    let (tx, rx) = channel();

    // Original rayon version:
    //let chunks = bucket.par_chunks(chunk_size);
    //let len = chunks.len();
    //chunks.enumerate().for_each_with(tx, |tx, (i, chunk)| {
    //    let counts = get_counts_with_ends(chunk, level);
    //    tx.send((i, counts.0, counts.1, counts.2, counts.3))
    //        .unwrap();
    //});

    radix_scheduler().par_chunks(
        bucket,
        &|i, chunk| {
            let counts = get_counts_with_ends(chunk, level);
            tx.send((i, counts.0, counts.1, counts.2, counts.3))
                .unwrap();
        },
        chunk_size,
    );

    let mut msb_counts = [0usize; 256];
    let mut already_sorted = true;
    let mut boundaries = vec![(0u8, 0u8); len];

    for _ in 0..len {
        let (i, counts, chunk_sorted, start, end) = rx.recv().unwrap();

        if !chunk_sorted {
            already_sorted = false;
        }

        boundaries[i].0 = start;
        boundaries[i].1 = end;

        for (i, c) in counts.iter().enumerate() {
            msb_counts[i] += *c;
        }
    }

    // Check the boundaries of each counted chunk, to see if the full bucket
    // is already sorted
    if already_sorted {
        for w in boundaries.windows(2) {
            if w[1].0 < w[0].1 {
                already_sorted = false;
                break;
            }
        }
    }

    (
        msb_counts,
        already_sorted,
        boundaries[0].0,
        boundaries[boundaries.len() - 1].1,
    )
}

#[inline]
pub fn get_counts_with_ends<T>(bucket: &[T], level: usize) -> ([usize; 256], bool, u8, u8)
where
    T: RadixKey,
{
    crate::scope!("get_counts_with_ends");
    let mut already_sorted = true;
    let mut continue_from = bucket.len();
    let mut counts_1 = [0usize; 256];
    let mut last = 0usize;

    for (i, item) in bucket.iter().enumerate() {
        let b = item.get_level(level) as usize;
        counts_1[b] += 1;

        if b < last {
            continue_from = i + 1;
            already_sorted = false;
            break;
        }

        last = b;
    }

    if continue_from == bucket.len() {
        return (
            counts_1,
            already_sorted,
            bucket[0].get_level(level),
            last as u8,
        );
    }

    let mut counts_2 = [0usize; 256];
    let mut counts_3 = [0usize; 256];
    let mut counts_4 = [0usize; 256];
    let chunks = bucket[continue_from..].chunks_exact(4);
    let rem = chunks.remainder();

    chunks.into_iter().for_each(|chunk| {
        let a = chunk[0].get_level(level) as usize;
        let b = chunk[1].get_level(level) as usize;
        let c = chunk[2].get_level(level) as usize;
        let d = chunk[3].get_level(level) as usize;

        counts_1[a] += 1;
        counts_2[b] += 1;
        counts_3[c] += 1;
        counts_4[d] += 1;
    });

    rem.iter().for_each(|v| {
        let b = v.get_level(level) as usize;
        counts_1[b] += 1;
    });

    for i in 0..256 {
        counts_1[i] += counts_2[i];
        counts_1[i] += counts_3[i];
        counts_1[i] += counts_4[i];
    }

    let b_first = bucket.first().unwrap().get_level(level);
    let b_last = bucket.last().unwrap().get_level(level);

    (counts_1, already_sorted, b_first, b_last)
}

#[inline]
pub fn get_counts<T>(bucket: &[T], level: usize) -> ([usize; 256], bool)
where
    T: RadixKey,
{
    if bucket.is_empty() {
        return ([0usize; 256], true);
    }

    let (counts, sorted, _, _) = get_counts_with_ends(bucket, level);

    (counts, sorted)
}

#[inline]
pub fn get_tile_counts<T>(bucket: &[T], tile_size: usize, level: usize) -> (Vec<[usize; 256]>, bool)
where
    T: RadixKey + Copy + Sized + Send + Sync,
{
    crate::scope!("get_tile_counts");
    // Original rayon version:
    //let tiles: Vec<([usize; 256], bool, u8, u8)> = bucket
    //    .par_chunks(tile_size)
    //    .map(|chunk| par_get_counts_with_ends(chunk, level))
    //    .collect();

    let tile_count = bucket.len().div_ceil(tile_size);

    let mut tiles: Vec<([usize; 256], bool, u8, u8)> =
        vec![([0usize; 256], false, 0u8, 0u8); tile_count];

    radix_scheduler().par_map(
        &mut tiles,
        &|i, tile| {
            let start = i * tile_size;
            let end = (start + tile_size).min(bucket.len());
            *tile = par_get_counts_with_ends(&bucket[start..end], level)
        },
        tile_count as u32,
    );

    let mut all_sorted = true;

    if tiles.len() == 1 {
        // If there is only one tile, we already have a flag for if it is sorted
        all_sorted = tiles[0].1;
    } else {
        // Check if any of the tiles, or any of the tile boundaries are unsorted
        for tile in tiles.windows(2) {
            if !tile[0].1 || !tile[1].1 || tile[1].2 < tile[0].3 {
                all_sorted = false;
                break;
            }
        }
    }

    (tiles.into_iter().map(|v| v.0).collect(), all_sorted)
}

#[inline]
pub fn aggregate_tile_counts(tile_counts: &[[usize; 256]]) -> [usize; 256] {
    crate::scope!("aggregate_tile_counts");
    let mut out = tile_counts[0];
    for tile in tile_counts.iter().skip(1) {
        for i in 0..256 {
            out[i] += tile[i];
        }
    }

    out
}

#[inline]
pub fn is_homogenous_bucket(counts: &[usize; 256]) -> bool {
    crate::scope!("is_homogenous_bucket");
    let mut seen = false;
    for c in counts {
        if *c > 0 {
            if seen {
                return false;
            } else {
                seen = true;
            }
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use crate::radix::sort_utils::get_tile_counts;

    #[test]
    pub fn test_get_tile_counts_correctly_marks_already_sorted_single_tile() {
        let mut data: Vec<u8> = vec![0, 5, 2, 3, 1];

        let (_counts, already_sorted) = get_tile_counts(&mut data, 5, 0);
        assert_eq!(already_sorted, false);

        let mut data: Vec<u8> = vec![0, 0, 1, 1, 2];

        let (_counts, already_sorted) = get_tile_counts(&mut data, 5, 0);
        assert_eq!(already_sorted, true);
    }

    #[test]
    pub fn test_get_tile_counts_correctly_marks_already_sorted_multiple_tiles() {
        let mut data: Vec<u8> = vec![0, 5, 2, 3, 1];

        let (_counts, already_sorted) = get_tile_counts(&mut data, 2, 0);
        assert_eq!(already_sorted, false);

        let mut data: Vec<u8> = vec![0, 0, 1, 1, 2];

        let (_counts, already_sorted) = get_tile_counts(&mut data, 2, 0);
        assert_eq!(already_sorted, true);
    }
}
