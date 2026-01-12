// Bitonic Sort Shader with Push Constants
// Sorts particle indices by cell_id for efficient neighbor searching
// Uses push constants for per-pass parameters

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
    block_size: u32,
    sub_block_size: u32,
    particle_count: u32,
    _padding: u32,
}

@group(0) @binding(0) var<storage, read> particles: array<Particle>;
@group(0) @binding(1) var<storage, read_write> indices: array<u32>;

var<push_constant> params: SortParams;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // Each thread handles one comparison pair
    let k = global_id.x;
    let particle_count = params.particle_count;
    
    // We only need N/2 comparisons
    // Note: Use ceil(N/2) if N is odd? Bitonic usually requires N=2^k. 
    // We'll trust the checked bounds logic below.
    if k >= (particle_count + 1u) / 2u {
        return;
    }
    
    let block_size = params.block_size;
    let sub_block_size = params.sub_block_size;
    
    // Proper mapping from comparison index k to element indices
    // This inserts a gap of 'half_sub' every 'half_sub' items
    let half_sub = sub_block_size >> 1u;
    let left_idx = (k / half_sub) * sub_block_size + (k % half_sub);
    let right_idx = left_idx + half_sub;
    
    if right_idx >= particle_count {
        return;
    }
    
    // Determine sort direction based on block position
    // Bitonic sort requires alternating ascending/descending per block
    let block_idx = left_idx / block_size;
    let ascending = (block_idx & 1u) == 0u;
    
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
