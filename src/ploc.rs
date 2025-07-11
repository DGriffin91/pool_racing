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

use bytemuck::{zeroed_vec, Zeroable};
use obvhs::{aabb::Aabb, ploc::morton::morton_encode_u64_unorm};

use glam::*;
use thread_local::ThreadLocal;

static PLOC_SCHEDULER: AtomicU32 = AtomicU32::new(0);

pub fn ploc_scheduler() -> Scheduler {
    Scheduler::from(PLOC_SCHEDULER.load(Ordering::Relaxed))
}

pub fn init_ploc_scheduler() {
    scope!("init_ploc_scheduler");
    let config: Args = argh::from_env();
    config.ploc_sch.init();
    PLOC_SCHEDULER.store(config.ploc_sch as u32, Ordering::Relaxed);
}

// Holds allocations so they can be reused and are profiled separately.
pub struct PlocBuilder {
    pub current_nodes: Vec<Bvh2Node>,
    pub next_nodes: Vec<Bvh2Node>,
    pub sorted_nodes: Vec<Bvh2Node>,
    pub merge: Vec<i8>,
    pub mortons: Vec<Morton64>,
    pub local_aabbs: ThreadLocal<RefCell<Aabb>>,
}

impl PlocBuilder {
    pub fn preallocate_builder(leaf_count: usize) -> PlocBuilder {
        scope_print_major!("preallocate_builder");
        PlocBuilder {
            current_nodes: zeroed_vec(leaf_count),
            next_nodes: zeroed_vec(leaf_count),
            sorted_nodes: zeroed_vec(leaf_count),
            merge: zeroed_vec(leaf_count),
            mortons: zeroed_vec(leaf_count),
            local_aabbs: ThreadLocal::default(),
        }
    }

    #[inline(always)]
    pub fn build_ploc(&mut self, aabbs: &[Aabb]) -> Bvh2 {
        let mut bvh = Bvh2::default();
        self.rebuild_ploc(aabbs, &mut bvh);
        bvh
    }

