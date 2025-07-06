// https://madmann91.github.io/2021/05/05/ploc-revisited.html
// https://github.com/meistdan/ploc/
// https://meistdan.github.io/publications/ploc/paper.pdf
// https://github.com/madmann91/bvh/blob/v1/include/bvh/locally_ordered_clustering_builder.hpp

use std::{
    cell::RefCell,
    mem,
    sync::atomic::{AtomicU32, Ordering},
};

use crate::{
    bvh::{Bvh2, Bvh2Node},
    radix::radix_key::RadixKey,
    scope, scope_print, scope_print_major, Args, Scheduler,
};

use obvhs::{aabb::Aabb, ploc::morton::morton_encode_u64_unorm};

use glam::*;
use thread_local::ThreadLocal;

static PLOC_SCHEDULER: AtomicU32 = AtomicU32::new(0);

pub fn ploc_scheduler() -> Scheduler {
    Scheduler::from(PLOC_SCHEDULER.load(Ordering::Relaxed))
}

pub fn init_ploc_scheduler() {
    let config: Args = argh::from_env();
    config.ploc_sch.init();
    PLOC_SCHEDULER.store(config.ploc_sch as u32, Ordering::Relaxed);
}

#[inline(always)] // This doesn't need to be inlined, but I thought it would funny if everything was.
pub fn build_ploc(aabbs: &[Aabb]) -> Bvh2 {
    scope_print_major!("build_ploc");
    init_ploc_scheduler();

    // How many workers per available_parallelism thread.
    // If tasks take an non-uniform amount of time more workers per thread can improve cpu utilization.
    let default_chunk_count = ploc_scheduler().current_num_threads();

    let prim_count = aabbs.len();

    if prim_count == 0 {
        return Bvh2::default();
    }

    let mut total_aabb = Aabb::empty();
    let mut local_aabbs: ThreadLocal<RefCell<Aabb>> = ThreadLocal::default();

    let mut current_nodes;
    {
        scope_print_major!("init nodes");

        current_nodes = vec![Bvh2Node::default(); prim_count];

        #[inline(always)]
        fn init_node(prim_index: usize, aabb: Aabb, total_aabb: &mut Aabb) -> Bvh2Node {
            total_aabb.extend(aabb.min);
            total_aabb.extend(aabb.max);
            debug_assert!(!aabb.min.is_nan());
            debug_assert!(!aabb.max.is_nan());
            Bvh2Node {
                aabb,
                index: -(prim_index as i32) - 1,
            }
        }

        let chunk_size = current_nodes.len() / default_chunk_count;

        match ploc_scheduler() {
            Scheduler::SequentialOptimized => {
                for (prim_index, aabb) in aabbs.iter().enumerate() {
                    total_aabb.extend(aabb.min).extend(aabb.max);
                    current_nodes[prim_index] =
                        init_node(prim_index, aabbs[prim_index], &mut total_aabb);
                }
            }
            _ => ploc_scheduler().par_chunks_mut(
                &mut current_nodes,
                &|chunk_id: usize, nodes: &mut [Bvh2Node]| {
                    scope!("init_nodes closure");
                    let start = chunk_id * chunk_size;
                    for (i, node) in nodes.iter_mut().enumerate() {
                        let prim_index = start + i;
                        *node = init_node(
                            prim_index,
                            aabbs[prim_index],
                            &mut local_aabbs.get_or_default().borrow_mut(),
                        );
                    }
                },
                chunk_size,
            ),
        }

        if ploc_scheduler() != Scheduler::SequentialOptimized {
            for local_aabb in local_aabbs.iter_mut() {
                total_aabb.extend(local_aabb.get_mut().min);
                total_aabb.extend(local_aabb.get_mut().max);
            }
        }
    }

    // Merge nodes until there is only one left
    let nodes_count = (2 * prim_count as i64 - 1).max(0) as usize;

    let scale = 1.0 / total_aabb.diagonal().as_dvec3();
    let offset = -total_aabb.min.as_dvec3() * scale;

    let mut sorted_nodes = vec![Bvh2Node::default(); current_nodes.len()];

    // Sort primitives according to their morton code
    sort_nodes_m64(&mut current_nodes, &mut sorted_nodes, scale, offset);

    mem::swap(&mut current_nodes, &mut sorted_nodes);

    let mut nodes = vec![Bvh2Node::default(); nodes_count];

    let mut insert_index = nodes_count;

    sorted_nodes.clear(); // Reuse allocation
    let mut next_nodes = sorted_nodes;

    let mut merge: Vec<i8> = vec![0; prim_count];

    #[allow(unused_variables)]
    let mut depth: usize = 0;
    while current_nodes.len() > 1 {
        let mut last_cost = f32::MAX;
        let count = current_nodes.len() - 1;
        assert!(count < merge.len()); // Try to elide bounds check
        {
            scope_print!("ploc calculate merge directions");

            let chunk_size = merge[..count].len() / default_chunk_count;

            let calculate_costs = |chunk_id: usize, chunk: &mut [i8]| {
                scope!("calculate_costs closure");
                let start = chunk_id * chunk_size;
                let mut last_cost = if start == 0 {
                    f32::MAX
                } else {
                    current_nodes[start - 1]
                        .aabb
                        .union(&current_nodes[start].aabb)
                        .half_area()
                };
                for (local_n, merge_n) in chunk.iter_mut().enumerate() {
                    let i = local_n + start;
                    let cost = current_nodes[i]
                        .aabb
                        .union(&current_nodes[i + 1].aabb)
                        .half_area();
                    *merge_n = if last_cost < cost { -1 } else { 1 };
                    last_cost = cost;
                }
            };

            match ploc_scheduler() {
                Scheduler::SequentialOptimized => (0..count).for_each(|i| {
                    let cost = current_nodes[i]
                        .aabb
                        .union(&current_nodes[i + 1].aabb)
                        .half_area();
                    merge[i] = if last_cost < cost { -1 } else { 1 };
                    last_cost = cost;
                }),
                _ => ploc_scheduler().par_chunks_mut(
                    &mut merge[..count],
                    &calculate_costs,
                    chunk_size,
                ),
            }

            // Have the last box to always prefer the box before it since there is none after it
            merge[current_nodes.len() - 1] = -1;
        }

        {
            scope_print!("ploc merge");
            let mut index = 0;
            while index < current_nodes.len() {
                let index_offset = merge[index] as i64;
                let best_index = (index as i64 + index_offset) as usize;
                // The two nodes should be merged if they agree on their respective merge indices.
                if best_index as i64 + merge[best_index] as i64 != index as i64 {
                    // If not, the current node should be kept for the next iteration
                    next_nodes.push(current_nodes[index]);
                    index += 1;
                    continue;
                }

                // Since we only need to merge once, we only merge if the first index is less than the second.
                if best_index > index {
                    index += 1;
                    continue;
                }

                debug_assert_ne!(best_index, index);

                let left = current_nodes[index];
                let right = current_nodes[best_index];

                // Reserve space in the target array for the two children
                debug_assert!(insert_index >= 2);
                insert_index -= 2;

                // Create the parent node and place it in the array for the next iteration
                next_nodes.push(Bvh2Node {
                    aabb: left.aabb.union(&right.aabb),
                    index: insert_index as i32,
                });

                // Out of bounds here error here could indicate NaN present in input aabb. Try running in debug mode.
                nodes[insert_index] = left;
                nodes[insert_index + 1] = right;

                if index_offset == 1 {
                    // Since search distance is only 1, and the next index was merged with this one,
                    // we can skip the next index.
                    // The code for this with the while loop seemed to also be slightly faster than:
                    //     for (index, best_index) in merge.iter().enumerate() {
                    // even in the other cases. For some reason...
                    index += 2;
                } else {
                    index += 1;
                }
            }
        }

        (next_nodes, current_nodes) = (current_nodes, next_nodes);
        next_nodes.clear();
        depth += 1;
    }

    insert_index = insert_index.saturating_sub(1);
    nodes[insert_index] = current_nodes[0];
    Bvh2(nodes)
}

