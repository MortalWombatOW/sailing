// Count Cells Shader (Pass 1 of Counting Sort)
// Counts how many particles are in each cell using atomics

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

struct GridParams {
    cell_size: f32,
    grid_width: u32,
    grid_height: u32,
    grid_origin_x: f32,
    grid_origin_y: f32,
    _padding: vec3<f32>,
}

@group(0) @binding(0) var<storage, read> particles: array<Particle>;
@group(0) @binding(1) var<storage, read_write> cell_counts: array<atomic<u32>>;
@group(0) @binding(2) var<uniform> grid: GridParams;

// Clear counts to zero
@compute @workgroup_size(256)
fn clear_counts(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let cell_idx = global_id.x;
    let total_cells = grid.grid_width * grid.grid_height;
    
    if cell_idx >= total_cells {
        return;
    }
    
    atomicStore(&cell_counts[cell_idx], 0u);
}

// Count particles per cell
@compute @workgroup_size(64)
fn count_cells(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let particle_count = arrayLength(&particles);
    
    if idx >= particle_count {
        return;
    }
    
    let p = particles[idx];
    atomicAdd(&cell_counts[p.cell_id], 1u);
}
