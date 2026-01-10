// Bitonic Sort Shader
// Sorts particle indices by cell_id for efficient neighbor searching
// Uses index buffer indirection to avoid moving full particle data

struct Particle {
    pos: vec2<f32>,
    vel: vec2<f32>,
    mass: f32,
    density: f32,
    pressure: f32,
    z_height: f32,
    layer_mask: u32,
    cell_id: u32,
    _padding: vec2<f32>,
}

struct SortParams {
    block_size: u32,    // Current block size in bitonic sort
    sub_block_size: u32, // Current sub-block (comparison distance)
    particle_count: u32,
    _padding: u32,
}

@group(0) @binding(0) var<storage, read> particles: array<Particle>;
@group(0) @binding(1) var<storage, read_write> indices: array<u32>;
@group(0) @binding(2) var<uniform> sort_params: SortParams;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    
    if idx >= sort_params.particle_count {
        return;
    }
    
    let block_size = sort_params.block_size;
    let sub_block_size = sort_params.sub_block_size;
    
    // Determine the pair to compare
    let block_start = (idx / sub_block_size) * sub_block_size;
    let half_sub = sub_block_size / 2u;
    let within_sub = idx % sub_block_size;
    
    // Only process left element of each pair
    if within_sub >= half_sub {
        return;
    }
    
    let left_idx = block_start + within_sub;
    let right_idx = left_idx + half_sub;
    
    if right_idx >= sort_params.particle_count {
        return;
    }
    
    // Determine sort direction based on block position
    let block_idx = idx / block_size;
    let ascending = (block_idx % 2u) == 0u;
    
    // Get indices and their cell_ids
    let idx_a = indices[left_idx];
    let idx_b = indices[right_idx];
    let cell_a = particles[idx_a].cell_id;
    let cell_b = particles[idx_b].cell_id;
    
    // Compare and swap if needed
    let should_swap = (ascending && cell_a > cell_b) || (!ascending && cell_a < cell_b);
    
    if should_swap {
        indices[left_idx] = idx_b;
        indices[right_idx] = idx_a;
    }
}
