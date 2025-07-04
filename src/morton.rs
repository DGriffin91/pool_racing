//---------------------------------------------------
// --- 21 bit resolution per channel morton curve ---
//---------------------------------------------------

use glam::DVec3;
use rdst::{RadixKey, RadixSort};

use crate::Bvh2Node;

#[inline(always)]
pub fn split_by_3_u64(a: u32) -> u64 {
    let mut x = a as u64 & 0x1fffff; // we only look at the first 21 bits
    x = (x | x << 32) & 0x1f00000000ffff;
    x = (x | x << 16) & 0x1f0000ff0000ff;
    x = (x | x << 8) & 0x100f00f00f00f00f;
    x = (x | x << 4) & 0x10c30c30c30c30c3;
    x = (x | x << 2) & 0x1249249249249249;
    x
}
#[inline(always)]
pub fn morton_encode_u64(x: u32, y: u32, z: u32) -> u64 {
    split_by_3_u64(x) | split_by_3_u64(y) << 1 | split_by_3_u64(z) << 2
}
#[inline(always)]
pub fn morton_encode_u64_unorm(p: DVec3) -> u64 {
    let p = p * (1 << 21) as f64;
    morton_encode_u64(p.x as u32, p.y as u32, p.z as u32)
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
#[profiling::function]
pub fn sort_nodes_m64(current_nodes: &mut Vec<Bvh2Node>, scale: DVec3, offset: DVec3) {
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
