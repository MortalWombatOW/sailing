// Scatter Sort Shader (Pass 3 of Counting Sort)
// Places each particle index at the correct sorted position

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

@group(0) @binding(0) var<storage, read> particles: array<Particle>;
@group(0) @binding(1) var<storage, read_write> cell_offsets: array<atomic<u32>>;
@group(0) @binding(2) var<storage, read_write> sorted_indices: array<u32>;

@compute @workgroup_size(64)
fn scatter(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let particle_count = arrayLength(&particles);
    
    if idx >= particle_count {
        return;
    }
    
    let p = particles[idx];
    
    // Atomically get and increment the offset for this cell
    // This gives us the position to write this particle's index
    let write_pos = atomicAdd(&cell_offsets[p.cell_id], 1u);
    
    // Write particle index to sorted position
    sorted_indices[write_pos] = idx;
}
