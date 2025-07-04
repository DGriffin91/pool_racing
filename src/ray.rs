//! A ray in 3D space.

use glam::{vec3a, Vec3A};

/// Computes the inverse of `x` avoiding division by zero.
#[inline(always)]
pub fn safe_inverse(x: f32) -> f32 {
    if x.abs() <= f32::EPSILON {
        x.signum() / f32::EPSILON
    } else {
        1.0 / x
    }
}

/// A struct representing a ray in 3D space.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Ray {
    /// The starting point of the ray.
    pub origin: Vec3A,
    /// The direction vector of the ray.
    pub direction: Vec3A,
    /// The inverse of the direction vector components.
    /// Used to avoid division in ray/aabb tests. Seems to improve performance in
    /// some cases on the cpu, but not the gpu in some others.
    pub inv_direction: Vec3A,
    /// The minimum `t` (distance) value for intersection tests.
    pub tmin: f32,
    /// The maximum `t` (distance) value for intersection tests.
    pub tmax: f32,
}

impl Ray {
    /// Creates a new `Ray` with the given origin, direction, and `t` (distance) range.
    #[inline(always)]
    pub fn new(origin: Vec3A, direction: Vec3A, min: f32, max: f32) -> Self {
        let ray = Ray {
            origin,
            direction,
            inv_direction: vec3a(
                safe_inverse(direction.x),
                safe_inverse(direction.y),
                safe_inverse(direction.z),
            ),
            tmin: min,
            tmax: max,
        };

        debug_assert!(ray.inv_direction.is_finite());
        debug_assert!(ray.direction.is_finite());
        debug_assert!(origin.is_finite());

        ray
    }

    /// Creates a new infinite `Ray` with the given origin, direction.
    #[inline(always)]
    pub fn new_inf(origin: Vec3A, direction: Vec3A) -> Self {
        Self::new(origin, direction, 0.0, f32::INFINITY)
    }
}