#[derive(Clone, Copy, Default)]
struct Morton64 {
    index: usize,
    code: u64,
}

impl RadixKey for Morton64 {
    const LEVELS: usize = 8;
    #[inline(always)]
    fn get_level(&self, level: usize) -> u8 {
        self.code.get_level(level)
    }
}

#[inline(always)]
pub fn sort_nodes_m64(
    current_nodes: &mut [Bvh2Node],
    sorted_nodes: &mut [Bvh2Node],
    scale: DVec3,
    offset: DVec3,
) {
    scope_print_major!("sort_nodes_m64");
    let chunk_size = ploc_scheduler().current_num_threads() as u32;
    let mut mortons = vec![Morton64::default(); current_nodes.len()];
    ploc_scheduler().par_map(
        &mut mortons,
        &|index: usize, m: &mut Morton64| {
            let center = current_nodes[index].aabb.center().as_dvec3() * scale + offset;
            *m = Morton64 {
                index,
                code: morton_encode_u64_unorm(center),
            };
        },
        chunk_size,
    );

    {
        scope_print_major!("radix sort");
        crate::radix::sorter::sort(&mut mortons)
    }

    ploc_scheduler().par_map(
        sorted_nodes,
        &|i: usize, n: &mut Bvh2Node| *n = current_nodes[mortons[i].index],
        chunk_size,
    );
}
