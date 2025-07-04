//! Triangle representation in 3D space.

use bytemuck::{Pod, Zeroable};
use glam::{vec2, Vec2, Vec3A};

use crate::{aabb::Aabb, ray::Ray};

#[derive(Clone, Copy, Default, Debug)]
pub struct Triangle {
    pub v0: Vec3A,
    pub v1: Vec3A,
    pub v2: Vec3A,
}

unsafe impl Pod for Triangle {}
unsafe impl Zeroable for Triangle {}

impl Triangle {
    /// Compute the normal of the triangle geometry.
    #[inline(always)]
    pub fn compute_normal(&self) -> Vec3A {
        let e1 = self.v1 - self.v0;
        let e2 = self.v2 - self.v0;
        e1.cross(e2).normalize_or_zero()
    }

    /// Compute the bounding box of the triangle.
    #[inline(always)]
    pub fn aabb(&self) -> Aabb {
        *Aabb::from_point(self.v0).extend(self.v1).extend(self.v2)
    }

    /// Find the distance (t) of the intersection of the `Ray` and this Triangle.
    /// Returns f32::INFINITY for miss.
    #[inline(always)]
    pub fn intersect(&self, ray: &Ray) -> f32 {
        // TODO not very water tight from the back side in some contexts (tris with edges at 0,0,0 show 1px gap)
        // Find out if this is typical of Möller
        // Based on Fast Minimum Storage Ray Triangle Intersection by T. Möller and B. Trumbore
        // https://madmann91.github.io/2021/04/29/an-introduction-to-bvhs.html
        let cull_backface = false;
        let e1 = self.v0 - self.v1;
        let e2 = self.v2 - self.v0;
        let n = e1.cross(e2);

        let c = self.v0 - ray.origin;
        let r = ray.direction.cross(c);
        let inv_det = 1.0 / n.dot(ray.direction);

        let u = r.dot(e2) * inv_det;
        let v = r.dot(e1) * inv_det;
        let w = 1.0 - u - v;

        //let hit = u >= 0.0 && v >= 0.0 && w >= 0.0;
        //let valid = if cull_backface {
        //    inv_det > 0.0 && hit
        //} else {
        //    inv_det != 0.0 && hit
        //};

        // Note: differs in that if v == -0.0, for example will cause valid to be false
        let hit = u.to_bits() | v.to_bits() | w.to_bits();
        let valid = if cull_backface {
            (inv_det.to_bits() | hit) & 0x8000_0000 == 0
        } else {
            inv_det != 0.0 && hit & 0x8000_0000 == 0
        };

        if valid {
            let t = n.dot(c) * inv_det;
            if t >= ray.tmin && t <= ray.tmax {
                return t;
            }
        }

        f32::INFINITY
    }

    #[inline(always)]
    pub fn compute_barycentric(&self, ray: &Ray) -> Vec2 {
        let e1 = self.v0 - self.v1;
        let e2 = self.v2 - self.v0;
        let ng = e1.cross(e2).normalize_or_zero();
        let r = ray.direction.cross(self.v0 - ray.origin);
        vec2(r.dot(e2), r.dot(e1)) / ng.dot(ray.direction)
    }
}
