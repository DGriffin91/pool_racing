use crate::{aabb::Aabb, ray::Ray, Traversal};

#[derive(Default, Clone, Copy, Debug)]
#[repr(C)]
pub struct Bvh2Node {
    pub aabb: Aabb,
    pub index: i32, // Negative for leaf (and offset down one to avoid collision at 0)
}

#[derive(Clone, Default)]
pub struct Bvh2(pub Vec<Bvh2Node>);

impl Bvh2 {
    #[inline(always)]
    pub fn new_traversal(&self, ray: Ray) -> Traversal {
        let mut stack = Vec::with_capacity(96);
        if !self.0.is_empty() {
            stack.push(0);
        }
        Traversal { stack, ray }
    }

    #[inline(always)]
    pub fn traverse<F: FnMut(&Ray, usize) -> f32>(
        &self,
        state: &mut Traversal,
        closest_t: &mut f32,
        closest_id: &mut u32,
        mut intersection_fn: F,
    ) -> bool {
        while let Some(current_node_index) = state.stack.pop() {
            let node = &self.0[current_node_index as usize];
            if node.aabb.intersect_ray(&state.ray) >= state.ray.tmax {
                continue;
            }
            if node.index < 0 {
                let primitive_id = -(node.index + 1) as u32;
                let t = intersection_fn(&state.ray, primitive_id as usize);
                if t < state.ray.tmax {
                    *closest_id = primitive_id;
                    *closest_t = t;
                    state.ray.tmax = t;
                    return true; // Yield when we hit a primitive
                }
            } else {
                state.stack.push(node.index as u32);
                state.stack.push(node.index as u32 + 1);
            }
        }
        false // Returns false when there are no more primitives to test.
    }
}
