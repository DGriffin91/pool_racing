// https://madmann91.github.io/2021/05/05/ploc-revisited.html
// https://github.com/meistdan/ploc/
// https://meistdan.github.io/publications/ploc/paper.pdf
// https://github.com/madmann91/bvh/blob/v1/include/bvh/locally_ordered_clustering_builder.hpp

use crate::{
    aabb::Aabb,
    bvh::{Bvh2, Bvh2Node},
    morton::sort_nodes_m64,
    par_forte, par_rayon, Scheduler, SCHEDULER,
};

use glam::*;

#[inline(always)] // This doesn't need to be inlined, but I thought it would funny if everything was.
#[profiling::function]
pub fn build_ploc(aabbs: &[Aabb]) -> Bvh2 {
    par_forte::COMPUTE.resize_to_available();

    let prim_count = aabbs.len();

    if prim_count == 0 {
        return Bvh2::default();
    }

    let mut current_nodes: Vec<Bvh2Node> = Vec::with_capacity(prim_count);
    let mut total_aabb = Aabb::empty();

    {
        profiling::scope!("init nodes");
        for (prim_index, aabb) in aabbs.iter().enumerate() {
            total_aabb.extend(aabb.min).extend(aabb.max);
            current_nodes.push(Bvh2Node {
                aabb: *aabb,
                index: -(prim_index as i32) - 1,
            });
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
            profiling::scope!("ploc calculate merge directions");
            let calculate_costs = |start: usize, chunk: &mut [i8]| {
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

            match SCHEDULER {
                Scheduler::Sequential => (0..count).for_each(|i| {
                    let cost = current_nodes[i]
                        .aabb
                        .union(&current_nodes[i + 1].aabb)
                        .half_area();
                    merge[i] = if last_cost < cost { -1 } else { 1 };
                    last_cost = cost;
                }),
                Scheduler::Forte => par_forte::par_chunks(&mut merge[..count], &calculate_costs),
                Scheduler::Rayon => par_rayon::par_chunks(&mut merge[..count], &calculate_costs),
            }

            // Have the last box to always prefer the box before it since there is none after it
            merge[current_nodes.len() - 1] = -1;
        }
        {
            profiling::scope!("ploc merge");
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
