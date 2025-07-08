use bytemuck::Zeroable;
use obvhs::{aabb::Aabb, cwbvh::TraversalStack32, ray::Ray};

#[derive(Default, Clone, Copy, Debug, Zeroable)]
#[repr(C)]
pub struct Bvh2Node {
    pub aabb: Aabb,
    pub index: i32, // Negative for leaf (and offset down one to avoid collision at 0)
}

#[derive(Clone, Default)]
pub struct Bvh2 {
    pub nodes: Vec<Bvh2Node>,
}

impl Bvh2 {
    #[inline(always)]
    pub fn traverse<F: FnMut(&Ray, usize) -> f32>(
        &self,
        ray: &mut Ray,
        closest_id: &mut u32,
        mut intersection_fn: F,
    ) {
        crate::scope!("traverse");
        // TODO allow for a deeper stack
        let mut stack = TraversalStack32::default();
        stack.clear();
        stack.push(0);
        while let Some(current_node_index) = stack.pop() {
            let node = &self.nodes[*current_node_index as usize];
            if node.aabb.intersect_ray(ray) >= ray.tmax {
                continue;
            }
            if node.index < 0 {
                let primitive_id = -(node.index + 1) as u32;
                let t = intersection_fn(ray, primitive_id as usize);
                if t < ray.tmax {
                    *closest_id = primitive_id;
                    ray.tmax = t;
                    continue;
                }
            } else {
                stack.push(node.index as u32);
                stack.push(node.index as u32 + 1);
            }
        }
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.nodes.clear();
    }
}
