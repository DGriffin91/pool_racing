//! An Axis-Aligned Bounding Box (AABB) represented by its minimum and maximum points.

use bytemuck::{Pod, Zeroable};
use glam::Vec3A;

use crate::ray::Ray;

/// An Axis-Aligned Bounding Box (AABB) represented by its minimum and maximum points.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct Aabb {
    pub min: Vec3A,
    pub max: Vec3A,
}

unsafe impl Pod for Aabb {}
unsafe impl Zeroable for Aabb {}

impl Aabb {
    /// Creates a new AABB with both min and max set to the given point.
    #[inline(always)]
    pub fn from_point(point: Vec3A) -> Self {
        Self {
            min: point,
            max: point,
        }
    }

    /// Extends the AABB to include the given point.
    #[inline(always)]
    pub fn extend(&mut self, point: Vec3A) -> &mut Self {
        *self = self.union(&Self::from_point(point));
        self
    }

    /// Returns the union of this AABB and another AABB.
    #[inline(always)]
    #[must_use]
    pub fn union(&self, other: &Self) -> Self {
        Aabb {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    /// Returns the diagonal vector of the AABB.
    #[inline(always)]
    pub fn diagonal(&self) -> Vec3A {
        self.max - self.min
    }

    /// Returns the center point of the AABB.
    #[inline(always)]
    pub fn center(&self) -> Vec3A {
        (self.max + self.min) * 0.5
    }

    /// Returns the center coordinate of the AABB along a specific axis.
    #[inline(always)]
    pub fn center_axis(&self, axis: usize) -> f32 {
        (self.max[axis] + self.min[axis]) * 0.5
    }

    /// Returns half the surface area of the AABB.
    #[inline(always)]
    pub fn half_area(&self) -> f32 {
        let d = self.diagonal();
        (d.x + d.y) * d.z + d.x * d.y
    }

    /// Returns the surface area of the AABB.
    #[inline(always)]
    pub fn surface_area(&self) -> f32 {
        let d = self.diagonal();
        2.0 * d.dot(d)
    }

    /// Returns an empty AABB.
    #[inline(always)]
    pub fn empty() -> Self {
        Self {
            min: Vec3A::new(f32::MAX, f32::MAX, f32::MAX),
            max: Vec3A::new(f32::MIN, f32::MIN, f32::MIN),
        }
    }

    /// Checks if this AABB intersects with another AABB.
    #[inline(always)]
    pub fn intersect_aabb(&self, other: &Aabb) -> bool {
        (self.min.cmpgt(other.max) | self.max.cmplt(other.min)).bitmask() == 0
    }

    /// Checks if this AABB intersects with a ray and returns the distance to the intersection point.
    /// Returns `f32::MAX` if there is no intersection.
    #[inline(always)]
    pub fn intersect_ray(&self, ray: &Ray) -> f32 {
        let t1 = (self.min - ray.origin) * ray.inv_direction;
        let t2 = (self.max - ray.origin) * ray.inv_direction;

        let tmin = t1.min(t2);
        let tmax = t1.max(t2);

        let tmin_n = tmin.x.max(tmin.y.max(tmin.z));
        let tmax_n = tmax.x.min(tmax.y.min(tmax.z));

        if tmax_n >= tmin_n && tmax_n >= 0.0 {
            tmin_n
        } else {
            f32::INFINITY
        }
    }
}