    #[inline(always)]
    pub fn rebuild_ploc(&mut self, aabbs: &[Aabb], bvh: &mut Bvh2) {
        scope_print_major!("build_ploc");
        init_ploc_scheduler();

        // How many workers per available_parallelism thread.
        // If tasks take an non-uniform amount of time more workers per thread can improve cpu utilization.
        let default_chunk_count = ploc_scheduler().current_num_threads();

        let prim_count = aabbs.len();

        if prim_count == 0 {
            bvh.clear();
        }

        let mut total_aabb = Aabb::empty();

        for local_aabb in self.local_aabbs.iter_mut() {
            *local_aabb = Default::default();
        }

        {
            scope_print_major!("init nodes");

            {
                scope!("resize current_nodes");
                self.current_nodes.resize(prim_count, Default::default());
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

            let chunk_size = self.current_nodes.len() / default_chunk_count;

            match ploc_scheduler() {
                Scheduler::SequentialOptimized => {
                    for (prim_index, aabb) in aabbs.iter().enumerate() {
                        total_aabb.extend(aabb.min).extend(aabb.max);
                        self.current_nodes[prim_index] =
                            init_node(prim_index, aabbs[prim_index], &mut total_aabb);
                    }
                }
                _ => ploc_scheduler().par_chunks_mut(
                    &mut self.current_nodes,
                    &|chunk_id: usize, nodes: &mut [Bvh2Node]| {
                        scope!("init_nodes closure");
                        let start = chunk_id * chunk_size;
                        for (i, node) in nodes.iter_mut().enumerate() {
                            let prim_index = start + i;
                            *node = init_node(
                                prim_index,
                                aabbs[prim_index],
                                &mut self.local_aabbs.get_or_default().borrow_mut(),
                            );
                        }
                    },
                    chunk_size,
                ),
            }

            if ploc_scheduler() != Scheduler::SequentialOptimized {
                for local_aabb in self.local_aabbs.iter_mut() {
                    total_aabb.extend(local_aabb.get_mut().min);
                    total_aabb.extend(local_aabb.get_mut().max);
                }
            }
        }

        // Merge nodes until there is only one left
        let nodes_count = (2 * prim_count as i64 - 1).max(0) as usize;

        let scale = 1.0 / total_aabb.diagonal().as_dvec3();
        let offset = -total_aabb.min.as_dvec3() * scale;

        {
            scope!("resize sorted_nodes");
            self.sorted_nodes
                .resize(self.current_nodes.len(), Default::default());
        };

        {
            scope!("resize mortons");
            self.mortons
                .resize(self.current_nodes.len(), Default::default());
        }

        // Sort primitives according to their morton code
        sort_nodes_m64(
            &mut self.current_nodes,
            &mut self.sorted_nodes,
            &mut self.mortons,
            scale,
            offset,
        );

        mem::swap(&mut self.current_nodes, &mut self.sorted_nodes);

        {
            scope!("resize nodes");
            bvh.nodes.resize(nodes_count, Bvh2Node::default());
        };

        let mut insert_index = nodes_count;

        {
            scope!("resize merge");
            self.merge.resize(prim_count, 0);
        };
        self.next_nodes.clear();

        #[allow(unused_variables)]
        let mut depth: usize = 0;
        while self.current_nodes.len() > 1 {
            scope!("merge pass");
            let mut last_cost = f32::MAX;
            let count = self.current_nodes.len() - 1;
            assert!(count < self.merge.len()); // Try to elide bounds check
            {
                scope_print!("ploc calculate merge directions");

                let chunk_size = self.merge[..count].len() / default_chunk_count;

                let calculate_costs = |chunk_id: usize, chunk: &mut [i8]| {
                    scope!("calculate_costs closure");
                    let start = chunk_id * chunk_size;
                    let mut last_cost = if start == 0 {
                        f32::MAX
                    } else {
                        self.current_nodes[start - 1]
                            .aabb
                            .union(&self.current_nodes[start].aabb)
                            .half_area()
                    };
                    for (local_n, merge_n) in chunk.iter_mut().enumerate() {
                        let i = local_n + start;
                        let cost = self.current_nodes[i]
                            .aabb
                            .union(&self.current_nodes[i + 1].aabb)
                            .half_area();
                        *merge_n = if last_cost < cost { -1 } else { 1 };
                        last_cost = cost;
                    }
                };

                match ploc_scheduler() {
                    Scheduler::SequentialOptimized => (0..count).for_each(|i| {
                        let cost = self.current_nodes[i]
                            .aabb
                            .union(&self.current_nodes[i + 1].aabb)
                            .half_area();
                        self.merge[i] = if last_cost < cost { -1 } else { 1 };
                        last_cost = cost;
                    }),
                    _ => ploc_scheduler().par_chunks_mut(
                        &mut self.merge[..count],
                        &calculate_costs,
                        chunk_size,
                    ),
                }

                // Have the last box to always prefer the box before it since there is none after it
                self.merge[self.current_nodes.len() - 1] = -1;
            }

            self.merge.resize(self.current_nodes.len(), 0);

            let mut index = 0;
            while index < self.current_nodes.len() {
                let index_offset = self.merge[index] as i64;
                let best_index = (index as i64 + index_offset) as usize;
                // The two nodes should be merged if they agree on their respective merge indices.
                if best_index as i64 + self.merge[best_index] as i64 != index as i64 {
                    // If not, the current node should be kept for the next iteration
                    self.next_nodes.push(self.current_nodes[index]);
                    index += 1;
                    continue;
                }

                // Since we only need to merge once, we only merge if the first index is less than the second.
                if best_index > index {
                    index += 1;
                    continue;
                }

                debug_assert_ne!(best_index, index);

                let left = self.current_nodes[index];
                let right = self.current_nodes[best_index];

                // Reserve space in the target array for the two children
                debug_assert!(insert_index >= 2);
                insert_index -= 2;

                // Create the parent node and place it in the array for the next iteration
                self.next_nodes.push(Bvh2Node {
                    aabb: left.aabb.union(&right.aabb),
                    index: insert_index as i32,
                });

                // Out of bounds here error here could indicate NaN present in input aabb. Try running in debug mode.
                bvh.nodes[insert_index] = left;
                bvh.nodes[insert_index + 1] = right;

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

            mem::swap(&mut self.current_nodes, &mut self.next_nodes);
            self.next_nodes.clear();
            depth += 1;
        }

        insert_index = insert_index.saturating_sub(1);
        bvh.nodes[insert_index] = self.current_nodes[0];
    }
}

#[derive(Clone, Copy, Default, Zeroable)]
pub struct Morton64 {
    pub index: usize,
    pub code: u64,
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
    mortons: &mut [Morton64],
    scale: DVec3,
    offset: DVec3,
) {
    scope_print_major!("sort_nodes_m64");
    let chunk_size = ploc_scheduler().current_num_threads() as u32;
    {
        scope!("par generate Morton64s");
        ploc_scheduler().par_map(
            mortons,
            &|index: usize, m: &mut Morton64| {
                //scope!("generate Morton64s");
                let center = current_nodes[index].aabb.center().as_dvec3() * scale + offset;
                *m = Morton64 {
                    index,
                    code: morton_encode_u64_unorm(center),
                };
            },
            chunk_size,
        );
    }

    {
        scope_print!("radix sort");
        crate::radix::sorter::sort(mortons)
    }

    {
        scope!("par copy back sorted");
        ploc_scheduler().par_map(
            sorted_nodes,
            &|i: usize, n: &mut Bvh2Node| {
                //scope!("copy back sorted");
                *n = current_nodes[mortons[i].index]
            },
            chunk_size,
        );
    }
}
