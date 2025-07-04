use glam::*;
use pico_ploc::{
    ploc::build_ploc,
    ray::Ray,
    test_util::geometry::{icosphere, PLANE},
    triangle::Triangle,
};

fn main() {
    // Build a scene with an icosphere and a plane
    // BVH primitives do not need to be triangles, the BVH builder is only concerned with AABBs.
    let mut tris: Vec<Triangle> = Vec::new();
    tris.extend(icosphere(1));
    tris.extend(PLANE);

    let aabbs = tris.iter().map(|t| t.aabb()).collect::<Vec<_>>();
    let bvh = build_ploc(&aabbs);

    // Create a new ray
    let ray = Ray::new_inf(vec3a(0.1, 0.1, 4.0), vec3a(0.0, 0.0, -1.0));

    // Traverse the BVH, finding the closest hit.
    let mut t = f32::MAX;
    let mut hit_id = u32::MAX;
    let mut state = bvh.new_traversal(ray);
    while bvh.traverse(&mut state, &mut t, &mut hit_id, |ray, id| {
        tris[id as usize].intersect(ray)
    }) {}
    if t < f32::MAX {
        println!("Hit Triangle {}", hit_id);
        println!("Distance to hit: {}", t);
    } else {
        println!("Miss");
    }
}
