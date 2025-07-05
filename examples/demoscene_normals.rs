use core::f32;

use glam::*;
use image::{ImageBuffer, Rgba};

#[path = "./helpers/debug.rs"]
mod debug;
use debug::simple_debug_window;
use obvhs::{ray::Ray, test_util::geometry::demoscene};
use pico_ploc::{ploc::build_ploc, Args};

fn main() {
    let config: Args = argh::from_env();
    config.backend.init();

    let tris = demoscene(1280, 570);
    let aabbs = tris.iter().map(|t| t.aabb()).collect::<Vec<_>>();
    // Build cwbvh (Change this to build_bvh2_from_tris to try with Bvh2)
    let bvh = build_ploc(&aabbs);

    // Setup render target and camera
    let width = 1280;
    let height = 720;
    let target_size = Vec2::new(width as f32, height as f32);
    let fov = 17.0f32;
    let eye = vec3a(0.0, 0.0, 1.35);
    let look_at = eye + vec3a(0.0, 0.16, -1.0);

    // Compute camera projection & view matrices
    let aspect_ratio = target_size.x / target_size.y;
    let proj_inv =
        Mat4::perspective_infinite_reverse_rh(fov.to_radians(), aspect_ratio, 0.01).inverse();
    let view_inv = Mat4::look_at_rh(eye.into(), look_at.into(), Vec3::Y).inverse();

    let window = simple_debug_window(width, height);

    let mut fragments = vec![Vec3A::ZERO; width * height];

    {
        pico_ploc::scope_print_major!("trace rays");
        // For each pixel trace ray into scene and write normal as color
        let trace_fn = |i: usize, fragment: &mut Vec3A| {
            pico_ploc::scope!("trace ray");
            let frag_coord = uvec2((i % width) as u32, (i / width) as u32);
            let mut screen_uv = frag_coord.as_vec2() / target_size;
            screen_uv.y = 1.0 - screen_uv.y;
            let ndc = screen_uv * 2.0 - Vec2::ONE;
            let clip_pos = vec4(ndc.x, ndc.y, 1.0, 1.0);

            let mut vs_pos = proj_inv * clip_pos;
            vs_pos /= vs_pos.w;
            let direction = (Vec3A::from((view_inv * vs_pos).xyz()) - eye).normalize();
            let mut ray = Ray::new(eye, direction, 0.0, f32::MAX);

            let mut hit_id = u32::MAX;
            bvh.traverse(&mut ray, &mut hit_id, |ray, id| tris[id].intersect(ray));
            if ray.tmax < f32::MAX {
                let mut normal: Vec3A = tris[hit_id as usize].compute_normal();
                normal *= normal.dot(-ray.direction).signum(); // Double sided
                *fragment = normal;
            }

            let accum_color = window.buffer.get(i as usize) + fragment.extend(1.0);
            window.buffer.set(i as usize, accum_color);
        };

        config.backend.par_map(&mut fragments, &trace_fn);
    }

    // Init image buffer
    let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(width as u32, height as u32);
    let pixels = img.as_mut();

    pixels.chunks_mut(4).enumerate().for_each(|(i, chunk)| {
        let c = (fragments[i].clamp(Vec3A::ZERO, Vec3A::ONE) * 255.0).as_uvec3();
        chunk.copy_from_slice(&[c.x as u8, c.y as u8, c.z as u8, 255]);
    });

    img.save("basic_cornell_box_rend.png")
        .expect("Failed to save image");

    window.thread.join().unwrap(); // Wait for window to close.
}
