// https://madmann91.github.io/2021/05/05/ploc-revisited.html
// https://github.com/meistdan/ploc/
// https://meistdan.github.io/publications/ploc/paper.pdf
// https://github.com/madmann91/bvh/blob/v1/include/bvh/locally_ordered_clustering_builder.hpp

use std::cell::RefCell;

use crate::{
    bvh::{Bvh2, Bvh2Node},
    par::{par_chili, par_forte, par_rayon, par_sequential},
    scope, scope_print, scope_print_major, Args, Scheduler,
};

use obvhs::{aabb::Aabb, ploc::morton::morton_encode_u64_unorm};

use glam::*;
use rdst::{RadixKey, RadixSort};
use thread_local::ThreadLocal;

#[inline(always)] // This doesn't need to be inlined, but I thought it would funny if everything was.
pub fn build_ploc(aabbs: &[Aabb]) -> Bvh2 {
    scope_print_major!("build_ploc");

    let config: Args = argh::from_env();

    match config.backend {
        Scheduler::Forte => {
            par_forte::COMPUTE.resize_to_available();
        }
        Scheduler::Chili => {
            par_chili::init_chili();
        }
        _ => (),
    }

    let prim_count = aabbs.len();

    if prim_count == 0 {
        return Bvh2::default();
    }

    let mut total_aabb = Aabb::empty();
    let mut local_aabbs: ThreadLocal<RefCell<Aabb>> = ThreadLocal::default();

    let mut current_nodes: Vec<Bvh2Node>;

    {
        scope_print_major!("init nodes");

        current_nodes = if config.backend != Scheduler::SequentialOptimized {
            vec![Bvh2Node::default(); prim_count]
        } else {
            Vec::with_capacity(prim_count)
        };

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

        let init_nodes = |start: usize, nodes: &mut [Bvh2Node]| {
            scope!("init_nodes closure");
            for (i, node) in nodes.iter_mut().enumerate() {
                let prim_index = start + i;
                *node = init_node(
                    prim_index,
                    aabbs[prim_index],
                    &mut local_aabbs.get_or_default().borrow_mut(),
                );
            }
        };

        match config.backend {
            Scheduler::SequentialOptimized => {
                for (prim_index, aabb) in aabbs.iter().enumerate() {
                    total_aabb.extend(aabb.min).extend(aabb.max);
                    current_nodes.push(init_node(prim_index, aabbs[prim_index], &mut total_aabb));
                }
            }
            Scheduler::Sequential => par_sequential::par_chunks(&mut current_nodes, &init_nodes),
            Scheduler::Forte => par_forte::par_chunks(&mut current_nodes, &init_nodes),
            Scheduler::Chili => par_chili::par_chunks(&mut current_nodes, &init_nodes),
            Scheduler::Rayon => par_rayon::par_chunks(&mut current_nodes, &init_nodes),
        }

        if config.backend != Scheduler::SequentialOptimized {
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

    // Sort primitives according to their morton code
    sort_nodes_m64(&mut current_nodes, scale, offset);

    let mut nodes = vec![Bvh2Node::default(); nodes_count];

    let mut insert_index = nodes_count;
    let mut next_nodes = Vec::with_capacity(prim_count);

    let mut merge: Vec<i8> = vec![0; prim_count];

    #[allow(unused_variables)]
    let mut depth: usize = 0;
    while current_nodes.len() > 1 {
        let mut last_cost = f32::MAX;
        let count = current_nodes.len() - 1;
        assert!(count < merge.len()); // Try to elide bounds check
        {
            scope_print!("ploc calculate merge directions");
            let calculate_costs = |start: usize, chunk: &mut [i8]| {
                scope!("calculate_costs closure");
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

            match config.backend {
                Scheduler::SequentialOptimized => (0..count).for_each(|i| {
                    let cost = current_nodes[i]
                        .aabb
                        .union(&current_nodes[i + 1].aabb)
                        .half_area();
                    merge[i] = if last_cost < cost { -1 } else { 1 };
                    last_cost = cost;
                }),
                Scheduler::Sequential => {
                    par_sequential::par_chunks(&mut merge[..count], &calculate_costs)
                }
                Scheduler::Forte => par_forte::par_chunks(&mut merge[..count], &calculate_costs),
                Scheduler::Chili => par_chili::par_chunks(&mut merge[..count], &calculate_costs),
                Scheduler::Rayon => par_rayon::par_chunks(&mut merge[..count], &calculate_costs),
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

#[derive(Clone, Copy)]
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
pub fn sort_nodes_m64(current_nodes: &mut Vec<Bvh2Node>, scale: DVec3, offset: DVec3) {
    scope_print!("sort_nodes_m64");
    let mut mortons: Vec<Morton64> = current_nodes
        .iter()
        .enumerate()
        .map(|(index, leaf)| {
            let center = leaf.aabb.center().as_dvec3() * scale + offset;
            Morton64 {
                index,
                code: morton_encode_u64_unorm(center),
            }
        })
        .collect();
    mortons.radix_sort_unstable();
    *current_nodes = mortons.iter().map(|m| current_nodes[m.index]).collect();
}
